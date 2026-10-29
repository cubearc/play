use super::{model::Context, storage};
use crate::{Result, protocol};
use serde_json::{Value, json};
use sqlx::{Row, SqliteConnection};
use std::collections::BTreeSet;
use uuid::Uuid;

pub(super) async fn revision(connection: &mut SqliteConnection, context: &Context) -> Result<i64> {
    sqlx::query_scalar(
        "SELECT revision FROM stages WHERE workspace_id=? AND exploration_id=? AND id=?",
    )
    .bind(context.workspace_id.to_string())
    .bind(context.exploration_id.to_string())
    .bind(context.stage_id.to_string())
    .fetch_optional(connection)
    .await
    .map_err(storage::failure)?
    .ok_or("not_found")
}
pub(super) async fn advance(
    connection: &mut SqliteConnection,
    context: &Context,
    expected: i64,
) -> Result<i64> {
    let next = expected
        .checked_add(1)
        .filter(|_| expected > 0)
        .ok_or("input_limit")?;
    let result = sqlx::query("UPDATE stages SET revision=? WHERE workspace_id=? AND exploration_id=? AND id=? AND revision=?")
        .bind(next).bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(context.stage_id.to_string()).bind(expected)
        .execute(connection).await.map_err(storage::failure)?;
    if result.rows_affected() != 1 {
        return Err("state_conflict");
    }
    Ok(next)
}
pub(super) async fn append_position(
    connection: &mut SqliteConnection,
    context: &Context,
) -> Result<i64> {
    let row = sqlx::query("SELECT position,kind FROM blocks WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND archived_revision IS NULL ORDER BY position DESC,id DESC LIMIT 1")
        .bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(context.stage_id.to_string())
        .fetch_one(connection).await.map_err(storage::failure)?;
    let position: i64 = row.get("position");
    if row.get::<String, _>("kind") == "note" {
        Ok(position)
    } else {
        position.checked_add(1).ok_or("corrupt_store")
    }
}
pub(super) async fn make_space(
    connection: &mut SqliteConnection,
    context: &Context,
    position: i64,
) -> Result<()> {
    sqlx::query("UPDATE blocks SET position=position+1 WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND archived_revision IS NULL AND position>=?")
        .bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(context.stage_id.to_string()).bind(position).execute(connection).await.map_err(storage::failure)?;
    Ok(())
}
async fn ids(connection: &mut SqliteConnection, context: &Context) -> Result<Vec<String>> {
    sqlx::query_scalar("SELECT id FROM blocks WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND archived_revision IS NULL ORDER BY position,id")
        .bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(context.stage_id.to_string()).fetch_all(connection).await.map_err(storage::failure)
}
async fn assign(
    connection: &mut SqliteConnection,
    context: &Context,
    ids: &[String],
) -> Result<()> {
    for (position, id) in ids.iter().enumerate() {
        let updated=sqlx::query("UPDATE blocks SET position=? WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND id=? AND archived_revision IS NULL")
            .bind(position as i64).bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(context.stage_id.to_string()).bind(id).execute(&mut *connection).await.map_err(storage::failure)?;
        if updated.rows_affected() != 1 {
            return Err("state_conflict");
        }
    }
    Ok(())
}
pub(super) async fn normalize(connection: &mut SqliteConnection, context: &Context) -> Result<()> {
    let current = ids(connection, context).await?;
    assign(connection, context, &current).await
}
pub(super) async fn reorder(
    connection: &mut SqliteConnection,
    context: &Context,
    expected: i64,
    requested: &[Uuid],
) -> Result<Value> {
    if expected < 1 || expected == i64::MAX || requested.len() > protocol::contract().block_limit {
        return Err("input_limit");
    }
    if revision(connection, context).await? != expected {
        return Err("state_conflict");
    }
    let current = ids(connection, context).await?;
    let requested: Vec<String> = requested.iter().map(Uuid::to_string).collect();
    if requested.len() != current.len()
        || requested.iter().collect::<BTreeSet<_>>() != current.iter().collect::<BTreeSet<_>>()
    {
        return Err("invalid_order");
    }
    if requested == current {
        return Ok(json!({"stageId":context.stage_id,"revision":expected}));
    }
    let next = advance(connection, context, expected).await?;
    assign(connection, context, &requested).await?;
    Ok(json!({"stageId":context.stage_id,"revision":next}))
}
