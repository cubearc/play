use super::{
    model::*,
    storage::{self, Owned},
};
use crate::Result;
use sqlx::{Row, SqliteConnection};
use uuid::Uuid;

pub async fn context(
    connection: &mut SqliteConnection,
    owner: Uuid,
    context: &Context,
) -> Result<()> {
    if context.workspace_id != owner {
        return Err("workspace_mismatch");
    }
    let exists: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM stages WHERE workspace_id=? AND exploration_id=? AND id=?",
    )
    .bind(owner.to_string())
    .bind(context.exploration_id.to_string())
    .bind(context.stage_id.to_string())
    .fetch_one(connection)
    .await
    .map_err(storage::failure)?;
    if exists != 1 {
        return Err("not_found");
    }
    Ok(())
}

pub async fn asset(
    connection: &mut SqliteConnection,
    workspace: &str,
    exploration: &str,
    id: &str,
    revision: i64,
) -> Result<Asset> {
    let row = sqlx::query("SELECT a.title,a.kind,r.data,r.digest FROM assets a JOIN asset_revisions r ON (a.workspace_id,a.exploration_id,a.id)=(r.workspace_id,r.exploration_id,r.asset_id) WHERE a.workspace_id=? AND a.exploration_id=? AND a.id=? AND r.revision=?")
        .bind(workspace).bind(exploration).bind(id).bind(revision).fetch_optional(connection).await.map_err(storage::failure)?.ok_or("not_found")?;
    let data: Vec<u8> = row.get("data");
    if cube_core::hashing::sha256(&data) != row.get::<String, _>("digest") {
        return Err("corrupt_store");
    }
    let content = match row.get::<String, _>("kind").as_str() {
        "text" => Content::Text {
            text: if data.len() <= crate::protocol::contract().text_bytes {
                String::from_utf8(data).map_err(|_| "corrupt_store")?
            } else {
                return Err("corrupt_store");
            },
        },
        "image" if data.len() <= crate::protocol::contract().image_bytes => Content::Image,
        _ => return Err("corrupt_store"),
    };
    Ok(Asset {
        id: id.into(),
        exploration_id: exploration.into(),
        revision,
        title: row.get("title"),
        content,
    })
}

pub async fn pipelines(
    connection: &mut SqliteConnection,
    workspace: &str,
) -> Result<Vec<Pipeline>> {
    let rows = sqlx::query("SELECT p.id,p.title,p.current_revision,r.definition FROM pipelines p JOIN pipeline_revisions r ON (p.workspace_id,p.id,p.current_revision)=(r.workspace_id,r.pipeline_id,r.revision) WHERE p.workspace_id=? ORDER BY p.id")
        .bind(workspace).fetch_all(connection).await.map_err(storage::failure)?;
    rows.into_iter()
        .map(|row| {
            let definition: serde_json::Value =
                serde_json::from_str(row.get::<&str, _>("definition"))
                    .map_err(|_| "corrupt_store")?;
            Ok(Pipeline {
                id: row.get("id"),
                title: row.get("title"),
                revision: row.get("current_revision"),
                input: "source".into(),
                initial_operation: definition["nodes"][0]["operation"]
                    .as_str()
                    .ok_or("corrupt_store")?
                    .into(),
                operation_version: definition["nodes"][0]["operation_version"]
                    .as_u64()
                    .and_then(|v| u32::try_from(v).ok())
                    .ok_or("corrupt_store")?,
            })
        })
        .collect()
}

pub async fn view(store: &mut Owned) -> Result<View> {
    let id = store.identity.workspace_id.to_string();
    let title = store.identity.title.clone();
    let connection = store.connection()?;
    let rows = sqlx::query(
        "SELECT id,title,revision FROM explorations WHERE workspace_id=? ORDER BY rowid",
    )
    .bind(&id)
    .fetch_all(&mut *connection)
    .await
    .map_err(storage::failure)?;
    let explorations = rows
        .into_iter()
        .map(|row| Summary {
            id: row.get(0),
            title: row.get(1),
            revision: row.get(2),
        })
        .collect();
    let navigation = sqlx::query(
        "SELECT exploration_id,stage_id FROM view_state WHERE workspace_id=? AND slot='primary'",
    )
    .bind(&id)
    .fetch_optional(&mut *connection)
    .await
    .map_err(storage::failure)?;
    Ok(View {
        id: id.clone(),
        title,
        generation: storage::GENERATION,
        explorations,
        pipelines: pipelines(connection, &id).await?,
        selected_exploration_id: navigation.as_ref().map(|row| row.get(0)),
        selected_stage_id: navigation.as_ref().map(|row| row.get(1)),
    })
}

pub async fn select(
    connection: &mut SqliteConnection,
    workspace: &str,
    exploration: &str,
    stage: &str,
) -> Result<()> {
    sqlx::query("INSERT INTO view_state(workspace_id,slot,exploration_id,stage_id,revision) VALUES(?,'primary',?,?,1) ON CONFLICT(workspace_id,slot) DO UPDATE SET exploration_id=excluded.exploration_id,stage_id=excluded.stage_id,revision=view_state.revision+1")
        .bind(workspace).bind(exploration).bind(stage).execute(&mut *connection).await.map_err(storage::failure)?;
    sqlx::query("INSERT INTO exploration_view_state VALUES(?,?,?) ON CONFLICT(workspace_id,exploration_id) DO UPDATE SET stage_id=excluded.stage_id")
        .bind(workspace).bind(exploration).bind(stage).execute(connection).await.map_err(storage::failure)?;
    Ok(())
}

