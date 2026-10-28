use super::{
    model::{Command, Context},
    queries,
    storage::{self, Owned},
};
use crate::{Result, protocol};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use sqlx::{Connection, Row, SqliteConnection};
use uuid::Uuid;

pub fn title(value: &str) -> Result<&str> {
    let value = value.trim();
    if value.is_empty() || value.len() > 120 {
        return Err("invalid_title");
    }
    Ok(value)
}

async fn count(
    connection: &mut SqliteConnection,
    sql: &'static str,
    workspace: &str,
    exploration: Option<&str>,
    stage: Option<&str>,
    maximum: usize,
) -> Result<()> {
    let mut query = sqlx::query_scalar::<_, i64>(sql).bind(workspace);
    if let Some(id) = exploration {
        query = query.bind(id);
    }
    if let Some(id) = stage {
        query = query.bind(id);
    }
    if query
        .fetch_one(connection)
        .await
        .map_err(storage::failure)?
        >= maximum as i64
    {
        return Err("storage_quota");
    }
    Ok(())
}

async fn bytes(
    connection: &mut SqliteConnection,
    workspace: &str,
    exploration: &str,
    additional: usize,
) -> Result<()> {
    let total: i64 = sqlx::query_scalar("SELECT (SELECT COALESCE(SUM(length(data)),0) FROM asset_revisions WHERE workspace_id=? AND exploration_id=?)+(SELECT COALESCE(SUM(length(CAST(text AS BLOB))),0) FROM note_revisions WHERE workspace_id=? AND exploration_id=?)")
        .bind(workspace).bind(exploration).bind(workspace).bind(exploration).fetch_one(connection).await.map_err(storage::failure)?;
    if total < 0 || total as usize + additional > protocol::contract().exploration_bytes {
        return Err("storage_quota");
    }
    Ok(())
}

async fn stage(
    connection: &mut SqliteConnection,
    workspace: &str,
    exploration: &str,
    name: &str,
) -> Result<String> {
    count(
        connection,
        "SELECT count(*) FROM stages WHERE workspace_id=? AND exploration_id=?",
        workspace,
        Some(exploration),
        None,
        protocol::contract().stage_limit,
    )
    .await?;
    let id = Uuid::new_v4().to_string();
    let note = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO stages(workspace_id,exploration_id,id,title,revision,position) VALUES(?,?,?,?,1,(SELECT count(*) FROM stages WHERE workspace_id=? AND exploration_id=?))")
        .bind(workspace).bind(exploration).bind(&id).bind(title(name)?).bind(workspace).bind(exploration).execute(&mut *connection).await.map_err(storage::failure)?;
    sqlx::query("INSERT INTO notes VALUES(?,?,?,?,1)")
        .bind(workspace)
        .bind(exploration)
        .bind(&id)
        .bind(&note)
        .execute(&mut *connection)
        .await
        .map_err(storage::failure)?;
    sqlx::query("INSERT INTO note_revisions VALUES(?,?,?,1,'')")
        .bind(workspace)
        .bind(exploration)
        .bind(&note)
        .execute(&mut *connection)
        .await
        .map_err(storage::failure)?;
    sqlx::query("INSERT INTO blocks(workspace_id,exploration_id,stage_id,id,position,kind,note_id) VALUES(?,?,?,?,0,'note',?)")
        .bind(workspace).bind(exploration).bind(&id).bind(Uuid::new_v4().to_string()).bind(note).execute(&mut *connection).await.map_err(storage::failure)?;
    queries::select(connection, workspace, exploration, &id).await?;
    Ok(id)
}

async fn new_exploration(
    connection: &mut SqliteConnection,
    workspace: &str,
    name: &str,
) -> Result<Value> {
    count(
        connection,
        "SELECT count(*) FROM explorations WHERE workspace_id=?",
        workspace,
        None,
        None,
        protocol::contract().exploration_limit,
    )
    .await?;
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO explorations VALUES(?,?,?,1)")
        .bind(workspace)
        .bind(&id)
        .bind(title(name)?)
        .execute(&mut *connection)
        .await
        .map_err(storage::failure)?;
    let stage = stage(connection, workspace, &id, "Stage 1").await?;
    Ok(json!({"explorationId":id,"stageId":stage}))
}

