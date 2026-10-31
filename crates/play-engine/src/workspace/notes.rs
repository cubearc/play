use super::{
    blocks, copies,
    model::{Context, Note, NoteKind, NoteOrigin},
    mutations, order, storage,
};
use crate::{Result, protocol};
use serde_json::{Value, json};
use sqlx::{Row, SqliteConnection};
use uuid::Uuid;

pub(super) async fn read(
    connection: &mut SqliteConnection,
    workspace: &str,
    exploration: &str,
    id: &str,
) -> Result<Note> {
    read_revision(connection, workspace, exploration, id, None).await
}

pub(super) async fn read_revision(
    connection: &mut SqliteConnection,
    workspace: &str,
    exploration: &str,
    id: &str,
    revision: Option<i64>,
) -> Result<Note> {
    let row=sqlx::query("SELECT n.id,n.kind,n.title,r.revision AS current_revision,r.text,n.source_note_id,n.source_revision,p.title AS source_title FROM notes n JOIN note_revisions r ON (n.workspace_id,n.exploration_id,n.id)=(r.workspace_id,r.exploration_id,r.note_id) AND r.revision=COALESCE(?,n.current_revision) LEFT JOIN notes p ON (n.workspace_id,n.exploration_id,n.source_note_id)=(p.workspace_id,p.exploration_id,p.id) WHERE n.workspace_id=? AND n.exploration_id=? AND n.id=?")
        .bind(revision).bind(workspace).bind(exploration).bind(id).fetch_optional(&mut *connection).await.map_err(storage::failure)?.ok_or("not_found")?;
    let kind = match row.get::<&str, _>("kind") {
        "reasoning" => NoteKind::Reasoning,
        "annotation" => NoteKind::Annotation,
        _ => return Err("corrupt_store"),
    };
    let text: String = row.get("text");
    if text.len() > protocol::contract().note_bytes {
        return Err("corrupt_store");
    }
    let origin = if let Some(id) = row.get::<Option<String>, _>("source_note_id") {
        Some(NoteOrigin {
            note_id: id,
            revision: row
                .get::<Option<i64>, _>("source_revision")
                .ok_or("corrupt_store")?,
            title: row
                .get::<Option<String>, _>("source_title")
                .ok_or("corrupt_store")?,
        })
    } else {
        None
    };
    Ok(Note {
        id: row.get("id"),
        kind,
        title: row.get("title"),
        revision: row.get("current_revision"),
        text,
        origin,
        target: super::note_targets::read(connection, workspace, exploration, id).await?,
    })
}

pub(super) async fn save(
    connection: &mut SqliteConnection,
    context: &Context,
    note: Uuid,
    expected: i64,
    text: &str,
) -> Result<Value> {
    if text.len() > protocol::contract().note_bytes || expected < 1 || expected == i64::MAX {
        return Err("input_limit");
    }
    let workspace = context.workspace_id.to_string();
    let exploration = context.exploration_id.to_string();
    let archived:Option<Option<i64>>=sqlx::query_scalar("SELECT archived_revision FROM blocks WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND note_id=? AND kind='note'").bind(&workspace).bind(&exploration).bind(context.stage_id.to_string()).bind(note.to_string()).fetch_optional(&mut *connection).await.map_err(storage::failure)?;
    match archived {
        Some(Some(_)) => return Err("block_archived"),
        None => return Err("state_conflict"),
        Some(None) => {}
    }
    mutations::bytes(connection, &workspace, &exploration, text.len()).await?;
    let updated=sqlx::query("UPDATE notes SET current_revision=current_revision+1 WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND id=? AND current_revision=?").bind(&workspace).bind(&exploration).bind(context.stage_id.to_string()).bind(note.to_string()).bind(expected).execute(&mut *connection).await.map_err(storage::failure)?;
    if updated.rows_affected() != 1 {
        return Err("state_conflict");
    }
    sqlx::query("INSERT INTO note_revisions VALUES(?,?,?,?,?)")
        .bind(workspace)
        .bind(exploration)
        .bind(note.to_string())
        .bind(expected + 1)
        .bind(text)
        .execute(connection)
        .await
        .map_err(storage::failure)?;
    Ok(json!({"noteId":note,"revision":expected+1}))
}

