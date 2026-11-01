//! Capture recorded creation ancestry; edited copies remain distinct from Run outputs.
use super::{
    model::{AssetOrigin, CandidateSelection, Context},
    queries, solution_path, solutions, storage,
};
use crate::{Result, protocol};
use serde_json::{Value, json};
use sqlx::SqliteConnection;
use std::collections::BTreeSet;
use uuid::Uuid;

fn uuid(value: &str) -> Result<Uuid> {
    Uuid::parse_str(value).map_err(|_| "corrupt_store")
}
fn inputs(path: &Value) -> Result<Vec<CandidateSelection>> {
    path["inputs"]
        .as_object()
        .ok_or("corrupt_store")?
        .values()
        .map(|input| {
            Ok(CandidateSelection::Asset {
                asset_id: uuid(input["assetId"].as_str().ok_or("corrupt_store")?)?,
                revision: input["revision"].as_i64().ok_or("corrupt_store")?,
            })
        })
        .collect()
}
fn source(origin: &AssetOrigin) -> Result<CandidateSelection> {
    Ok(match origin {
        AssetOrigin::Asset {
            asset_id, revision, ..
        } => CandidateSelection::Asset {
            asset_id: uuid(asset_id)?,
            revision: *revision,
        },
        AssetOrigin::RunOutput {
            run_id,
            attempt_id,
            port,
            ..
        } => CandidateSelection::RunOutput {
            run_id: uuid(run_id)?,
            attempt_id: uuid(attempt_id)?,
            port: port.clone(),
        },
    })
}
enum Visit {
    Enter(CandidateSelection),
    Leave(CandidateSelection),
}

