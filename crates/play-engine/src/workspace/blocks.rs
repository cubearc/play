use super::{
    model::{AssetRevision, BlockSource, Context},
    order, storage,
};
use crate::{Result, protocol};
use serde_json::{Value, json};
use sqlx::{Row, SqliteConnection};
use uuid::Uuid;

pub(super) struct Binding {
    pub position: i64,
    pub archived_revision: Option<i64>,
}
fn reference(id: String, revision: i64) -> Result<AssetRevision> {
    Ok(AssetRevision {
        asset_id: Uuid::parse_str(&id).map_err(|_| "corrupt_store")?,
        revision,
    })
}

pub(super) async fn binding(
    connection: &mut SqliteConnection,
    context: &Context,
    id: Uuid,
    expected: &BlockSource,
) -> Result<Binding> {
    let invalid = match expected {
        BlockSource::Asset { asset, caption } => {
            asset.revision < 1 || caption.as_ref().is_some_and(|c| c.revision < 1)
        }
        BlockSource::Note { revision, .. } => *revision < 1,
    };
    if invalid {
        return Err("input_limit");
    }
    let row=sqlx::query("SELECT b.position,b.kind,b.asset_id,b.asset_revision,b.caption_id,b.caption_revision,b.note_id,b.archived_revision,n.kind AS note_kind,n.current_revision AS note_revision FROM blocks b LEFT JOIN notes n ON (b.workspace_id,b.exploration_id,b.note_id)=(n.workspace_id,n.exploration_id,n.id) WHERE b.workspace_id=? AND b.exploration_id=? AND b.stage_id=? AND b.id=?")
        .bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(context.stage_id.to_string()).bind(id.to_string())
        .fetch_optional(connection).await.map_err(storage::failure)?.ok_or("not_found")?;
    if row.get::<Option<String>, _>("note_kind").as_deref() == Some("reasoning") {
        return Err("invalid_block_type");
    }
    let source = if row.get::<String, _>("kind") == "note" {
        if row.get::<Option<String>, _>("note_kind").as_deref() != Some("annotation") {
            return Err("corrupt_store");
        }
        BlockSource::Note {
            note_id: Uuid::parse_str(row.get("note_id")).map_err(|_| "corrupt_store")?,
            revision: row.get("note_revision"),
        }
    } else {
        let caption = row
            .get::<Option<String>, _>("caption_id")
            .zip(row.get::<Option<i64>, _>("caption_revision"))
            .map(|(id, rev)| reference(id, rev))
            .transpose()?;
        BlockSource::Asset {
            asset: reference(row.get("asset_id"), row.get("asset_revision"))?,
            caption,
        }
    };
    if source != *expected {
        return Err("state_conflict");
    }
    Ok(Binding {
        position: row.get("position"),
        archived_revision: row.get("archived_revision"),
    })
}

async fn capacity(connection: &mut SqliteConnection, context: &Context, new: bool) -> Result<()> {
    let (active,total):(i64,i64)=sqlx::query_as("SELECT count(CASE WHEN archived_revision IS NULL THEN 1 END),count(*) FROM blocks WHERE workspace_id=? AND exploration_id=? AND stage_id=?")
        .bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(context.stage_id.to_string()).fetch_one(connection).await.map_err(storage::failure)?;
    if active >= protocol::contract().block_limit as i64 {
        return Err("storage_quota");
    }
    if new && total >= protocol::contract().retained_block_limit as i64 {
        return Err("retained_block_limit");
    }
    Ok(())
}
pub(super) async fn admit_new(connection: &mut SqliteConnection, context: &Context) -> Result<()> {
    capacity(connection, context, true).await
}

pub(super) async fn source_active(
    connection: &mut SqliteConnection,
    context: &Context,
    id: &str,
) -> Result<()> {
    let active:i64=sqlx::query_scalar("SELECT count(*) FROM blocks WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND archived_revision IS NULL AND (asset_id=? OR caption_id=?)")
        .bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(context.stage_id.to_string()).bind(id).bind(id).fetch_one(connection).await.map_err(storage::failure)?;
    if active == 0 {
        return Err("block_archived");
    }
    Ok(())
}

pub(super) async fn change(
    connection: &mut SqliteConnection,
    context: &Context,
    id: Uuid,
    expected: i64,
    source: &BlockSource,
    archived: bool,
) -> Result<Value> {
    if expected < 1 || expected == i64::MAX {
        return Err("input_limit");
    }
    if order::revision(connection, context).await? != expected {
        return Err("state_conflict");
    }
    let bound = binding(connection, context, id, source).await?;
    if bound.archived_revision.is_some() == archived {
        return Ok(json!({"blockId":id,"revision":expected,"archived":archived}));
    }
    let position = if archived {
        bound.position
    } else {
        capacity(connection, context, false).await?;
        let position = order::append_position(connection, context).await?;
        order::make_space(connection, context, position).await?;
        position
    };
    let revision = order::advance(connection, context, expected).await?;
    let updated=sqlx::query("UPDATE blocks SET archived_revision=?,position=? WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND id=? AND archived_revision IS ?")
        .bind(if archived{Some(revision)}else{None}).bind(position).bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(context.stage_id.to_string()).bind(id.to_string()).bind(bound.archived_revision)
        .execute(&mut *connection).await.map_err(storage::failure)?;
    if updated.rows_affected() != 1 {
        return Err("state_conflict");
    }
    order::normalize(connection, context).await?;
    Ok(json!({"blockId":id,"revision":revision,"archived":archived}))
}
