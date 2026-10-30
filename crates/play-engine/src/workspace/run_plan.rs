use super::{
    bindings, blocks,
    model::{Content, RunRequest},
    queries, storage,
};
use crate::{
    Result,
    worker::{InputValue, PipelineInvocation},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use sqlx::{Row, SqliteConnection};
use std::collections::BTreeMap;
use uuid::Uuid;

pub struct Plan {
    pub value: Value,
    pub invocation: PipelineInvocation,
    pub pipeline_id: String,
    pub pipeline_revision: i64,
    pub node_id: Uuid,
    pub inputs: BTreeMap<String, super::model::AssetRevision>,
}

pub async fn build(
    connection: &mut SqliteConnection,
    request: &RunRequest,
    caller: super::runs::Caller,
) -> Result<Plan> {
    let context = &request.context;
    let workspace = context.workspace_id.to_string();
    let exploration = context.exploration_id.to_string();
    let row=sqlx::query("SELECT source_id,source_revision,pipeline_id,pipeline_revision,revision,hypothesis FROM trails WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND id=?")
  .bind(&workspace).bind(&exploration).bind(context.stage_id.to_string()).bind(request.trail_id.to_string()).fetch_optional(&mut *connection).await.map_err(storage::failure)?.ok_or("not_found")?;
    if row.get::<i64, _>("revision") != request.trail_revision {
        return Err("state_conflict");
    }
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
    let target = graph.node(request.target_node_id)?;
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
    let mut assets = bindings::read(
        connection,
        &workspace,
        &exploration,
        &request.trail_id.to_string(),
    )
    .await?;
    assets.insert("source".into(), asset);
    if assets.len() != graph.inputs.len() {
        return Err("invalid_binding");
    }
    let mut selected = graph.clone();
    selected.output = request.target_node_id;
    let schedule = selected.schedule()?;
    selected.nodes.retain(|n| schedule.contains(&n.id));
    let required = selected.required_inputs();
    let mut input_data = BTreeMap::new();
    let mut refs = BTreeMap::new();
    let mut inputs = serde_json::Map::new();
    for slot in &graph.inputs {
        let asset = assets.get(&slot.name).ok_or("invalid_binding")?;
        blocks::source_active(connection, context, &asset.id).await?;
        let kind = bindings::semantic_type(asset)?;
        if kind != slot.semantic_type {
            return Err("invalid_asset_type");
        }
        let used = required.contains(slot.name.as_str());
        inputs.insert(
            slot.name.clone(),
            json!({"assetId":asset.id,"revision":asset.revision,"semanticType":kind,"used":used}),
        );
        refs.insert(
            slot.name.clone(),
            super::model::AssetRevision {
                asset_id: Uuid::parse_str(&asset.id).map_err(|_| "corrupt_store")?,
                revision: asset.revision,
            },
        );
        if used {
            let data = match &asset.content {
                Content::Text { text } | Content::Page { text } => {
                    InputValue::Text { text: text.clone() }
                }
                Content::Image => {
                    let bytes:Vec<u8>=sqlx::query_scalar("SELECT data FROM asset_revisions WHERE workspace_id=? AND exploration_id=? AND asset_id=? AND revision=?").bind(&workspace).bind(&exploration).bind(&asset.id).bind(asset.revision).fetch_one(&mut *connection).await.map_err(storage::failure)?;
                    InputValue::Image {
                        png: STANDARD.encode(bytes),
                    }
                }
                _ => return Err("invalid_asset_type"),
            };
            input_data.insert(slot.name.clone(), data);
        }
    }
    let invocation = PipelineInvocation {
        definition: selected,
        inputs: input_data,
    };
    crate::worker::validate_pipeline(&invocation)?;
    let contract = target.contract()?;
    let pipeline_id = request.pipeline_id.to_string();
    let pipeline_revision = request.pipeline_revision;
    let node_id = *schedule.first().ok_or("invalid_pipeline")?;
    let mut nodes = schedule
        .iter()
        .map(|id| node_plan(graph.node(*id)?))
        .collect::<Result<Vec<_>>>()?;
    let target_plan = node_plan(target)?;
    let contracts = compact_contracts(&mut nodes, &target_plan)?;
    let value = json!({"formatVersion":3,"trail":{"id":request.trail_id,"revision":row.get::<i64,_>("revision"),"hypothesis":row.get::<String,_>("hypothesis")},"pipeline":{"id":pipeline_id,"revision":pipeline_revision,"definition":definition},"targetNodes":[request.target_node_id],"executionOrder":schedule,"nodes":nodes,"contracts":contracts,"reusePolicy":"none","reusedOutputs":[],"caller":caller.as_str(),"adapterVersion":3,"contract":contract,"contractDigest":target_plan["contractDigest"],"operation":target_plan["operation"],"inputs":inputs,"parameters":target.parameters,"limits":{"parallelNodes":crate::protocol::contract().parallel_nodes,"runMs":crate::protocol::contract().run_ms,"cpuSeconds":crate::protocol::contract().cpu_seconds,"vmGrowthBytes":crate::protocol::contract().vm_growth_bytes,"outputBytes":crate::protocol::contract().output_bytes}});
    if value.to_string().len() > 16384 {
        return Err("output_limit");
    }
    Ok(Plan {
        value,
        invocation,
        pipeline_id,
        pipeline_revision,
        node_id,
        inputs: refs,
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

fn compact_contracts(nodes: &mut [Value], target: &Value) -> Result<Value> {
    let mut contracts = serde_json::Map::new();
    for node in nodes {
        let digest = node["contractDigest"]
            .as_str()
            .ok_or("invalid_state")?
            .to_owned();
        let fields = node.as_object_mut().ok_or("invalid_state")?;
        let contract = fields.remove("contract").ok_or("invalid_state")?;
        fields.remove("parameters");
        fields.remove("inputs");
        if digest != target["contractDigest"] {
            contracts.entry(digest).or_insert(contract);
        }
    }
    Ok(Value::Object(contracts))
}