pub async fn build(
    connection: &mut SqliteConnection,
    context: &Context,
    selected: &CandidateSelection,
    path: &Value,
) -> Result<Value> {
    let roots = if path.is_null() {
        vec![selected.clone()]
    } else {
        inputs(path)?
    };
    let mut pending: Vec<_> = roots.into_iter().map(Visit::Enter).collect();
    let mut seen = BTreeSet::new();
    let mut active = BTreeSet::new();
    let mut assets = Vec::new();
    let mut paths = Vec::new();
    let limits = protocol::contract();
    while let Some(visit) = pending.pop() {
        let reference = match visit {
            Visit::Leave(reference) => {
                active.remove(&reference);
                continue;
            }
            Visit::Enter(reference) => reference,
        };
        if active.contains(&reference) {
            return Err("corrupt_store");
        }
        if !seen.insert(reference.clone()) {
            continue;
        }
        if seen.len() > limits.solution_lineage_limit {
            return Err("output_limit");
        }
        active.insert(reference.clone());
        pending.push(Visit::Leave(reference.clone()));
        let children = capture(connection, context, &reference, &mut assets, &mut paths).await?;
        // Bound both traversal work and accumulated metadata before following more origins.
        if json!({"assets":assets,"paths":paths}).to_string().len() > limits.solution_snapshot_bytes
        {
            return Err("output_limit");
        }
        pending.extend(children.into_iter().map(Visit::Enter));
    }
    Ok(json!({"assets":assets,"paths":paths}))
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct RunLocation {
    run_id: Uuid,
    trail_id: Uuid,
    stage_id: Uuid,
}
impl RunLocation {
    fn context(&self, owner: &Context) -> Context {
        Context {
            stage_id: self.stage_id,
            ..owner.clone()
        }
    }
}
// Only recorded ancestry reaches this lookup. Direct selections remain Stage-scoped.
async fn source_run(
    connection: &mut SqliteConnection,
    context: &Context,
    run_id: Uuid,
) -> Result<RunLocation> {
    let (stage, trail): (String, String) = sqlx::query_as(
        "SELECT stage_id,trail_id FROM pipeline_runs WHERE workspace_id=? AND exploration_id=? AND id=?",
    )
    .bind(context.workspace_id.to_string())
    .bind(context.exploration_id.to_string())
    .bind(run_id.to_string())
    .fetch_optional(connection)
    .await
    .map_err(storage::failure)?
    .ok_or("not_found")?;
    Ok(RunLocation {
        run_id,
        trail_id: uuid(&trail)?,
        stage_id: uuid(&stage)?,
    })
}

/// Resolve navigation from immutable recorded paths without rewriting snapshots.
pub(super) async fn locations(
    connection: &mut SqliteConnection,
    context: &Context,
    snapshot: &Value,
) -> Result<Vec<RunLocation>> {
    let paths = snapshot["lineage"]["paths"]
        .as_array()
        .ok_or("corrupt_store")?;
    let mut seen = BTreeSet::new();
    let mut locations = Vec::new();
    for path in std::iter::once(&snapshot["path"]).chain(paths.iter().map(|entry| &entry["path"])) {
        if path.is_null() {
            continue;
        }
        let id = uuid(path["runId"].as_str().ok_or("corrupt_store")?)?;
        if !seen.insert(id) {
            continue;
        }
        match source_run(connection, context, id).await {
            Ok(location) => locations.push(location),
            Err("not_found") => (), // Missing evidence is reported by the gap resolver.
            Err(code) => return Err(code),
        }
    }
    Ok(locations)
}

async fn capture(
    connection: &mut SqliteConnection,
    context: &Context,
    reference: &CandidateSelection,
    assets: &mut Vec<Value>,
    paths: &mut Vec<Value>,
) -> Result<Vec<CandidateSelection>> {
    let children = match reference {
        CandidateSelection::Asset { asset_id, revision } => {
            let asset = queries::asset(
                connection,
                &context.workspace_id.to_string(),
                &context.exploration_id.to_string(),
                &asset_id.to_string(),
                *revision,
            )
            .await?;
            let children = asset
                .origin
                .as_deref()
                .map(source)
                .transpose()?
                .into_iter()
                .collect();
            let edited =
                *revision > 1 && matches!(asset.origin.as_deref(), Some(AssetOrigin::Asset { .. }));
            assets.push(json!({"assetId":asset_id,"revision":revision,"title":asset.title,"origin":asset.origin,"editedSinceCopy":edited}));
            children
        }
        CandidateSelection::RunOutput { run_id, .. } => {
            let source = source_run(connection, context, *run_id).await?;
            let path =
                solution_path::build(connection, &source.context(context), reference).await?;
            let children = inputs(&path)?;
            paths.push(json!({"selection":reference,"path":path}));
            children
        }
    };
    Ok(children)
}

pub async fn gaps(
    connection: &mut SqliteConnection,
    context: &Context,
    lineage: &Value,
) -> Result<Vec<Value>> {
    let mut gaps = Vec::new();
    for asset in lineage["assets"].as_array().ok_or("corrupt_store")? {
        let id = asset["assetId"].as_str().ok_or("corrupt_store")?;
        let revision = asset["revision"].as_i64().ok_or("corrupt_store")?;
        let result = queries::asset(
            connection,
            &context.workspace_id.to_string(),
            &context.exploration_id.to_string(),
            id,
            revision,
        )
        .await;
        let available = solutions::available(result.map(|_| Value::Null))?;
        if available["kind"] == "unavailable" {
            gaps.push(json!({"reference":format!("asset:{id}:{revision}"),"source":format!("{} · r{revision}",asset["title"].as_str().ok_or("corrupt_store")?),"code":available["code"]}));
        }
    }
    for entry in lineage["paths"].as_array().ok_or("corrupt_store")? {
        let run_id = uuid(entry["path"]["runId"].as_str().ok_or("corrupt_store")?)?;
        match source_run(connection, context, run_id).await {
            Ok(source) => gaps.extend(solution_path::gaps(connection, &source.context(context), &entry["path"]).await?),
            Err("not_found") => gaps.push(json!({"reference":format!("run:{run_id}"),"source":"Contributing Run","code":"not_found"})),
            Err(code) => return Err(code),
        }
    }
    Ok(gaps)
}
