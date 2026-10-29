use super::{
    model::{Context, Pipeline, PipelineOrigin, PipelineRef},
    mutations, storage,
};
use crate::{Result, worker};
use serde_json::{Value, json};
use sqlx::{Row, SqliteConnection};
use uuid::Uuid;

use crate::graph::Definition;
fn parse(text: &str) -> Result<Definition> {
    Definition::parse(serde_json::from_str(text).map_err(|_| "unsupported_pipeline")?)
}
async fn stored(c: &mut SqliteConnection, w: &str, id: &str, rev: i64) -> Result<String> {
    sqlx::query_scalar("SELECT definition FROM pipeline_revisions WHERE workspace_id=? AND pipeline_id=? AND revision=?").bind(w).bind(id).bind(rev).fetch_optional(c).await.map_err(storage::failure)?.ok_or("not_found")
}
pub async fn definition_value(
    c: &mut SqliteConnection,
    w: &str,
    id: &str,
    revision: i64,
) -> Result<Value> {
    serde_json::to_value(parse(&stored(c, w, id, revision).await?)?).map_err(|_| "invalid_state")
}
pub async fn read(c: &mut SqliteConnection, w: &str, id: &str, rev: i64) -> Result<Pipeline> {
    let d = parse(&stored(c, w, id, rev).await?)?;
    let title = sqlx::query_scalar("SELECT title FROM pipelines WHERE workspace_id=? AND id=?")
        .bind(w)
        .bind(id)
        .fetch_one(&mut *c)
        .await
        .map_err(storage::failure)?;
    let revisions=sqlx::query_scalar("SELECT revision FROM pipeline_revisions WHERE workspace_id=? AND pipeline_id=? AND json_extract(definition,'$.formatVersion')=3 ORDER BY revision DESC").bind(w).bind(id).fetch_all(&mut *c).await.map_err(storage::failure)?;
    let origin=sqlx::query("SELECT source_pipeline_id,source_revision FROM pipeline_origins WHERE workspace_id=? AND pipeline_id=?").bind(w).bind(id).fetch_optional(c).await.map_err(storage::failure)?.map(|r|PipelineOrigin{pipeline_id:r.get(0),revision:r.get(1)});
    let node = d.node(d.output)?;
    Ok(Pipeline {
        definition: serde_json::to_value(&d).map_err(|_| "invalid_state")?,
        output_node_id: d.output.to_string(),
        id: id.into(),
        title,
        revision: rev,
        output_operation: node.operation.clone(),
        operation_version: node.operation_version,
        input_type: d.inputs[0].semantic_type.clone(),
        revisions,
        origin,
    })
}
async fn insert(c: &mut SqliteConnection, w: &str, title: &str, d: &Definition) -> Result<Value> {
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM pipelines WHERE workspace_id=?")
        .bind(w)
        .fetch_one(&mut *c)
        .await
        .map_err(storage::failure)?;
    if count >= crate::protocol::contract().pipeline_limit as i64 {
        return Err("storage_quota");
    }
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO pipelines VALUES(?,?,?,1)")
        .bind(w)
        .bind(id.to_string())
        .bind(mutations::title(title)?)
        .execute(&mut *c)
        .await
        .map_err(storage::failure)?;
    sqlx::query("INSERT INTO pipeline_revisions VALUES(?,?,1,?)")
        .bind(w)
        .bind(id.to_string())
        .bind(serde_json::to_string(d).map_err(|_| "invalid_state")?)
        .execute(c)
        .await
        .map_err(storage::failure)?;
    Ok(json!({"pipelineId":id,"revision":1}))
}
pub async fn create(
    c: &mut SqliteConnection,
    w: &str,
    title: &str,
    value: &Value,
) -> Result<Value> {
    insert(c, w, title, &Definition::parse(value.clone())?).await
}
pub async fn revise(
    c: &mut SqliteConnection,
    w: &str,
    id: &str,
    expected: i64,
    value: &Value,
) -> Result<Value> {
    let head: i64 =
        sqlx::query_scalar("SELECT current_revision FROM pipelines WHERE workspace_id=? AND id=?")
            .bind(w)
            .bind(id)
            .fetch_optional(&mut *c)
            .await
            .map_err(storage::failure)?
            .ok_or("not_found")?;
    if expected != head {
        return Err("state_conflict");
    }
    let old = stored(c, w, id, head).await?;
    let next = Definition::parse(value.clone())?;
    let next = serde_json::to_string(&next).map_err(|_| "invalid_state")?;
    if serde_json::from_str::<Value>(&old).map_err(|_| "corrupt_store")?
        == serde_json::from_str::<Value>(&next).map_err(|_| "invalid_state")?
    {
        return Ok(json!({"pipelineId":id,"revision":head}));
    }
    let count:i64=sqlx::query_scalar("SELECT count(*) FROM pipeline_revisions WHERE workspace_id=? AND pipeline_id=? AND json_extract(definition,'$.formatVersion')=3").bind(w).bind(id).fetch_one(&mut *c).await.map_err(storage::failure)?;
    if count >= crate::protocol::contract().pipeline_revision_limit as i64 {
        return Err("storage_quota");
    }
    sqlx::query("INSERT INTO pipeline_revisions VALUES(?,?,?,?)")
        .bind(w)
        .bind(id)
        .bind(head + 1)
        .bind(next)
        .execute(&mut *c)
        .await
        .map_err(storage::failure)?;
    sqlx::query("UPDATE pipelines SET current_revision=? WHERE workspace_id=? AND id=? AND current_revision=?").bind(head+1).bind(w).bind(id).bind(head).execute(c).await.map_err(storage::failure)?;
    Ok(json!({"pipelineId":id,"revision":head+1}))
}
pub async fn fork(
    c: &mut SqliteConnection,
    w: &str,
    id: &str,
    rev: i64,
    title: &str,
) -> Result<Value> {
    let d = parse(&stored(c, w, id, rev).await?)?;
    let result = insert(c, w, title, &d).await?;
    sqlx::query("INSERT INTO pipeline_origins VALUES(?,?,?,?)")
        .bind(w)
        .bind(result["pipelineId"].as_str().ok_or("invalid_state")?)
        .bind(id)
        .bind(rev)
        .execute(c)
        .await
        .map_err(storage::failure)?;
    Ok(result)
}
pub async fn default_for(
    c: &mut SqliteConnection,
    w: &str,
    operation: &str,
) -> Result<PipelineRef> {
    let descriptor =
        cube_core::operations::require(operation).map_err(|_| "unsupported_operation")?;
    let version = descriptor
        .contract()
        .map_err(|_| "contract_mismatch")?
        .operation_version;
    let params = worker::parameters(operation)?;
    let rows=sqlx::query("SELECT pipeline_id,revision,definition FROM pipeline_revisions WHERE workspace_id=? ORDER BY rowid").bind(w).fetch_all(&mut *c).await.map_err(storage::failure)?;
    for row in rows {
        if let Ok(d) = parse(row.get("definition"))
            && d.nodes.len() == 1
            && serde_json::to_value(&d).map_err(|_| "invalid_state")?
                == serde_json::to_value(Definition::single(
                    d.nodes[0].id,
                    operation,
                    version,
                    params.clone(),
                )?)
                .map_err(|_| "invalid_state")?
        {
            return Ok(PipelineRef {
                id: Uuid::parse_str(row.get("pipeline_id")).map_err(|_| "corrupt_store")?,
                revision: row.get("revision"),
            });
        }
    }

    let name = descriptor
        .metadata()
        .map_or(descriptor.command, |m| m.canonical_name);
    let result = insert(
        c,
        w,
        name,
        &Definition::single(Uuid::new_v4(), operation, version, params)?,
    )
    .await?;
    Ok(PipelineRef {
        id: serde_json::from_value(result["pipelineId"].clone()).map_err(|_| "invalid_state")?,
        revision: 1,
    })
}
pub async fn save_trail(
    c: &mut SqliteConnection,
    ctx: &Context,
    trail: Uuid,
    expected: i64,
    hypothesis: &str,
    pipeline: Option<&PipelineRef>,
) -> Result<Value> {
    if hypothesis.len() > crate::protocol::contract().hypothesis_bytes {
        return Err("input_limit");
    }
    let w = ctx.workspace_id.to_string();
    let e = ctx.exploration_id.to_string();
    let row=sqlx::query("SELECT revision,hypothesis,source_id,source_revision FROM trails WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND id=?").bind(&w).bind(&e).bind(ctx.stage_id.to_string()).bind(trail.to_string()).fetch_optional(&mut *c).await.map_err(storage::failure)?.ok_or("not_found")?;
    if row.get::<i64, _>("revision") != expected {
        return Err("state_conflict");
    }
    super::blocks::source_active(c, ctx, row.get("source_id")).await?;
    if let Some(p) = pipeline {
        let d = read(c, &w, &p.id.to_string(), p.revision).await?;
        let source =
            super::queries::asset(c, &w, &e, row.get("source_id"), row.get("source_revision"))
                .await?;
        let input = match source.content {
            super::model::Content::Text { .. } => "text",
            super::model::Content::Image => "image.samples",
            _ => return Err("invalid_asset_type"),
        };
        if d.input_type != input {
            return Err("invalid_asset_type");
        }
    }
    let previous: String = row.get("hypothesis");
    mutations::bytes(c, &w, &e, hypothesis.len().saturating_sub(previous.len())).await?;
    sqlx::query("UPDATE trails SET hypothesis=?,pipeline_id=?,pipeline_revision=?,revision=revision+1 WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND id=? AND revision=?").bind(hypothesis).bind(pipeline.map(|p|p.id.to_string())).bind(pipeline.map(|p|p.revision)).bind(w).bind(e).bind(ctx.stage_id.to_string()).bind(trail.to_string()).bind(expected).execute(c).await.map_err(storage::failure)?;
    Ok(json!({"trailId":trail,"revision":expected+1}))
}
// Preserve immutable historical definitions and Run snapshots; publish complete working revisions.
pub async fn upgrade(c: &mut SqliteConnection) -> Result<()> {
    let rows=sqlx::query("SELECT p.workspace_id,p.id,p.current_revision,r.definition FROM pipelines p JOIN pipeline_revisions r ON (p.workspace_id,p.id,p.current_revision)=(r.workspace_id,r.pipeline_id,r.revision)").fetch_all(&mut *c).await.map_err(storage::failure)?;
    for row in rows {
        let raw: Value =
            serde_json::from_str(row.get("definition")).map_err(|_| "corrupt_store")?;
        if matches!(raw["formatVersion"].as_u64(), Some(2 | 3)) {
            continue;
        }
        if raw["formatVersion"] != 1 || raw["nodes"].as_array().is_none_or(|n| n.len() != 1) {
            return Err("unsupported_pipeline");
        }
        let w: String = row.get("workspace_id");
        let id: String = row.get("id");
        let rev: i64 = row.get("current_revision");
        let node = &raw["nodes"][0];
        let op = node["operation"].as_str().ok_or("corrupt_store")?;
        let d = historical_definition(
            serde_json::from_value(node["id"].clone()).map_err(|_| "corrupt_store")?,
            op,
            serde_json::from_value(node["operation_version"].clone())
                .map_err(|_| "corrupt_store")?,
            &worker::parameters(op)?,
        )?;
        sqlx::query("INSERT INTO pipeline_revisions VALUES(?,?,?,?)")
            .bind(&w)
            .bind(&id)
            .bind(rev + 1)
            .bind(serde_json::to_string(&d).map_err(|_| "invalid_state")?)
            .execute(&mut *c)
            .await
            .map_err(storage::failure)?;
        sqlx::query("UPDATE pipelines SET current_revision=? WHERE workspace_id=? AND id=?")
            .bind(rev + 1)
            .bind(&w)
            .bind(&id)
            .execute(&mut *c)
            .await
            .map_err(storage::failure)?;
        sqlx::query("UPDATE trails SET pipeline_revision=?,revision=revision+1 WHERE workspace_id=? AND pipeline_id=? AND pipeline_revision=?").bind(rev+1).bind(w).bind(id).bind(rev).execute(&mut *c).await.map_err(storage::failure)?;
    }
    Ok(())
}

