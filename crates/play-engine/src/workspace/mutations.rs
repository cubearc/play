use super::{
    assets, blocks, copies, files,
    model::{Command, Context},
    notes, order, queries, results,
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

pub(super) async fn bytes(
    connection: &mut SqliteConnection,
    workspace: &str,
    exploration: &str,
    additional: usize,
) -> Result<()> {
    let total: i64 = sqlx::query_scalar("SELECT (SELECT COALESCE(SUM(length(data)),0) FROM asset_revisions WHERE workspace_id=? AND exploration_id=?)+(SELECT COALESCE(SUM(length(CAST(text AS BLOB))),0) FROM note_revisions WHERE workspace_id=? AND exploration_id=?)+(SELECT COALESCE(SUM(length(CAST(plan AS BLOB))+COALESCE(length(CAST(result AS BLOB)),0)+length(reservation)),0) FROM pipeline_runs WHERE workspace_id=? AND exploration_id=?)+(SELECT COALESCE(SUM(length(CAST(hypothesis AS BLOB))),0) FROM trails WHERE workspace_id=? AND exploration_id=?)+(SELECT COALESCE(SUM(length(CAST(result AS BLOB))),0) FROM node_results WHERE workspace_id=? AND exploration_id=?)+(SELECT COALESCE(SUM(length(CAST(title AS BLOB))+length(CAST(explanation AS BLOB))+length(CAST(snapshot AS BLOB))),0) FROM solution_revisions WHERE workspace_id=? AND exploration_id=?)")
        .bind(workspace).bind(exploration).bind(workspace).bind(exploration).bind(workspace).bind(exploration).bind(workspace).bind(exploration).bind(workspace).bind(exploration).bind(workspace).bind(exploration).fetch_one(connection).await.map_err(storage::failure)?;
    if total < 0 || total as usize + additional > protocol::contract().exploration_bytes {
        return Err("storage_quota");
    }
    Ok(())
}

pub(super) async fn stage(
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
    sqlx::query("INSERT INTO notes VALUES(?,?,?,?,1,'reasoning','Reasoning',NULL,NULL)")
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

pub(super) async fn new_exploration(
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

pub(super) async fn pipeline(
    connection: &mut SqliteConnection,
    workspace: &str,
    operation: &str,
) -> Result<super::model::PipelineRef> {
    super::pipelines::default_for(connection, workspace, operation).await
}

pub(super) async fn store_asset(
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
    Ok(id)
}

async fn asset(
    connection: &mut SqliteConnection,
    context: &Context,
    name: &str,
    kind: &str,
    data: &[u8],
) -> Result<String> {
    let id = store_asset(connection, context, name, kind, data).await?;
    let workspace = context.workspace_id.to_string();
    let exploration = context.exploration_id.to_string();
    let operation = match kind {
        "text" => Some("cipher.rot13"),
        "image" => Some("image.pixels.read"),
        _ => None,
    };
    let pipeline_id = if let Some(operation) = operation {
        Some(pipeline(connection, &workspace, operation).await?)
    } else {
        None
    };
    sqlx::query("INSERT INTO trails(workspace_id,exploration_id,stage_id,id,title,hypothesis,source_id,source_revision,pipeline_id,pipeline_revision) VALUES(?,?,?,?,?,'',?,1,?,?)")
        .bind(workspace)
        .bind(exploration)
        .bind(context.stage_id.to_string())
        .bind(Uuid::new_v4().to_string())
        .bind(format!("Explore {}", title(name)?))
        .bind(&id)
        .bind(pipeline_id.as_ref().map(|p|p.id.to_string()))
        .bind(pipeline_id.as_ref().map(|p|p.revision))
        .execute(connection)
        .await
        .map_err(storage::failure)?;
    Ok(id)
}

pub(super) async fn add(
    connection: &mut SqliteConnection,
    context: &Context,
    name: &str,
    text: &str,
    encoded: Option<&str>,
) -> Result<Value> {
    if text.len() > protocol::contract().text_bytes {
        return Err("input_limit");
    }
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
    add_block(
        connection,
        context,
        name,
        kind,
        &data,
        (encoded.is_some() && !text.is_empty()).then_some(text),
    )
    .await
    .map(AddedBlock::value)
}

pub(super) struct AddedBlock {
    pub id: String,
    pub asset_id: String,
}
impl AddedBlock {
    pub(super) fn value(self) -> Value {
        json!({"blockId":self.id})
    }
}
pub(super) async fn add_block(
    connection: &mut SqliteConnection,
    context: &Context,
    name: &str,
    kind: &str,
    data: &[u8],
    caption: Option<&str>,
) -> Result<AddedBlock> {
    let workspace = context.workspace_id.to_string();
    let exploration = context.exploration_id.to_string();
    let stage = context.stage_id.to_string();
    blocks::admit_new(connection, context).await?;
    bytes(
        connection,
        &workspace,
        &exploration,
        data.len() + caption.map_or(0, str::len),
    )
    .await?;
    let primary = asset(connection, context, name, kind, data).await?;
    let caption = if let Some(text) = caption {
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
    let position = order::append_position(connection, context).await?;
    order::make_space(connection, context, position).await?;
    let block = Uuid::new_v4().to_string();
    sqlx::query("INSERT INTO blocks(workspace_id,exploration_id,stage_id,id,position,kind,asset_id,asset_revision,caption_id,caption_revision) VALUES(?,?,?,?,?,'asset',?,1,?,?)")
        .bind(workspace).bind(exploration).bind(stage).bind(&block).bind(position).bind(&primary).bind(&caption).bind(caption.as_ref().map(|_|1i64)).execute(&mut *connection).await.map_err(storage::failure)?;
    let expected = order::revision(connection, context).await?;
    order::advance(connection, context, expected).await?;
    Ok(AddedBlock {
        id: block,
        asset_id: primary,
    })
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
        Command::CreatePipeline {
            workspace_id,
            title,
            definition,
            ..
        } => {
            if *workspace_id != owner {
                return Err("workspace_mismatch");
            }
            super::pipelines::create(connection, &workspace, title, definition).await?
        }
        Command::RevisePipeline {
            workspace_id,
            pipeline_id,
            expected_revision,
            definition,
            ..
        } => {
            if *workspace_id != owner {
                return Err("workspace_mismatch");
            }
            super::pipelines::revise(
                connection,
                &workspace,
                &pipeline_id.to_string(),
                *expected_revision,
                definition,
            )
            .await?
        }
        Command::ForkPipeline {
            workspace_id,
            pipeline_id,
            revision,
            title,
            ..
        } => {
            if *workspace_id != owner {
                return Err("workspace_mismatch");
            }
            super::pipelines::fork(
                connection,
                &workspace,
                &pipeline_id.to_string(),
                *revision,
                title,
            )
            .await?
        }
        Command::SaveTrail {
            context,
            trail_id,
            expected_revision,
            hypothesis,
            pipeline,
            bindings,
            ..
        } => {
            queries::context(connection, owner, context).await?;
            super::pipelines::save_trail(
                connection,
                context,
                *trail_id,
                *expected_revision,
                hypothesis,
                pipeline.as_ref(),
                bindings,
            )
            .await?
        }
        Command::AddPage {
            context,
            title,
            text,
            ..
        } => {
            queries::context(connection, owner, context).await?;
            if text.len() > protocol::contract().text_bytes {
                return Err("input_limit");
            }
            add_block(connection, context, title, "page", text.as_bytes(), None)
                .await?
                .value()
        }
        Command::AddAnnotation {
            context,
            title,
            text,
            ..
        } => {
            queries::context(connection, owner, context).await?;
            notes::add(connection, context, title, text).await?
        }
        Command::SaveSolution { context, .. } => {
            queries::context(connection, owner, context).await?;
            super::solutions::save(connection, command).await?
        }
        Command::SelectCandidate {
            context,
            expected_revision,
            selection,
            ..
        } => {
            queries::context(connection, owner, context).await?;
            super::candidates::select(connection, context, *expected_revision, selection).await?
        }
        Command::PinRunOutput {
            context,
            expected_revision,
            run_id,
            attempt_id,
            port,
            ..
        } => {
            queries::context(connection, owner, context).await?;
            results::pin(
                connection,
                context,
                *expected_revision,
                *run_id,
                *attempt_id,
                port,
            )
            .await?
        }
        Command::ImportChallenge {
            workspace_id,
            package,
            ..
        } => {
            if *workspace_id != owner {
                return Err("workspace_mismatch");
            }
            super::challenges::import(connection, owner, package).await?
        }
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
            notes::save(connection, context, *note_id, *expected_revision, text).await?
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
        Command::AddFile {
            context,
            title,
            base64,
            ..
        } => {
            queries::context(connection, owner, context).await?;
            files::add(connection, context, title, base64).await?
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
        Command::ReviseText {
            context,
            block_id,
            asset_id,
            expected_revision,
            text,
            ..
        } => {
            queries::context(connection, owner, context).await?;
            assets::revise_text(
                connection,
                context,
                *block_id,
                *asset_id,
                *expected_revision,
                text,
            )
            .await?
        }
        Command::ReorderBlocks {
            context,
            expected_revision,
            block_ids,
            ..
        } => {
            queries::context(connection, owner, context).await?;
            order::reorder(connection, context, *expected_revision, block_ids).await?
        }
        Command::ArchiveBlock {
            context,
            block_id,
            expected_revision,
            source,
            ..
        } => {
            queries::context(connection, owner, context).await?;
            blocks::change(
                connection,
                context,
                *block_id,
                *expected_revision,
                source,
                true,
            )
            .await?
        }
        Command::RestoreBlock {
            context,
            block_id,
            expected_revision,
            source,
            ..
        } => {
            queries::context(connection, owner, context).await?;
            blocks::change(
                connection,
                context,
                *block_id,
                *expected_revision,
                source,
                false,
            )
            .await?
        }
        Command::DuplicateBlock {
            context,
            block_id,
            expected_revision,
            source,
            ..
        } => {
            queries::context(connection, owner, context).await?;
            copies::duplicate(connection, context, *block_id, *expected_revision, source).await?
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

// Bind existing image inspection Trails without changing their identities or evidence.
pub(super) async fn bind_image_trails(connection: &mut SqliteConnection) -> Result<()> {
    let workspaces: Vec<String> = sqlx::query_scalar("SELECT DISTINCT t.workspace_id FROM trails t JOIN assets a ON (t.workspace_id,t.exploration_id,t.source_id)=(a.workspace_id,a.exploration_id,a.id) WHERE a.kind='image' AND t.pipeline_id IS NULL").fetch_all(&mut *connection).await.map_err(storage::failure)?;
    for workspace in workspaces {
        let id = pipeline(connection, &workspace, "image.pixels.read").await?;
        sqlx::query("UPDATE trails SET pipeline_id=?,pipeline_revision=? WHERE workspace_id=? AND pipeline_id IS NULL AND EXISTS(SELECT 1 FROM assets a WHERE (a.workspace_id,a.exploration_id,a.id)=(trails.workspace_id,trails.exploration_id,trails.source_id) AND a.kind='image')").bind(id.id.to_string()).bind(id.revision).bind(workspace).execute(&mut *connection).await.map_err(storage::failure)?;
    }
    Ok(())
}
