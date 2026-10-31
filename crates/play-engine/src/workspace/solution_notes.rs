//! Selected saved reasoning, independent of later notebook edits or archive state.
use super::{
    model::{Context, Note, NoteRevision, NoteTarget},
    note_targets, notes, solutions, storage,
};
use crate::{Result, protocol};
use serde_json::{Value, json};
use sqlx::SqliteConnection;
use std::collections::BTreeSet;
use uuid::Uuid;

async fn read(
    connection: &mut SqliteConnection,
    context: &Context,
    reference: &NoteRevision,
) -> Result<Note> {
    if reference.revision < 1 {
        return Err("invalid_command");
    }
    let workspace = context.workspace_id.to_string();
    let exploration = context.exploration_id.to_string();
    let id = reference.note_id.to_string();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM notes WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND id=?")
        .bind(&workspace).bind(&exploration).bind(context.stage_id.to_string()).bind(&id).fetch_one(&mut *connection).await.map_err(storage::failure)?;
    if count != 1 {
        return Err("not_found");
    }
    notes::read_revision(
        connection,
        &workspace,
        &exploration,
        &id,
        Some(reference.revision),
    )
    .await
}

pub async fn capture(
    connection: &mut SqliteConnection,
    context: &Context,
    selected: &[NoteRevision],
) -> Result<Vec<Value>> {
    if selected.len() > protocol::contract().retained_block_limit {
        return Err("input_limit");
    }
    let mut seen = BTreeSet::new();
    let mut result = Vec::new();
    for reference in selected {
        if !seen.insert(reference.note_id) {
            return Err("invalid_command");
        }
        let note = read(connection, context, reference).await?;
        result.push(serde_json::to_value(note).map_err(|_| "invalid_state")?);
    }
    Ok(result)
}

pub async fn gaps(
    connection: &mut SqliteConnection,
    context: &Context,
    selected: &Value,
) -> Result<Vec<Value>> {
    let mut result = Vec::new();
    for note in selected.as_array().ok_or("corrupt_store")? {
        let id = note["id"].as_str().ok_or("corrupt_store")?;
        let revision = note["revision"].as_i64().ok_or("corrupt_store")?;
        let reference = NoteRevision {
            note_id: Uuid::parse_str(id).map_err(|_| "corrupt_store")?,
            revision,
        };
        let live = solutions::available(
            read(connection, context, &reference)
                .await
                .and_then(|n| serde_json::to_value(n).map_err(|_| "invalid_state")),
        )?;
        if live["kind"] == "unavailable" {
            result.push(json!({"reference":format!("note:{id}:{revision}"),"source":format!("{} · r{revision}",note["title"].as_str().ok_or("corrupt_store")?),"code":live["code"]}));
        } else if live != *note {
            return Err("corrupt_store");
        }
        if let Some(link) = note.get("target")
            && let Some(code) = target_gap(connection, context, &link["reference"]).await?
        {
            result.push(json!({"reference":format!("note-target:{id}:{revision}"),"source":link["label"],"code":code}));
        }
    }
    Ok(result)
}
async fn target_gap(
    connection: &mut SqliteConnection,
    context: &Context,
    reference: &Value,
) -> Result<Option<Value>> {
    let target: NoteTarget =
        serde_json::from_value(reference.clone()).map_err(|_| "corrupt_store")?;
    let detail = solutions::available(note_targets::resolve(connection, context, &target).await)?;
    if detail["kind"] == "unavailable" {
        return Ok(Some(detail["code"].clone()));
    }
    Ok((detail["evidence"]["kind"] == "unavailable").then(|| detail["evidence"]["code"].clone()))
}