async fn store(
    connection: &mut SqliteConnection,
    context: &Context,
    title: &str,
    text: &str,
    source: Option<(&str, i64)>,
) -> Result<String> {
    mutations::title(title)?;
    if text.len() > protocol::contract().note_bytes {
        return Err("input_limit");
    }
    let workspace = context.workspace_id.to_string();
    let exploration = context.exploration_id.to_string();
    let id = Uuid::new_v4().to_string();
    mutations::bytes(connection, &workspace, &exploration, text.len()).await?;
    sqlx::query("INSERT INTO notes VALUES(?,?,?,?,1,'annotation',?,?,?)")
        .bind(&workspace)
        .bind(&exploration)
        .bind(context.stage_id.to_string())
        .bind(&id)
        .bind(title.trim())
        .bind(source.map(|(id, _)| id))
        .bind(source.map(|(_, revision)| revision))
        .execute(&mut *connection)
        .await
        .map_err(storage::failure)?;
    sqlx::query("INSERT INTO note_revisions VALUES(?,?,?,1,?)")
        .bind(workspace)
        .bind(exploration)
        .bind(&id)
        .bind(text)
        .execute(connection)
        .await
        .map_err(storage::failure)?;
    Ok(id)
}
async fn block(
    connection: &mut SqliteConnection,
    context: &Context,
    note: &str,
    position: i64,
) -> Result<String> {
    order::make_space(connection, context, position).await?;
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO blocks(workspace_id,exploration_id,stage_id,id,position,kind,note_id) VALUES(?,?,?,?,?,'note',?)").bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(context.stage_id.to_string()).bind(&id).bind(position).bind(note).execute(connection).await.map_err(storage::failure)?;
    Ok(id)
}
pub(super) async fn add(
    connection: &mut SqliteConnection,
    context: &Context,
    title: &str,
    text: &str,
) -> Result<Value> {
    blocks::admit_new(connection, context).await?;
    let note = store(connection, context, title, text, None).await?;
    let position = order::append_position(connection, context).await?;
    let id = block(connection, context, &note, position).await?;
    let expected = order::revision(connection, context).await?;
    order::advance(connection, context, expected).await?;
    Ok(json!({"blockId":id}))
}
pub(super) async fn duplicate(
    connection: &mut SqliteConnection,
    context: &Context,
    note: Uuid,
    revision: i64,
    position: i64,
) -> Result<String> {
    let original = read(
        connection,
        &context.workspace_id.to_string(),
        &context.exploration_id.to_string(),
        &note.to_string(),
    )
    .await?;
    if original.kind != NoteKind::Annotation {
        return Err("invalid_block_type");
    }
    if original.revision != revision {
        return Err("state_conflict");
    }
    let titles: Vec<String> = sqlx::query_scalar(
        "SELECT title FROM notes WHERE workspace_id=? AND exploration_id=? AND stage_id=?",
    )
    .bind(context.workspace_id.to_string())
    .bind(context.exploration_id.to_string())
    .bind(context.stage_id.to_string())
    .fetch_all(&mut *connection)
    .await
    .map_err(storage::failure)?;
    let title = (1..=titles.len() + 1)
        .map(|n| copies::copy_title(&original.title, n))
        .find(|candidate| !titles.contains(candidate))
        .ok_or("corrupt_store")?;
    let copy = store(
        connection,
        context,
        &title,
        &original.text,
        Some((&original.id, revision)),
    )
    .await?;
    super::note_targets::copy(connection, context, &original.id, &copy).await?;
    block(connection, context, &copy, position).await
}
