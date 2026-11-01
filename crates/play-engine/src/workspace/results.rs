use super::{
    model::{AssetOrigin, Context},
    mutations, order, storage,
};
use crate::{Result, protocol};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use sqlx::{Row, SqliteConnection};
use uuid::Uuid;

struct Output {
    kind: &'static str,
    data: Vec<u8>,
    semantic_type: String,
    title: String,
}

async fn output(
    connection: &mut SqliteConnection,
    context: &Context,
    run: Uuid,
    attempt: Uuid,
    port: &str,
) -> Result<Output> {
    let row=sqlx::query("SELECT r.rowid AS sequence,r.status,r.retain_output,r.plan,r.result,n.node_id,n.status AS attempt_status FROM pipeline_runs r JOIN node_attempts n ON (r.workspace_id,r.exploration_id,r.id)=(n.workspace_id,n.exploration_id,n.run_id) WHERE r.workspace_id=? AND r.exploration_id=? AND r.stage_id=? AND r.id=? AND n.id=?")
        .bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(context.stage_id.to_string()).bind(run.to_string()).bind(attempt.to_string()).fetch_optional(connection).await.map_err(storage::failure)?.ok_or("not_found")?;
    if row.get::<&str, _>("status") != "completed"
        || row.get::<&str, _>("attempt_status") != "completed"
        || !row.get::<bool, _>("retain_output")
    {
        return Err("result_not_available");
    }
    let plan: Value = serde_json::from_str(row.get("plan")).map_err(|_| "corrupt_store")?;
    let result: Value = serde_json::from_str(
        row.get::<Option<&str>, _>("result")
            .ok_or("corrupt_store")?,
    )
    .map_err(|_| "corrupt_store")?;
    resolve(
        &plan,
        &result,
        row.get("node_id"),
        row.get("sequence"),
        port,
    )
}

fn contract(plan: &Value) -> Result<cube_core::operations::contracts::Contract> {
    let operation = plan["operation"]["operation"]
        .as_str()
        .ok_or("corrupt_store")?;
    let contract = cube_core::operations::require(operation)
        .map_err(|_| "contract_mismatch")?
        .contract()
        .map_err(|_| "contract_mismatch")?;
    if plan["contract"] != super::contract_snapshots::reference(&contract)?
        || plan["contractDigest"]
            != cube_core::hashing::sha256(
                &serde_json::to_vec(&contract).map_err(|_| "invalid_state")?,
            )
        || plan["operation"]["operationVersion"] != contract.operation_version
    {
        return Err("contract_mismatch");
    }
    super::run_plan::validate_retention(
        plan,
        &json!({"id":plan["targetNodes"][0], "operation":plan["operation"]}),
        &contract,
    )?;
    Ok(contract)
}

fn resolve(plan: &Value, result: &Value, node: &str, sequence: i64, port: &str) -> Result<Output> {
    if plan["targetNodes"] != json!([node])
        || plan["adapterVersion"] != 3
        || plan["formatVersion"] != 4
    {
        return Err("unsupported_pipeline");
    }
    let contract = contract(plan)?;
    if result["status"] != "completed" || result.get("replayGap").is_some() {
        return Err("result_not_available");
    }
    contract
        .validate_outputs(&result["outputs"])
        .map_err(|_| "corrupt_store")?;
    let definition = contract
        .outputs
        .iter()
        .find(|p| p.name == port)
        .ok_or("not_found")?;
    let value = &result["outputs"][port];
    let executable =
        cube_core::operations::execution::describe(&contract.operation, Default::default())
            .map_err(|_| "contract_mismatch")?;
    let representation = *executable.outputs.get(port).ok_or("contract_mismatch")?;
    let (kind, data) = representation_data(representation, &definition.semantic_type, value)?;
    let name = plan["operation"]["name"].as_str().ok_or("corrupt_store")?;
    Ok(Output {
        kind,
        data,
        semantic_type: definition.semantic_type.clone(),
        title: format!("{name} · {port} · Run {sequence}"),
    })
}

