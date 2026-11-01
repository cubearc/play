use super::{
    model::{Context, StageProgress, StageSummary},
    mutations, storage,
};
use crate::Result;
use serde_json::{Value, json};
use sqlx::{Row, SqliteConnection, sqlite::SqliteRow};
use uuid::Uuid;

pub(super) fn summary(row: &SqliteRow) -> Result<StageSummary> {
    let progress = match row.get::<&str, _>("progress") {
        "not_started" => StageProgress::NotStarted,
        "in_progress" => StageProgress::InProgress,
        "completed" => StageProgress::Completed,
        _ => return Err("corrupt_store"),
    };
    Ok(StageSummary {
        id: row.get("id"),
        title: row.get("title"),
        revision: row.get("revision"),
        progress,
        archived: row.get("archived"),
    })
}
pub(super) async fn writable(c: &mut SqliteConnection, context: &Context) -> Result<()> {
    let archived: bool = sqlx::query_scalar(
        "SELECT archived FROM stages WHERE workspace_id=? AND exploration_id=? AND id=?",
    )
    .bind(context.workspace_id.to_string())
    .bind(context.exploration_id.to_string())
    .bind(context.stage_id.to_string())
    .fetch_optional(c)
    .await
    .map_err(storage::failure)?
    .ok_or("not_found")?;
    if archived {
        Err("stage_archived")
    } else {
        Ok(())
    }
}
pub(super) async fn advance(
    c: &mut SqliteConnection,
    workspace: &str,
    exploration: &str,
) -> Result<()> {
    let changed =
        sqlx::query("UPDATE explorations SET revision=revision+1 WHERE workspace_id=? AND id=?")
            .bind(workspace)
            .bind(exploration)
            .execute(c)
            .await
            .map_err(storage::failure)?;
    if changed.rows_affected() != 1 {
        return Err("not_found");
    }
    Ok(())
}
pub(super) async fn edit(
    c: &mut SqliteConnection,
    context: &Context,
    revision: i64,
    title: &str,
    progress: StageProgress,
    archived: bool,
) -> Result<Value> {
    if revision < 1 || revision == i64::MAX {
        return Err("input_limit");
    }
    let progress = serde_json::to_value(progress).map_err(|_| "invalid_command")?;
    let changed=sqlx::query("UPDATE stages SET title=?,progress=?,archived=?,revision=revision+1 WHERE workspace_id=? AND exploration_id=? AND id=? AND revision=?")
        .bind(mutations::title(title)?).bind(progress.as_str().ok_or("invalid_command")?).bind(archived).bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(context.stage_id.to_string()).bind(revision).execute(&mut *c).await.map_err(storage::failure)?;
    if changed.rows_affected() != 1 {
        return Err("state_conflict");
    }
    advance(
        c,
        &context.workspace_id.to_string(),
        &context.exploration_id.to_string(),
    )
    .await?;
    Ok(json!({"stageId":context.stage_id,"revision":revision+1}))
}
pub(super) async fn reorder(
    c: &mut SqliteConnection,
    workspace: Uuid,
    exploration: Uuid,
    revision: i64,
    ids: &[Uuid],
) -> Result<Value> {
    if revision < 1 || revision == i64::MAX {
        return Err("input_limit");
    }
    let w = workspace.to_string();
    let e = exploration.to_string();
    let current: i64 =
        sqlx::query_scalar("SELECT revision FROM explorations WHERE workspace_id=? AND id=?")
            .bind(&w)
            .bind(&e)
            .fetch_optional(&mut *c)
            .await
            .map_err(storage::failure)?
            .ok_or("not_found")?;
    if current != revision {
        return Err("state_conflict");
    }
    let existing: Vec<String> = sqlx::query_scalar(
        "SELECT id FROM stages WHERE workspace_id=? AND exploration_id=? AND archived=0",
    )
    .bind(&w)
    .bind(&e)
    .fetch_all(&mut *c)
    .await
    .map_err(storage::failure)?;
    let selected = ids
        .iter()
        .map(Uuid::to_string)
        .collect::<std::collections::BTreeSet<_>>();
    if ids.len() != existing.len()
        || selected.len() != ids.len()
        || !existing.iter().all(|id| selected.contains(id))
    {
        return Err("invalid_stage_order");
    }
    for (position, id) in ids.iter().enumerate() {
        sqlx::query(
            "UPDATE stages SET position=? WHERE workspace_id=? AND exploration_id=? AND id=?",
        )
        .bind(position as i64)
        .bind(&w)
        .bind(&e)
        .bind(id.to_string())
        .execute(&mut *c)
        .await
        .map_err(storage::failure)?;
    }
    advance(c, &w, &e).await?;
    Ok(json!({"explorationId":exploration,"revision":revision+1}))
}
