use super::{
    blocks,
    model::{Content, RunRequest},
    queries, storage,
};
use crate::{Result, worker::PipelineInvocation};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use sqlx::{Row, SqliteConnection};
use uuid::Uuid;

pub struct Plan {
    pub value: Value,
    pub invocation: PipelineInvocation,
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
    let graph = crate::graph::Definition::parse(definition.clone())?;
    let target = graph.node(graph.output)?;
    if target.operation != request.operation
        || target.operation_version != request.operation_version
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
        Content::Text { text } if graph.inputs[0].semantic_type == "text" => (text, "text"),
        Content::Image if graph.inputs[0].semantic_type == "image.samples" => {
            let bytes:Vec<u8>=sqlx::query_scalar("SELECT data FROM asset_revisions WHERE workspace_id=? AND exploration_id=? AND asset_id=? AND revision=?").bind(&workspace).bind(&exploration).bind(&asset_id).bind(asset_revision).fetch_one(&mut *connection).await.map_err(storage::failure)?;
            (STANDARD.encode(bytes), "image.samples")
        }
        _ => return Err("invalid_asset_type"),
    };
    let schedule = graph.schedule()?;
    let mut selected = graph.clone();
    selected.nodes.retain(|n| schedule.contains(&n.id));
    let invocation = PipelineInvocation {
        definition: selected,
        input: text,
    };
    crate::worker::validate_pipeline(&invocation)?;
    let contract = target.contract()?;
    let pipeline_id = request.pipeline_id.to_string();
    let pipeline_revision = request.pipeline_revision;
    let node_id = *schedule.first().ok_or("invalid_pipeline")?;
    let input_port = "source".to_owned();
    let inputs = json!({"source":{"assetId":asset_id,"revision":asset_revision,"semanticType":semantic_type}});
    let nodes = schedule
        .iter()
        .map(|id| node_plan(graph.node(*id)?))
        .collect::<Result<Vec<_>>>()?;
    let target_plan = node_plan(target)?;
    let value = json!({"formatVersion":2,"trail":{"id":request.trail_id,"revision":row.get::<i64,_>("revision"),"hypothesis":row.get::<String,_>("hypothesis")},"pipeline":{"id":pipeline_id,"revision":pipeline_revision,"definition":definition},"targetNodes":[graph.output],"executionOrder":schedule,"nodes":nodes,"reusePolicy":"none","reusedOutputs":[],"caller":"desktop","adapterVersion":2,"contract":contract,"contractDigest":target_plan["contractDigest"],"operation":target_plan["operation"],"inputs":inputs,"parameters":target.parameters,"limits":{"runMs":crate::protocol::contract().run_ms,"cpuSeconds":crate::protocol::contract().cpu_seconds,"vmGrowthBytes":crate::protocol::contract().vm_growth_bytes,"outputBytes":crate::protocol::contract().output_bytes}});
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

pub fn node_plan(node: &crate::graph::Node) -> Result<Value> {
    let d = cube_core::operations::require(&node.operation).map_err(|_| "unsupported_operation")?;
    let c = node.contract()?;
    let name = d.metadata().map_or(d.command, |m| m.canonical_name);
    Ok(
        json!({"id":node.id,"operation":{"operation":node.operation,"operationVersion":node.operation_version,"name":name},"contract":c,"contractDigest":cube_core::hashing::sha256(&serde_json::to_vec(&c).map_err(|_|"invalid_state")?),"parameters":node.parameters,"inputs":node.inputs}),
    )
}
