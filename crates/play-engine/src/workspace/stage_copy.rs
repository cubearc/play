//! A Stage copy starts a fresh investigation path. Selected answers stay in the source.
use super::{model::Context, mutations, notes, queries, storage};
use crate::Result;
use serde_json::{Value, json};
use sqlx::{Row, SqliteConnection, sqlite::SqliteRow};
use uuid::Uuid;

pub(super) async fn duplicate(
    c: &mut SqliteConnection,
    source: &Context,
    revision: i64,
    title: Option<&str>,
) -> Result<Value> {
    let w = source.workspace_id.to_string();
    let e = source.exploration_id.to_string();
    let current: i64 = sqlx::query_scalar(
        "SELECT revision FROM stages WHERE workspace_id=? AND exploration_id=? AND id=?",
    )
    .bind(&w)
    .bind(&e)
    .bind(source.stage_id.to_string())
    .fetch_one(&mut *c)
    .await
    .map_err(storage::failure)?;
    if current != revision {
        return Err("state_conflict");
    }
    let title = match title {
        Some(title) => title.to_owned(),
        None => {
            let titles: Vec<String> = sqlx::query_scalar(
                "SELECT title FROM stages WHERE workspace_id=? AND exploration_id=?",
            )
            .bind(&w)
            .bind(&e)
            .fetch_all(&mut *c)
            .await
            .map_err(storage::failure)?;
            let source_title: String = sqlx::query_scalar(
                "SELECT title FROM stages WHERE workspace_id=? AND exploration_id=? AND id=?",
            )
            .bind(&w)
            .bind(&e)
            .bind(source.stage_id.to_string())
            .fetch_one(&mut *c)
            .await
            .map_err(storage::failure)?;
            (1..=titles.len() + 1)
                .map(|n| super::copies::copy_title(&source_title, n))
                .find(|name| !titles.contains(name))
                .ok_or("corrupt_store")?
        }
    };
    let rows=sqlx::query("SELECT b.*,n.kind AS note_kind FROM blocks b LEFT JOIN notes n ON (b.workspace_id,b.exploration_id,b.note_id)=(n.workspace_id,n.exploration_id,n.id) WHERE b.workspace_id=? AND b.exploration_id=? AND b.stage_id=? AND b.archived_revision IS NULL ORDER BY b.position,b.id").bind(&w).bind(&e).bind(source.stage_id.to_string()).fetch_all(&mut *c).await.map_err(storage::failure)?;
    let reasoning = rows
        .iter()
        .find(|r| r.get::<Option<&str>, _>("note_kind") == Some("reasoning"))
        .ok_or("corrupt_store")?;
    let reasoning = notes::read(c, &w, &e, reasoning.get("note_id")).await?;
    let id = mutations::stage(c, &w, &e, &title, Some(&reasoning)).await?;
    let target = Context {
        workspace_id: source.workspace_id,
        exploration_id: source.exploration_id,
        stage_id: Uuid::parse_str(&id).map_err(|_| "corrupt_store")?,
    };
    let mut order = Vec::new();
    for row in rows {
        let copied = if row.get::<&str, _>("kind") == "note" {
            copy_note(c, &target, &row).await?
        } else {
            copy_asset(c, &target, &row).await?
        };
        order.push(Uuid::parse_str(&copied).map_err(|_| "corrupt_store")?);
    }
    let revision = super::order::revision(c, &target).await?;
    super::order::reorder(c, &target, revision, &order).await?;
    Ok(json!({"explorationId":source.exploration_id,"stageId":id}))
}
async fn copy_note(c: &mut SqliteConnection, target: &Context, row: &SqliteRow) -> Result<String> {
    let w = target.workspace_id.to_string();
    let e = target.exploration_id.to_string();
    let original = notes::read(c, &w, &e, row.get("note_id")).await?;
    if original.kind == super::model::NoteKind::Reasoning {
        sqlx::query_scalar("SELECT b.id FROM blocks b JOIN notes n ON (b.workspace_id,b.exploration_id,b.note_id)=(n.workspace_id,n.exploration_id,n.id) WHERE b.workspace_id=? AND b.exploration_id=? AND b.stage_id=? AND n.kind='reasoning'").bind(&w).bind(&e).bind(target.stage_id.to_string()).fetch_one(c).await.map_err(storage::failure)
    } else {
        Ok(notes::add(
            c,
            target,
            &original.title,
            &original.text,
            Some((&original.id, original.revision)),
        )
        .await?["blockId"]
            .as_str()
            .ok_or("invalid_state")?
            .to_owned())
    }
}
async fn copy_asset(c: &mut SqliteConnection, target: &Context, row: &SqliteRow) -> Result<String> {
    let w = target.workspace_id.to_string();
    let e = target.exploration_id.to_string();
    let source: &str = row.get("asset_id");
    let revision: i64 = row.get("asset_revision");
    let original = queries::asset(c, &w, &e, source, revision).await?;
    let (kind,data):(String,Vec<u8>)=sqlx::query_as("SELECT a.kind,r.data FROM assets a JOIN asset_revisions r ON (a.workspace_id,a.exploration_id,a.id)=(r.workspace_id,r.exploration_id,r.asset_id) WHERE a.workspace_id=? AND a.exploration_id=? AND a.id=? AND r.revision=?").bind(&w).bind(&e).bind(source).bind(revision).fetch_one(&mut *c).await.map_err(storage::failure)?;
    let caption = if let Some(id) = row.get::<Option<&str>, _>("caption_id") {
        let caption = queries::asset(c, &w, &e, id, row.get("caption_revision")).await?;
        let super::model::Content::Text { text } = caption.content else {
            return Err("corrupt_store");
        };
        Some(text)
    } else {
        None
    };
    let added =
        mutations::add_block(c, target, &original.title, &kind, &data, caption.as_deref()).await?;
    origin(c, &w, &e, &added.asset_id, source, revision).await?;
    if let Some(source) = row.get::<Option<&str>, _>("caption_id") {
        let copied: String = sqlx::query_scalar(
            "SELECT caption_id FROM blocks WHERE workspace_id=? AND exploration_id=? AND id=?",
        )
        .bind(&w)
        .bind(&e)
        .bind(&added.id)
        .fetch_one(&mut *c)
        .await
        .map_err(storage::failure)?;
        origin(c, &w, &e, &copied, source, row.get("caption_revision")).await?;
    }
    Ok(added.id)
}
async fn origin(
    c: &mut SqliteConnection,
    w: &str,
    e: &str,
    id: &str,
    source: &str,
    revision: i64,
) -> Result<()> {
    sqlx::query("INSERT INTO asset_origins VALUES(?,?,?,?,?)")
        .bind(w)
        .bind(e)
        .bind(id)
        .bind(source)
        .bind(revision)
        .execute(c)
        .await
        .map_err(storage::failure)?;
    Ok(())
}
