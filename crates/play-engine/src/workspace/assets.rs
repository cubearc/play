use super::{
    model::{Content, Context},
    mutations, queries, storage,
};
use crate::{Result, protocol};
use serde_json::{Value, json};
use sqlx::{Row, SqliteConnection};
use uuid::Uuid;

pub(super) async fn revise_text(
    connection: &mut SqliteConnection,
    context: &Context,
    block: Uuid,
    asset: Uuid,
    expected: i64,
    text: &str,
) -> Result<Value> {
    if expected < 1 || expected == i64::MAX || text.len() > protocol::contract().text_bytes {
        return Err("input_limit");
    }
    let workspace = context.workspace_id.to_string();
    let exploration = context.exploration_id.to_string();
    let stage = context.stage_id.to_string();
    let asset = asset.to_string();
    let row = sqlx::query("SELECT asset_id,asset_revision,caption_id,caption_revision,archived_revision FROM blocks WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND id=? AND kind='asset'")
        .bind(&workspace).bind(&exploration).bind(&stage).bind(block.to_string())
        .fetch_optional(&mut *connection).await.map_err(storage::failure)?.ok_or("not_found")?;
    if row.get::<Option<i64>, _>("archived_revision").is_some() {
        return Err("block_archived");
    }
    let primary = row.get::<String, _>("asset_id") == asset;
    let revision = if primary {
        row.get::<i64, _>("asset_revision")
    } else if row.get::<Option<String>, _>("caption_id").as_deref() == Some(&asset) {
        row.get::<i64, _>("caption_revision")
    } else {
        return Err("not_found");
    };
    let latest: i64 = sqlx::query_scalar("SELECT MAX(revision) FROM asset_revisions WHERE workspace_id=? AND exploration_id=? AND asset_id=?")
        .bind(&workspace).bind(&exploration).bind(&asset).fetch_one(&mut *connection).await.map_err(storage::failure)?;
    if revision != expected || latest != expected {
        return Err("state_conflict");
    }
    let original = queries::asset(connection, &workspace, &exploration, &asset, expected).await?;
    let Content::Text { text: previous } = original.content else {
        return Err("invalid_asset_type");
    };
    if previous == text {
        return Ok(json!({"assetId":asset,"revision":expected}));
    }
    mutations::bytes(connection, &workspace, &exploration, text.len()).await?;
    let next = expected + 1;
    sqlx::query("INSERT INTO asset_revisions VALUES(?,?,?,?,?,?)")
        .bind(&workspace)
        .bind(&exploration)
        .bind(&asset)
        .bind(next)
        .bind(text.as_bytes())
        .bind(cube_core::hashing::sha256(text.as_bytes()))
        .execute(&mut *connection)
        .await
        .map_err(storage::failure)?;
    let sql = if primary {
        "UPDATE blocks SET asset_revision=? WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND id=? AND asset_id=? AND asset_revision=?"
    } else {
        "UPDATE blocks SET caption_revision=? WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND id=? AND caption_id=? AND caption_revision=?"
    };
    let updated = sqlx::query(sql)
        .bind(next)
        .bind(&workspace)
        .bind(&exploration)
        .bind(&stage)
        .bind(block.to_string())
        .bind(&asset)
        .bind(expected)
        .execute(&mut *connection)
        .await
        .map_err(storage::failure)?;
    if updated.rows_affected() != 1 {
        return Err("state_conflict");
    }
    sqlx::query("UPDATE trails SET source_revision=? WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND source_id=? AND source_revision=?")
        .bind(next).bind(workspace).bind(exploration).bind(stage).bind(&asset).bind(expected)
        .execute(connection).await.map_err(storage::failure)?;
    Ok(json!({"assetId":asset,"revision":next}))
}