async fn save_note(
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
    bytes(connection, &workspace, &exploration, text.len()).await?;
    let updated = sqlx::query("UPDATE notes SET current_revision=current_revision+1 WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND id=? AND current_revision=?")
        .bind(&workspace).bind(&exploration).bind(context.stage_id.to_string()).bind(note.to_string()).bind(expected).execute(&mut *connection).await.map_err(storage::failure)?;
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

async fn pipeline(connection: &mut SqliteConnection, workspace: &str) -> Result<String> {
    let existing: Option<String> =
        sqlx::query_scalar("SELECT id FROM pipelines WHERE workspace_id=? ORDER BY rowid LIMIT 1")
            .bind(workspace)
            .fetch_optional(&mut *connection)
            .await
            .map_err(storage::failure)?;
    if let Some(id) = existing {
        return Ok(id);
    }
    let descriptor =
        cube_core::operations::require("cipher.rot13").map_err(|_| "unsupported_operation")?;
    let contract = descriptor.contract().map_err(|_| "contract_mismatch")?;
    let id = Uuid::new_v4().to_string();
    let definition = json!({"formatVersion":1,"inputs":[{"name":"source","semanticType":descriptor.input_type}],"nodes":[{"id":Uuid::new_v4(),"operation":descriptor.id,"operation_version":contract.operation_version,"input":"source"}]}).to_string();
    sqlx::query("INSERT INTO pipelines VALUES(?,?,'Text transform',1)")
        .bind(workspace)
        .bind(&id)
        .execute(&mut *connection)
        .await
        .map_err(storage::failure)?;
    sqlx::query("INSERT INTO pipeline_revisions VALUES(?,?,1,?)")
        .bind(workspace)
        .bind(&id)
        .bind(definition)
        .execute(connection)
        .await
        .map_err(storage::failure)?;
    Ok(id)
}

async fn asset(
    connection: &mut SqliteConnection,
    context: &Context,
    name: &str,
    kind: &str,
    data: &[u8],
) -> Result<String> {
    let workspace = context.workspace_id.to_string();
    let exploration = context.exploration_id.to_string();
    let id = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO assets VALUES(?,?,?,?,?)")
        .bind(&workspace)
        .bind(&exploration)
        .bind(&id)
        .bind(title(name)?)
        .bind(kind)
        .execute(&mut *connection)
        .await
        .map_err(storage::failure)?;
    sqlx::query("INSERT INTO asset_revisions VALUES(?,?,?,1,?,?)")
        .bind(&workspace)
        .bind(&exploration)
        .bind(&id)
        .bind(data)
        .bind(cube_core::hashing::sha256(data))
        .execute(&mut *connection)
        .await
        .map_err(storage::failure)?;
    let pipeline_id = if kind == "text" {
        Some(pipeline(connection, &workspace).await?)
    } else {
        None
    };
    sqlx::query("INSERT INTO trails VALUES(?,?,?,?,?,'',?,1,?,?)")
        .bind(workspace)
        .bind(exploration)
        .bind(context.stage_id.to_string())
        .bind(Uuid::new_v4().to_string())
        .bind(format!("Explore {}", title(name)?))
        .bind(&id)
        .bind(&pipeline_id)
        .bind(pipeline_id.as_ref().map(|_| 1i64))
        .execute(connection)
        .await
        .map_err(storage::failure)?;
    Ok(id)
}

async fn add(
    connection: &mut SqliteConnection,
    context: &Context,
    name: &str,
    text: &str,
    encoded: Option<&str>,
) -> Result<Value> {
    if text.len() > protocol::contract().text_bytes {
        return Err("input_limit");
    }
    let workspace = context.workspace_id.to_string();
    let exploration = context.exploration_id.to_string();
    let stage = context.stage_id.to_string();
    count(
        connection,
        "SELECT count(*) FROM blocks WHERE workspace_id=? AND exploration_id=? AND stage_id=?",
        &workspace,
        Some(&exploration),
        Some(&stage),
        protocol::contract().block_limit,
    )
    .await?;
    let (kind, data) = if let Some(encoded) = encoded {
        if encoded.len() > protocol::contract().image_bytes.div_ceil(3) * 4 {
            return Err("input_limit");
        }
        let bytes = STANDARD.decode(encoded).map_err(|_| "invalid_image")?;
        if bytes.len() > protocol::contract().image_bytes
            || !bytes.starts_with(b"\x89PNG\r\n\x1a\n")
        {
            return Err("invalid_image");
        }
        cube_core::image::Image::decode(&bytes).map_err(|_| "invalid_image")?;
        ("image", bytes)
    } else {
        ("text", text.as_bytes().to_vec())
    };
    bytes(
        connection,
        &workspace,
        &exploration,
        data.len() + if encoded.is_some() { text.len() } else { 0 },
    )
    .await?;
    let primary = asset(connection, context, name, kind, &data).await?;
    let caption = if encoded.is_some() && !text.is_empty() {
        Some(
            asset(
                connection,
                context,
                "Image caption",
                "text",
                text.as_bytes(),
            )
            .await?,
        )
    } else {
        None
    };
    let position:i64=sqlx::query_scalar("SELECT count(*) FROM blocks WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND kind='asset'").bind(&workspace).bind(&exploration).bind(&stage).fetch_one(&mut *connection).await.map_err(storage::failure)?;
    sqlx::query("UPDATE blocks SET position=position+1 WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND position>=?").bind(&workspace).bind(&exploration).bind(&stage).bind(position).execute(&mut *connection).await.map_err(storage::failure)?;
    let block = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO blocks(workspace_id,exploration_id,stage_id,id,position,kind,asset_id,asset_revision,caption_id,caption_revision) VALUES(?,?,?,?,?,'asset',?,1,?,?)")
        .bind(workspace).bind(exploration).bind(stage).bind(&block).bind(position).bind(primary).bind(&caption).bind(caption.as_ref().map(|_|1i64)).execute(connection).await.map_err(storage::failure)?;
    Ok(json!({"blockId":block}))
}

pub async fn apply(store: &mut Owned, command: &Command) -> Result<Value> {
    let key = command.mutation_key().ok_or("invalid_command")?.to_string();
    let serialized = serde_json::to_vec(command).map_err(|_| "invalid_command")?;
    let digest = cube_core::hashing::sha256(&serialized);
    let value = serde_json::to_value(command).map_err(|_| "invalid_command")?;
    let kind = value["kind"].as_str().ok_or("invalid_command")?;
    let owner = store.identity.workspace_id;
    let workspace = owner.to_string();
    let mut transaction = store
        .connection()?
        .begin()
        .await
        .map_err(storage::failure)?;
    let connection = &mut *transaction;
    let previous = sqlx::query(
        "SELECT digest,result FROM receipts WHERE workspace_id=? AND command=? AND key=?",
    )
    .bind(&workspace)
    .bind(kind)
    .bind(&key)
    .fetch_optional(&mut *connection)
    .await
    .map_err(storage::failure)?;
    if let Some(previous) = previous {
        if previous.get::<String, _>(0) != digest {
            return Err("idempotency_conflict");
        }
        return serde_json::from_str(previous.get::<&str, _>(1)).map_err(|_| "corrupt_store");
    }
    let result = match command {
        Command::CreateExploration {
            workspace_id,
            title,
            ..
        } => {
            if *workspace_id != owner {
                return Err("workspace_mismatch");
            }
            new_exploration(connection, &workspace, title).await?
        }
        Command::CreateStage {
            workspace_id,
            exploration_id,
            title,
            ..
        } => {
            if *workspace_id != owner {
                return Err("workspace_mismatch");
            }
            let exists: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM explorations WHERE workspace_id=? AND id=?",
            )
            .bind(&workspace)
            .bind(exploration_id.to_string())
            .fetch_one(&mut *connection)
            .await
            .map_err(storage::failure)?;
            if exists != 1 {
                return Err("not_found");
            }
            json!({"explorationId":exploration_id,"stageId":stage(connection,&workspace,&exploration_id.to_string(),title).await?})
        }
        Command::SaveNote {
            context,
            key: _,
            note_id,
            expected_revision,
            text,
        } => {
            queries::context(connection, owner, context).await?;
            save_note(connection, context, *note_id, *expected_revision, text).await?
        }
        Command::AddText {
            context,
            title,
            text,
            ..
        } => {
            queries::context(connection, owner, context).await?;
            add(connection, context, title, text, None).await?
        }
        Command::AddImage {
            context,
            title,
            caption,
            png,
            ..
        } => {
            queries::context(connection, owner, context).await?;
            add(connection, context, title, caption, Some(png)).await?
        }
        _ => return Err("invalid_command"),
    };
    sqlx::query("INSERT INTO receipts VALUES(?,?,?,?,?)")
        .bind(workspace)
        .bind(kind)
        .bind(key)
        .bind(digest)
        .bind(result.to_string())
        .execute(&mut *transaction)
        .await
        .map_err(storage::failure)?;
    transaction.commit().await.map_err(storage::failure)?;
    Ok(result)
}