fn representation_data(
    representation: cube_core::operations::execution::Representation,
    semantic_type: &str,
    value: &Value,
) -> Result<(&'static str, Vec<u8>)> {
    use cube_core::operations::execution::Representation;
    match representation {
        Representation::Text => {
            let text = value.as_str().ok_or("corrupt_store")?;
            if text.len() > protocol::contract().text_bytes {
                return Err("output_limit");
            }
            Ok(("text", text.as_bytes().to_vec()))
        }
        Representation::Bytes => {
            let encoded = value["base64"].as_str().ok_or("corrupt_store")?;
            if encoded.len() > protocol::contract().file_bytes.div_ceil(3) * 4 {
                return Err("output_limit");
            }
            let data = STANDARD.decode(encoded).map_err(|_| "corrupt_store")?;
            if data.len() > protocol::contract().file_bytes {
                return Err("output_limit");
            }
            Ok(("file", data))
        }
        Representation::ByteArray => {
            let bytes: Vec<u8> =
                serde_json::from_value(value.clone()).map_err(|_| "corrupt_store")?;
            if bytes.len() > protocol::contract().file_bytes {
                return Err("output_limit");
            }
            Ok(("file", bytes))
        }
        Representation::Json => Ok((
            "structured",
            super::structured::encode(semantic_type, value)?,
        )),
        Representation::ImageSamples => Ok(("image", super::image_views::encode(value)?)),
    }
}

pub(super) async fn pin(
    connection: &mut SqliteConnection,
    context: &Context,
    expected: i64,
    run: Uuid,
    attempt: Uuid,
    port: &str,
) -> Result<Value> {
    if expected < 1
        || expected == i64::MAX
        || port.is_empty()
        || port.len() > 64
        || !port
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
    {
        return Err("invalid_command");
    }
    if order::revision(connection, context).await? != expected {
        return Err("state_conflict");
    }
    let output = output(connection, context, run, attempt, port).await?;
    let existing:Option<String>=sqlx::query_scalar("SELECT b.id FROM result_origins o JOIN blocks b ON (o.workspace_id,o.exploration_id,o.asset_id)=(b.workspace_id,b.exploration_id,b.asset_id) WHERE o.workspace_id=? AND o.exploration_id=? AND b.stage_id=? AND o.run_id=? AND o.attempt_id=? AND o.port=?")
        .bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(context.stage_id.to_string()).bind(run.to_string()).bind(attempt.to_string()).bind(port).fetch_optional(&mut *connection).await.map_err(storage::failure)?;
    if let Some(block) = existing {
        return Ok(json!({"blockId":block,"stageId":context.stage_id,"revision":expected}));
    }
    let block = mutations::add_block(
        connection,
        context,
        &output.title,
        output.kind,
        &output.data,
        None,
    )
    .await?;
    sqlx::query("INSERT INTO result_origins VALUES(?,?,?,?,?,?,?)")
        .bind(context.workspace_id.to_string())
        .bind(context.exploration_id.to_string())
        .bind(&block.asset_id)
        .bind(run.to_string())
        .bind(attempt.to_string())
        .bind(port)
        .bind(output.semantic_type)
        .execute(&mut *connection)
        .await
        .map_err(storage::failure)?;
    Ok(
        json!({"blockId":block.id,"stageId":context.stage_id,"revision":order::revision(connection,context).await?}),
    )
}

