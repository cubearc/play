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
    let row=sqlx::query("SELECT source_id,source_revision,pipeline_id,pipeline_revision,revision,hypothesis FROM trails WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND id=?")
  .bind(&workspace).bind(&exploration).bind(context.stage_id.to_string()).bind(request.trail_id.to_string()).fetch_optional(&mut *connection).await.map_err(storage::failure)?.ok_or("not_found")?;
    if row.get::<Option<String>, _>("pipeline_id").as_deref()
        != Some(request.pipeline_id.to_string().as_str())
        || row.get::<Option<i64>, _>("pipeline_revision") != Some(request.pipeline_revision)
    {
        return Err("state_conflict");
    }
    let definition = super::pipelines::definition_value(
        connection,
        &workspace,
        &request.pipeline_id.to_string(),
        request.pipeline_revision,
    )
    .await?;
    if definition["nodes"][0]["operation"] != request.operation
        || definition["nodes"][0]["operation_version"] != request.operation_version
    {
        return Err("pipeline_mismatch");
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
        parameters: definition["nodes"][0]["parameters"].clone(),
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
    let parameters = invocation.parameters.clone();
    contract
        .validate_parameters(&parameters)
        .map_err(|_| "contract_mismatch")?;
    let name = descriptor
        .metadata()
        .map_or(descriptor.command, |metadata| metadata.canonical_name);
    let pipeline_id = request.pipeline_id.to_string();
    let pipeline_revision = request.pipeline_revision;
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
    let value = json!({"formatVersion":1,"trail":{"id":request.trail_id,"revision":row.get::<i64,_>("revision"),"hypothesis":row.get::<String,_>("hypothesis")},"pipeline":{"id":pipeline_id,"revision":pipeline_revision,"definition":definition},"targetNodes":[node_id],"reusePolicy":"none","reusedOutputs":[],"caller":"desktop","adapterVersion":1,"contract":contract,"contractDigest":cube_core::hashing::sha256(serde_json::to_string(&contract).map_err(|_| "invalid_state")?.as_bytes()),"operation":{"operation":invocation.operation,"operationVersion":invocation.operation_version,"name":name},"inputs":inputs,"parameters":parameters,"limits":{"runMs":crate::protocol::contract().run_ms,"cpuSeconds":crate::protocol::contract().cpu_seconds,"vmGrowthBytes":crate::protocol::contract().vm_growth_bytes,"outputBytes":crate::protocol::contract().output_bytes}});
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