fn historical_definition(id: Uuid, op: &str, version: u32, params: &Value) -> Result<Value> {
    let d = Definition::single(id, op, version, params.clone())?;
    Ok(
        json!({"formatVersion":2,"inputs":d.inputs,"nodes":[{"id":id,"operation":op,"operation_version":version,"input":"source","parameters":params}]}),
    )
}
pub async fn upgrade_graphs(c: &mut SqliteConnection) -> Result<()> {
    let rows=sqlx::query("SELECT workspace_id,pipeline_id,revision,definition FROM pipeline_revisions WHERE json_extract(definition,'$.formatVersion')=2 ORDER BY workspace_id,pipeline_id,revision").fetch_all(&mut *c).await.map_err(storage::failure)?;
    for r in rows {
        let raw: Value = serde_json::from_str(r.get("definition")).map_err(|_| "corrupt_store")?;
        let n = &raw["nodes"][0];
        let d = Definition::single(
            serde_json::from_value(n["id"].clone()).map_err(|_| "corrupt_store")?,
            n["operation"].as_str().ok_or("corrupt_store")?,
            serde_json::from_value(n["operation_version"].clone()).map_err(|_| "corrupt_store")?,
            n["parameters"].clone(),
        )?;
        let w: String = r.get("workspace_id");
        let id: String = r.get("pipeline_id");
        let rev: i64 = r.get("revision");
        let head: i64 = sqlx::query_scalar(
            "SELECT current_revision FROM pipelines WHERE workspace_id=? AND id=?",
        )
        .bind(&w)
        .bind(&id)
        .fetch_one(&mut *c)
        .await
        .map_err(storage::failure)?;
        let next = head + 1;
        sqlx::query("INSERT INTO pipeline_revisions VALUES(?,?,?,?)")
            .bind(&w)
            .bind(&id)
            .bind(next)
            .bind(serde_json::to_string(&d).map_err(|_| "invalid_state")?)
            .execute(&mut *c)
            .await
            .map_err(storage::failure)?;
        sqlx::query("UPDATE pipelines SET current_revision=? WHERE workspace_id=? AND id=?")
            .bind(next)
            .bind(&w)
            .bind(&id)
            .execute(&mut *c)
            .await
            .map_err(storage::failure)?;
        sqlx::query("UPDATE trails SET pipeline_revision=?,revision=revision+1 WHERE workspace_id=? AND pipeline_id=? AND pipeline_revision=?").bind(next).bind(&w).bind(&id).bind(rev).execute(&mut *c).await.map_err(storage::failure)?;
    }
    Ok(())
}
