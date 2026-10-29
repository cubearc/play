use super::{
    blocks,
    model::{Content, RunRequest},
    queries, storage,
};
use crate::{Result, protocol::Invocation};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use sqlx::{Row, SqliteConnection};
use uuid::Uuid;

pub struct Plan {
    pub value: Value,
    pub invocation: Invocation,
    pub pipeline_id: String,
    pub pipeline_revision: i64,
    pub node_id: Uuid,
    pub asset_id: String,
    pub asset_revision: i64,
    pub input_port: String,
}

pub async fn build(connection: &mut SqliteConnection, request: &RunRequest) -> Result<Plan> {
    let context = &request.context;
    let workspace = context.workspace_id.to_string();
    let exploration = context.exploration_id.to_string();
    let row=sqlx::query("SELECT source_id,source_revision,pipeline_id,pipeline_revision FROM trails WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND id=?")
  .bind(&workspace).bind(&exploration).bind(context.stage_id.to_string()).bind(request.trail_id.to_string()).fetch_optional(&mut *connection).await.map_err(storage::failure)?.ok_or("not_found")?;
    if row.get::<Option<String>, _>("pipeline_id").as_deref()
        != Some(request.pipeline_id.to_string().as_str())
        || row.get::<Option<i64>, _>("pipeline_revision") != Some(request.pipeline_revision)
    {
        return Err("state_conflict");
    }
    let definition:String=sqlx::query_scalar("SELECT definition FROM pipeline_revisions WHERE workspace_id=? AND pipeline_id=? AND revision=?")
  .bind(&workspace).bind(request.pipeline_id.to_string()).bind(request.pipeline_revision).fetch_optional(&mut *connection).await.map_err(storage::failure)?.ok_or("not_found")?;
    let mut definition: Value = serde_json::from_str(&definition).map_err(|_| "corrupt_store")?;
    if definition["formatVersion"] != 1
        || definition["nodes"]
            .as_array()
            .is_none_or(|nodes| nodes.len() != 1)
    {
        return Err("unsupported_pipeline");
    }
    let asset_id: String = row.get("source_id");
    let asset_revision: i64 = row.get("source_revision");
    if asset_id != request.source_asset_id.to_string() || asset_revision != request.source_revision
    {
        return Err("state_conflict");
    }
    blocks::source_active(connection, context, &asset_id).await?;
    let asset = queries::asset(
        connection,
        &workspace,
        &exploration,
        &asset_id,
        asset_revision,
    )
    .await?;
    let (text, semantic_type) = match asset.content {
        Content::Text { text } if request.operation != "image.pixels.read" => (text, "text"),
        Content::Image if request.operation == "image.pixels.read" => {
            let bytes:Vec<u8>=sqlx::query_scalar("SELECT data FROM asset_revisions WHERE workspace_id=? AND exploration_id=? AND asset_id=? AND revision=?").bind(&workspace).bind(&exploration).bind(&asset_id).bind(asset_revision).fetch_one(&mut *connection).await.map_err(storage::failure)?;
            (STANDARD.encode(bytes), "image.samples")
        }
        _ => return Err("invalid_asset_type"),
    };
    let invocation = Invocation {
        operation: request.operation.clone(),
        operation_version: request.operation_version,
        input: text,
    };
    crate::worker::validate(&invocation)?;
    let descriptor = cube_core::operations::require(&invocation.operation)
        .map_err(|_| "unsupported_operation")?;
    let contract = descriptor.contract().map_err(|_| "contract_mismatch")?;
    if contract.policy.private_input
        || contract.policy.private_output
        || !contract.policy.sensitive_parameters.is_empty()
    {
        return Err("retention_policy_unavailable");
    }
    contract
        .validate_inputs(&crate::worker::named_inputs(&invocation)?)
        .map_err(|_| "contract_mismatch")?;
    let parameters = crate::worker::parameters(&invocation.operation)?;
    contract
        .validate_parameters(&parameters)
        .map_err(|_| "contract_mismatch")?;
    let name = descriptor
        .metadata()
        .map_or(descriptor.command, |metadata| metadata.canonical_name);
    let pipeline_id = if definition["nodes"][0]["operation"] == invocation.operation
        && definition["nodes"][0]["operation_version"] == invocation.operation_version
    {
        request.pipeline_id.to_string()
    } else {
        // A selected operation is frozen as a separate reusable definition. Other Trails keep their selected revision.
        let existing:Option<(String,String)>=sqlx::query_as("SELECT pipeline_id,definition FROM pipeline_revisions WHERE workspace_id=? AND revision=1 AND json_extract(definition,'$.nodes[0].operation')=? AND json_extract(definition,'$.nodes[0].operation_version')=? ORDER BY rowid LIMIT 1")
   .bind(&workspace).bind(&invocation.operation).bind(invocation.operation_version).fetch_optional(&mut *connection).await.map_err(storage::failure)?;
        if let Some((id, stored)) = existing {
            definition = serde_json::from_str(&stored).map_err(|_| "corrupt_store")?;
            id
        } else {
            let id = Uuid::new_v4().to_string();
            definition["inputs"][0]["semanticType"] = json!(semantic_type);
            definition["nodes"][0]["id"] = json!(Uuid::new_v4());
            definition["nodes"][0]["operation"] = json!(invocation.operation);
            definition["nodes"][0]["operation_version"] = json!(invocation.operation_version);
            sqlx::query("INSERT INTO pipelines VALUES(?,?,?,1)")
                .bind(&workspace)
                .bind(&id)
                .bind(name)
                .execute(&mut *connection)
                .await
                .map_err(storage::failure)?;
            sqlx::query("INSERT INTO pipeline_revisions VALUES(?,?,1,?)")
                .bind(&workspace)
                .bind(&id)
                .bind(definition.to_string())
                .execute(&mut *connection)
                .await
                .map_err(storage::failure)?;
            id
        }
    };
    let pipeline_revision = if pipeline_id == request.pipeline_id.to_string() {
        request.pipeline_revision
    } else {
        1
    };
    let node_id = Uuid::parse_str(
        definition["nodes"][0]["id"]
            .as_str()
            .ok_or("corrupt_store")?,
    )
    .map_err(|_| "corrupt_store")?;
    let input_port = contract
        .inputs
        .iter()
        .find(|port| port.semantic_type == semantic_type)
        .ok_or("contract_mismatch")?
        .name
        .clone();
    let mut inputs = json!({input_port.clone():{"assetId":asset_id,"revision":asset_revision,"semanticType":semantic_type}});
    if invocation.operation == "esolang.brainfuck.execute" {
        inputs["input"] = json!({"literal":[]});
    }
    let value = json!({"formatVersion":1,"pipeline":{"id":pipeline_id,"revision":pipeline_revision,"definition":definition},"targetNodes":[node_id],"reusePolicy":"none","reusedOutputs":[],"caller":"desktop","adapterVersion":1,"contract":contract,"contractDigest":cube_core::hashing::sha256(serde_json::to_string(&contract).map_err(|_| "invalid_state")?.as_bytes()),"operation":{"operation":invocation.operation,"operationVersion":invocation.operation_version,"name":name},"inputs":inputs,"parameters":parameters,"limits":{"runMs":crate::protocol::contract().run_ms,"cpuSeconds":crate::protocol::contract().cpu_seconds,"vmGrowthBytes":crate::protocol::contract().vm_growth_bytes,"outputBytes":crate::protocol::contract().output_bytes}});
    if value.to_string().len() > 16384 {
        return Err("output_limit");
    }
    Ok(Plan {
        value,
        invocation,
        pipeline_id,
        pipeline_revision,
        node_id,
        asset_id,
        asset_revision,
        input_port,
    })
}
