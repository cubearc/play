//! Freeze only the selected node's dependency closure, not alternative graph branches.
use super::{
    candidates,
    model::{CandidateSelection, Context},
    queries, solutions, storage,
};
use crate::{
    Result,
    graph::{Definition, Source},
};
use serde_json::{Value, json};
use sqlx::{Row, SqliteConnection};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

pub async fn build(
    connection: &mut SqliteConnection,
    context: &Context,
    selected: &CandidateSelection,
) -> Result<Value> {
    let CandidateSelection::RunOutput {
        run_id,
        attempt_id,
        port,
    } = selected
    else {
        return Ok(Value::Null);
    };
    let row=sqlx::query("SELECT r.plan,n.node_id FROM pipeline_runs r JOIN node_attempts n ON (r.workspace_id,r.exploration_id,r.id)=(n.workspace_id,n.exploration_id,n.run_id) WHERE r.workspace_id=? AND r.exploration_id=? AND r.stage_id=? AND r.id=? AND n.id=?")
        .bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(context.stage_id.to_string()).bind(run_id.to_string()).bind(attempt_id.to_string()).fetch_optional(&mut *connection).await.map_err(storage::failure)?.ok_or("not_found")?;
    let plan: Value = serde_json::from_str(row.get("plan")).map_err(|_| "corrupt_store")?;
    let mut graph = Definition::parse(plan["pipeline"]["definition"].clone())?;
    graph.output = Uuid::parse_str(row.get("node_id")).map_err(|_| "corrupt_store")?;
    let schedule = graph.schedule()?;
    graph.nodes.retain(|n| schedule.contains(&n.id));
    let ports = required_ports(&graph, port);
    let mut nodes = Vec::new();
    for id in schedule {
        let node = graph.node(id)?;
        let pinned = plan["nodes"]
            .as_array()
            .and_then(|n| n.iter().find(|v| v["id"] == id.to_string()))
            .ok_or("corrupt_store")?;
        candidates::output_contract(&plan, pinned)?;
        let attempt:String=sqlx::query_scalar("SELECT id FROM node_attempts WHERE workspace_id=? AND exploration_id=? AND run_id=? AND node_id=? AND status='completed'")
            .bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(run_id.to_string()).bind(id.to_string()).fetch_optional(&mut *connection).await.map_err(storage::failure)?.ok_or("result_not_available")?;
        let used = ports.get(&id).ok_or("corrupt_store")?;
        for name in used {
            candidates::evidence(
                connection,
                context,
                &CandidateSelection::RunOutput {
                    run_id: *run_id,
                    attempt_id: Uuid::parse_str(&attempt).map_err(|_| "corrupt_store")?,
                    port: name.clone(),
                },
            )
            .await?;
        }
        nodes.push(json!({"id":id,"name":pinned["operation"]["name"],"operation":node.operation,"operationVersion":node.operation_version,"parameters":node.parameters,"inputs":node.inputs,"contractDigest":pinned["contractDigest"],"attemptId":attempt,"ports":used}));
    }
    let inputs = graph
        .required_inputs()
        .iter()
        .map(|name| {
            Ok((
                (*name).to_owned(),
                plan["inputs"].get(*name).ok_or("corrupt_store")?.clone(),
            ))
        })
        .collect::<Result<serde_json::Map<_, _>>>()?;
    let path = json!({"runId":run_id,"trail":plan["trail"],"pipeline":{"id":plan["pipeline"]["id"],"revision":plan["pipeline"]["revision"]},"nodes":nodes,"inputs":inputs});
    if !gaps(connection, context, &path).await?.is_empty() {
        return Err("result_not_available");
    }
    Ok(path)
}
fn required_ports(graph: &Definition, output: &str) -> BTreeMap<Uuid, BTreeSet<String>> {
    let mut ports = BTreeMap::from([(graph.output, BTreeSet::from([output.to_owned()]))]);
    for node in &graph.nodes {
        for source in node.inputs.values() {
            if let Source::Output { node_id, port } = source {
                ports.entry(*node_id).or_default().insert(port.clone());
            }
        }
    }
    ports
}
pub async fn gaps(
    connection: &mut SqliteConnection,
    context: &Context,
    path: &Value,
) -> Result<Vec<Value>> {
    if path.is_null() {
        return Ok(Vec::new());
    }
    let mut gaps = Vec::new();
    for (slot, input) in path["inputs"].as_object().ok_or("corrupt_store")? {
        let result = queries::asset(
            connection,
            &context.workspace_id.to_string(),
            &context.exploration_id.to_string(),
            input["assetId"].as_str().ok_or("corrupt_store")?,
            input["revision"].as_i64().ok_or("corrupt_store")?,
        )
        .await;
        let value = solutions::available(result.map(|_| Value::Null))?;
        if value["kind"] == "unavailable" {
            gaps.push(
                json!({"reference":format!("input:{slot}"),"source":slot,"code":value["code"]}),
            );
        }
    }
    let run = Uuid::parse_str(path["runId"].as_str().ok_or("corrupt_store")?)
        .map_err(|_| "corrupt_store")?;
    for node in path["nodes"].as_array().ok_or("corrupt_store")? {
        let attempt = Uuid::parse_str(node["attemptId"].as_str().ok_or("corrupt_store")?)
            .map_err(|_| "corrupt_store")?;
        for port in node["ports"].as_array().ok_or("corrupt_store")? {
            let selected = CandidateSelection::RunOutput {
                run_id: run,
                attempt_id: attempt,
                port: port.as_str().ok_or("corrupt_store")?.into(),
            };
            let value =
                solutions::available(candidates::evidence(connection, context, &selected).await)?;
            if value["kind"] == "unavailable" {
                gaps.push(json!({"reference":format!("{attempt}:{}",port.as_str().ok_or("corrupt_store")?),"source":format!("{} · {}",node["name"].as_str().ok_or("corrupt_store")?,port.as_str().ok_or("corrupt_store")?),"code":value["code"]}));
            }
        }
    }
    Ok(gaps)
}