pub(super) async fn origin(
    connection: &mut SqliteConnection,
    workspace: &str,
    exploration: &str,
    asset: &str,
) -> Result<Option<Box<AssetOrigin>>> {
    let row=sqlx::query("SELECT o.run_id,o.attempt_id,o.port,o.semantic_type,r.rowid AS sequence,r.trail_id,r.plan,n.node_id FROM result_origins o JOIN pipeline_runs r ON (o.workspace_id,o.exploration_id,o.run_id)=(r.workspace_id,r.exploration_id,r.id) JOIN node_attempts n ON (o.workspace_id,o.exploration_id,o.run_id,o.attempt_id)=(n.workspace_id,n.exploration_id,n.run_id,n.id) WHERE o.workspace_id=? AND o.exploration_id=? AND o.asset_id=?")
        .bind(workspace).bind(exploration).bind(asset).fetch_optional(connection).await.map_err(storage::failure)?;
    row.map(|row| {
        let plan: Value = serde_json::from_str(row.get("plan")).map_err(|_| "corrupt_store")?;
        Ok(Box::new(AssetOrigin::RunOutput {
            run_id: row.get("run_id"),
            attempt_id: row.get("attempt_id"),
            node_id: row.get("node_id"),
            trail_id: row.get("trail_id"),
            port: row.get("port"),
            semantic_type: row.get("semantic_type"),
            operation_name: plan["operation"]["name"]
                .as_str()
                .ok_or("corrupt_store")?
                .into(),
            sequence: row.get("sequence"),
        }))
    })
    .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_current_adapter_results_can_be_promoted_to_assets() {
        let contract = cube_core::operations::require("cipher.rot13")
            .unwrap()
            .contract()
            .unwrap();
        let mut plan = json!({"formatVersion":4,"adapterVersion":3,"targetNodes":["node"],"operation":{"operation":"cipher.rot13","operationVersion":contract.operation_version,"name":"ROT13"},"contract":super::super::contract_snapshots::reference(&contract).unwrap(),"contractDigest":cube_core::hashing::sha256(&serde_json::to_vec(&contract).unwrap())});
        let result = json!({"status":"completed","outputs":{"output":"Hello"}});
        assert_eq!(
            resolve(&plan, &result, "node", 1, "output").unwrap().data,
            b"Hello"
        );
        for version in [0, 1, 2, 4] {
            plan["adapterVersion"] = json!(version);
            assert!(
                matches!(
                    resolve(&plan, &result, "node", 1, "output"),
                    Err("unsupported_pipeline")
                ),
                "adapter {version}"
            );
        }
    }
    #[test]
    fn private_output_promotion_uses_consent_from_the_exact_frozen_node() {
        let fixtures: Vec<Value> = serde_json::from_str(include_str!(
            "../../../../../cube/crates/cube-core/src/operations/contracts/conformance.json"
        ))
        .unwrap();
        let fixture = fixtures
            .iter()
            .find(|f| f["operation"] == "cipher.vic.encode")
            .unwrap();
        let contract = cube_core::operations::require("cipher.vic.encode")
            .unwrap()
            .contract()
            .unwrap();
        let id = Uuid::new_v4();
        let definition = json!({"formatVersion":4,"inputs":[{"name":"source","semanticType":"text"}],"nodes":[{"id":id,"operation":"cipher.vic.encode","operationVersion":1,"parameters":fixture["parameters"],"inputs":{"text":{"kind":"literal","value":fixture["inputs"]["text"]}},"retainPrivate":true,"position":{"x":0,"y":0}}],"output":id});
        let mut plan = json!({"formatVersion":4,"adapterVersion":3,"targetNodes":[id],"pipeline":{"definition":definition},"operation":{"operation":"cipher.vic.encode","operationVersion":1,"name":"VIC"},"contract":super::super::contract_snapshots::reference(&contract).unwrap(),"contractDigest":cube_core::hashing::sha256(&serde_json::to_vec(&contract).unwrap())});
        let result = json!({"status":"completed","outputs":fixture["outputs"]});
        assert_eq!(
            resolve(&plan, &result, &id.to_string(), 1, "text")
                .unwrap()
                .data,
            fixture["outputs"]["text"].as_str().unwrap().as_bytes()
        );
        let node =
            json!({"id":id,"operation":plan["operation"],"contractDigest":plan["contractDigest"]});
        super::super::candidates::output_contract(&plan, &node).unwrap();
        for replacement in [Value::Null, json!(false)] {
            plan["pipeline"]["definition"]["nodes"][0]["retainPrivate"] = replacement;
            assert!(matches!(
                resolve(&plan, &result, &id.to_string(), 1, "text"),
                Err("retention_policy_unavailable")
            ));
            assert!(matches!(
                super::super::candidates::output_contract(&plan, &node),
                Err("retention_policy_unavailable")
            ));
        }
        plan["pipeline"]["definition"]["nodes"][0]["retainPrivate"] = json!(true);
        plan["pipeline"]["definition"]["nodes"][0]["id"] = json!(Uuid::new_v4());
        assert!(matches!(
            resolve(&plan, &result, &id.to_string(), 1, "text"),
            Err("retention_policy_unavailable")
        ));
    }
}