pub async fn exploration(
    store: &mut Owned,
    exploration_id: Uuid,
    stage_id: Option<Uuid>,
) -> Result<Exploration> {
    let workspace = store.identity.workspace_id.to_string();
    let exploration = exploration_id.to_string();
    let connection = store.connection()?;
    let record =
        sqlx::query("SELECT title,revision FROM explorations WHERE workspace_id=? AND id=?")
            .bind(&workspace)
            .bind(&exploration)
            .fetch_optional(&mut *connection)
            .await
            .map_err(storage::failure)?
            .ok_or("not_found")?;
    let stages = sqlx::query("SELECT id,title,revision FROM stages WHERE workspace_id=? AND exploration_id=? ORDER BY position,id")
        .bind(&workspace).bind(&exploration).fetch_all(&mut *connection).await.map_err(storage::failure)?
        .into_iter().map(|row| Summary {id:row.get(0),title:row.get(1),revision:row.get(2)}).collect::<Vec<_>>();
    let previous: Option<String> = sqlx::query_scalar(
        "SELECT stage_id FROM exploration_view_state WHERE workspace_id=? AND exploration_id=?",
    )
    .bind(&workspace)
    .bind(&exploration)
    .fetch_optional(&mut *connection)
    .await
    .map_err(storage::failure)?;
    let selected = stage_id
        .map(|id| id.to_string())
        .or(previous)
        .or_else(|| stages.first().map(|stage| stage.id.clone()))
        .ok_or("corrupt_store")?;
    let stage_summary = stages
        .iter()
        .find(|stage| stage.id == selected)
        .ok_or("not_found")?;
    let note = sqlx::query("SELECT n.id,n.current_revision,r.text FROM notes n JOIN note_revisions r ON (n.workspace_id,n.exploration_id,n.id,n.current_revision)=(r.workspace_id,r.exploration_id,r.note_id,r.revision) WHERE n.workspace_id=? AND n.exploration_id=? AND n.stage_id=?")
        .bind(&workspace).bind(&exploration).bind(&selected).fetch_one(&mut *connection).await.map_err(storage::failure)?;
    let block_rows = sqlx::query("SELECT id,asset_id,asset_revision,caption_id,caption_revision FROM blocks WHERE workspace_id=? AND exploration_id=? AND stage_id=? AND kind='asset' ORDER BY position,id")
        .bind(&workspace).bind(&exploration).bind(&selected).fetch_all(&mut *connection).await.map_err(storage::failure)?;
    let mut blocks = Vec::new();
    for row in block_rows {
        let caption_id: Option<String> = row.get("caption_id");
        let caption = if let Some(id) = caption_id {
            Some(
                asset(
                    connection,
                    &workspace,
                    &exploration,
                    &id,
                    row.get("caption_revision"),
                )
                .await?,
            )
        } else {
            None
        };
        blocks.push(Block {
            id: row.get("id"),
            asset: asset(
                connection,
                &workspace,
                &exploration,
                row.get("asset_id"),
                row.get("asset_revision"),
            )
            .await?,
            caption,
        });
    }
    let definitions = pipelines(connection, &workspace).await?;
    let trail_rows = sqlx::query("SELECT id,title,hypothesis,source_id,source_revision,pipeline_id,pipeline_revision FROM trails WHERE workspace_id=? AND exploration_id=? AND stage_id=? ORDER BY rowid")
        .bind(&workspace).bind(&exploration).bind(&selected).fetch_all(&mut *connection).await.map_err(storage::failure)?;
    let mut trails = Vec::new();
    for row in trail_rows {
        let pipeline_id: Option<String> = row.get("pipeline_id");
        let pipeline = pipeline_id
            .as_ref()
            .and_then(|id| {
                definitions.iter().find(|pipeline| {
                    pipeline.id == *id
                        && pipeline.revision == row.get::<i64, _>("pipeline_revision")
                })
            })
            .cloned();
        if pipeline_id.is_some() && pipeline.is_none() {
            return Err("corrupt_store");
        }
        trails.push(Trail {
            id: row.get("id"),
            workspace_id: workspace.clone(),
            exploration_id: exploration.clone(),
            stage_id: selected.clone(),
            title: row.get("title"),
            hypothesis: row.get("hypothesis"),
            source: asset(
                connection,
                &workspace,
                &exploration,
                row.get("source_id"),
                row.get("source_revision"),
            )
            .await?,
            pipeline_id,
            pipeline,
        });
    }
    let stage = Stage {
        id: selected.clone(),
        title: stage_summary.title.clone(),
        revision: stage_summary.revision,
        introduction: String::new(),
        blocks,
        note: Note {
            id: note.get(0),
            revision: note.get(1),
            text: note.get(2),
        },
    };
    select(connection, &workspace, &exploration, &selected).await?;
    Ok(Exploration {
        id: exploration,
        workspace_id: workspace,
        title: record.get(0),
        revision: record.get(1),
        stages,
        stage,
        trails,
    })
}
