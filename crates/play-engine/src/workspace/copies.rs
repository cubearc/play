use super::{
    blocks,
    model::{AssetRevision, BlockSource, Content, Context},
    mutations, notes, order, queries, storage,
};
use crate::Result;
use serde_json::{Value, json};
use sqlx::SqliteConnection;
use uuid::Uuid;

pub(super) fn copy_title(title: &str, number: usize) -> String {
    let suffix = if number == 1 {
        " (copy)".into()
    } else {
        format!(" (copy {number})")
    };
    let mut end = title.len().min(120 - suffix.len());
    while !title.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{suffix}", title[..end].trim_end())
}

async fn available_title(
    connection: &mut SqliteConnection,
    context: &Context,
    title: &str,
) -> Result<String> {
    let titles: Vec<String> =
        sqlx::query_scalar("SELECT title FROM assets WHERE workspace_id=? AND exploration_id=?")
            .bind(context.workspace_id.to_string())
            .bind(context.exploration_id.to_string())
            .fetch_all(connection)
            .await
            .map_err(storage::failure)?;
    (1..=titles.len() + 1)
        .map(|number| copy_title(title, number))
        .find(|candidate| !titles.contains(candidate))
        .ok_or("corrupt_store")
}

async fn fork_asset(
    connection: &mut SqliteConnection,
    context: &Context,
    source: &AssetRevision,
) -> Result<String> {
    let workspace = context.workspace_id.to_string();
    let exploration = context.exploration_id.to_string();
    let original = queries::asset(
        connection,
        &workspace,
        &exploration,
        &source.asset_id.to_string(),
        source.revision,
    )
    .await?;
    let title = available_title(connection, context, &original.title).await?;
    let (kind, data) = match original.content {
        Content::Text { text } => ("text", text.into_bytes()),
        Content::Image | Content::File { .. } => {
            let data: Vec<u8> = sqlx::query_scalar("SELECT data FROM asset_revisions WHERE workspace_id=? AND exploration_id=? AND asset_id=? AND revision=?")
                .bind(&workspace).bind(&exploration).bind(source.asset_id.to_string()).bind(source.revision)
                .fetch_one(&mut *connection).await.map_err(storage::failure)?;
            if matches!(original.content, Content::Image) {
                cube_core::image::Image::decode(&data).map_err(|_| "invalid_image")?;
                ("image", data)
            } else {
                ("file", data)
            }
        }
    };
    mutations::bytes(connection, &workspace, &exploration, data.len()).await?;
    let id = mutations::store_asset(connection, context, &title, kind, &data).await?;
    sqlx::query("INSERT INTO asset_origins VALUES(?,?,?,?,?)")
        .bind(&workspace)
        .bind(&exploration)
        .bind(&id)
        .bind(source.asset_id.to_string())
        .bind(source.revision)
        .execute(&mut *connection)
        .await
        .map_err(storage::failure)?;
    let trail = sqlx::query("INSERT INTO trails(workspace_id,exploration_id,stage_id,id,title,hypothesis,source_id,source_revision,pipeline_id,pipeline_revision) SELECT workspace_id,exploration_id,stage_id,?,?,'',?,1,pipeline_id,pipeline_revision FROM trails WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND source_id=? AND source_revision=? ORDER BY rowid LIMIT 1")
        .bind(Uuid::new_v4().to_string()).bind(format!("Explore {title}")).bind(&id)
        .bind(workspace).bind(exploration).bind(context.stage_id.to_string()).bind(source.asset_id.to_string()).bind(source.revision)
        .execute(connection).await.map_err(storage::failure)?;
    if trail.rows_affected() != 1 {
        return Err("corrupt_store");
    }
    Ok(id)
}

pub(super) async fn duplicate(
    connection: &mut SqliteConnection,
    context: &Context,
    block_id: Uuid,
    expected: i64,
    source: &BlockSource,
) -> Result<Value> {
    if expected < 1 || expected == i64::MAX {
        return Err("input_limit");
    }
    if order::revision(connection, context).await? != expected {
        return Err("state_conflict");
    }
    let workspace = context.workspace_id.to_string();
    let exploration = context.exploration_id.to_string();
    let stage = context.stage_id.to_string();
    let bound = blocks::binding(connection, context, block_id, source).await?;
    if bound.archived_revision.is_some() {
        return Err("block_archived");
    }
    blocks::admit_new(connection, context).await?;
    let position = bound.position.checked_add(1).ok_or("corrupt_store")?;
    if let BlockSource::Note { note_id, revision } = source {
        let id = notes::duplicate(connection, context, *note_id, *revision, position).await?;
        let revision = order::advance(connection, context, expected).await?;
        return Ok(json!({"blockId":id,"stageId":context.stage_id,"revision":revision}));
    }
    let BlockSource::Asset { asset, caption } = source else {
        return Err("invalid_block_type");
    };
    let primary = fork_asset(connection, context, asset).await?;
    let caption = if let Some(source) = caption {
        Some(fork_asset(connection, context, source).await?)
    } else {
        None
    };
    order::make_space(connection, context, position).await?;
    let block = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO blocks(workspace_id,exploration_id,stage_id,id,position,kind,asset_id,asset_revision,caption_id,caption_revision) VALUES(?,?,?,?,?,'asset',?,1,?,?)")
        .bind(workspace).bind(exploration).bind(stage).bind(&block).bind(position).bind(primary).bind(&caption).bind(caption.as_ref().map(|_|1i64))
        .execute(&mut *connection).await.map_err(storage::failure)?;
    let revision = order::advance(connection, context, expected).await?;
    Ok(json!({"blockId":block,"stageId":context.stage_id,"revision":revision}))
}
