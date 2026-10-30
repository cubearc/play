//! Inert, self-contained packages enter the same notebook transaction as manual content.
use super::{files, model::Context, mutations, notes, queries, storage};
use crate::{Result, protocol};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::SqliteConnection;
use std::collections::BTreeSet;
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Package {
    format: String,
    version: u32,
    id: Uuid,
    revision: u32,
    title: String,
    origin: Option<RemoteOrigin>,
    stages: Vec<Stage>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Stage {
    id: Uuid,
    title: String,
    blocks: Vec<Block>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Block {
    id: Uuid,
    title: String,
    content: Content,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Content {
    Text {
        text: String,
    },
    Page {
        text: String,
    },
    Annotation {
        text: String,
    },
    Image {
        base64: String,
        sha256: String,
        caption: String,
    },
    File {
        base64: String,
        sha256: String,
    },
}
/// Claimed source identity only. Local imports confer no service authorization.
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RemoteOrigin {
    service_id: Uuid,
    challenge_id: Uuid,
    edition_id: Uuid,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Origin {
    package_id: Uuid,
    revision: u32,
    sha256: String,
    remote: Option<RemoteOrigin>,
    stages: Vec<StageOrigin>,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StageOrigin {
    source_id: Uuid,
    stage_id: Uuid,
    blocks: Vec<BlockOrigin>,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BlockOrigin {
    source_id: Uuid,
    block_id: Uuid,
}
fn identity(ids: &mut BTreeSet<Uuid>, id: Uuid) -> Result<()> {
    if id.is_nil() || !ids.insert(id) {
        return Err("invalid_package");
    }
    Ok(())
}
fn decode(encoded: &str, maximum: usize) -> Result<Vec<u8>> {
    if encoded.len() > maximum.div_ceil(3) * 4 {
        return Err("package_limit");
    }
    let bytes = STANDARD.decode(encoded).map_err(|_| "invalid_package")?;
    if bytes.len() > maximum {
        return Err("package_limit");
    }
    Ok(bytes)
}
fn parse(encoded: &str) -> Result<(Package, String)> {
    let limits = protocol::contract();
    let bytes = decode(encoded, limits.package_bytes)?;
    let text = std::str::from_utf8(&bytes).map_err(|_| "invalid_package")?;
    let value = cube_core::pipeline::json::parse_with_limits(
        text,
        limits.package_bytes,
        16,
        limits.package_bytes,
    )
    .map_err(|_| "invalid_package")?;
    if value["format"] != "cubearc.play.challenge" || value["version"] != 1 {
        return Err("unsupported_package");
    }
    let package: Package = serde_json::from_value(value).map_err(|_| "invalid_package")?;
    package.validate()?;
    Ok((package, cube_core::hashing::sha256(&bytes)))
}
impl Package {
    fn validate(&self) -> Result<()> {
        // Retain closed format fields in the typed boundary as well as the version check.
        if self.format != "cubearc.play.challenge" || self.version != 1 || self.revision == 0 {
            return Err("invalid_package");
        }
        mutations::title(&self.title)?;
        if self.stages.is_empty() || self.stages.len() > protocol::contract().stage_limit {
            return Err("package_limit");
        }
        if self.origin.as_ref().is_some_and(|o| {
            o.service_id.is_nil() || o.challenge_id.is_nil() || o.edition_id.is_nil()
        }) {
            return Err("invalid_package");
        }
        let mut ids = BTreeSet::new();
        identity(&mut ids, self.id)?;
        for stage in &self.stages {
            identity(&mut ids, stage.id)?;
            mutations::title(&stage.title)?;
            // Every Stage keeps its own editable Reasoning block.
            if stage.blocks.len() >= protocol::contract().block_limit {
                return Err("package_limit");
            }
            for block in &stage.blocks {
                identity(&mut ids, block.id)?;
                mutations::title(&block.title)?;
            }
        }
        Ok(())
    }
}
impl Block {
    async fn import(&self, connection: &mut SqliteConnection, context: &Context) -> Result<Uuid> {
        let title = &self.title;
        let value = match &self.content {
            Content::Text { text } => {
                mutations::add(connection, context, title, text, None).await?
            }
            Content::Image {
                base64,
                sha256,
                caption,
            } => {
                integrity(base64, sha256, protocol::contract().image_bytes)?;
                mutations::add(connection, context, title, caption, Some(base64)).await?
            }
            Content::File { base64, sha256 } => {
                integrity(base64, sha256, protocol::contract().file_bytes)?;
                files::add(connection, context, title, base64).await?
            }
            Content::Annotation { text } => notes::add(connection, context, title, text).await?,
            Content::Page { text } => {
                if text.len() > protocol::contract().text_bytes {
                    return Err("input_limit");
                }
                mutations::add_block(connection, context, title, "page", text.as_bytes(), None)
                    .await?
                    .value()
            }
        };
        serde_json::from_value(value["blockId"].clone()).map_err(|_| "invalid_state")
    }
}
fn integrity(encoded: &str, digest: &str, maximum: usize) -> Result<()> {
    let data = decode(encoded, maximum)?;
    if cube_core::hashing::sha256(&data) != digest {
        return Err("package_integrity");
    }
    Ok(())
}

pub(super) async fn import(
    connection: &mut SqliteConnection,
    workspace: Uuid,
    encoded: &str,
) -> Result<Value> {
    let (package, sha256) = parse(encoded)?;
    let created =
        mutations::new_exploration(connection, &workspace.to_string(), &package.title).await?;
    let exploration: Uuid =
        serde_json::from_value(created["explorationId"].clone()).map_err(|_| "invalid_state")?;
    let first: Uuid =
        serde_json::from_value(created["stageId"].clone()).map_err(|_| "invalid_state")?;
    let mut stages = Vec::new();
    for (index, source) in package.stages.iter().enumerate() {
        let stage = if index == 0 {
            sqlx::query(
                "UPDATE stages SET title=? WHERE workspace_id=? AND exploration_id=? AND id=?",
            )
            .bind(source.title.trim())
            .bind(workspace.to_string())
            .bind(exploration.to_string())
            .bind(first.to_string())
            .execute(&mut *connection)
            .await
            .map_err(storage::failure)?;
            first
        } else {
            Uuid::parse_str(
                &mutations::stage(
                    connection,
                    &workspace.to_string(),
                    &exploration.to_string(),
                    &source.title,
                )
                .await?,
            )
            .map_err(|_| "invalid_state")?
        };
        let context = Context {
            workspace_id: workspace,
            exploration_id: exploration,
            stage_id: stage,
        };
        let mut blocks = Vec::new();
        for block in &source.blocks {
            blocks.push(BlockOrigin {
                source_id: block.id,
                block_id: block.import(connection, &context).await?,
            });
        }
        stages.push(StageOrigin {
            source_id: source.id,
            stage_id: stage,
            blocks,
        });
    }
    let origin = Origin {
        package_id: package.id,
        revision: package.revision,
        sha256,
        remote: package.origin,
        stages,
    };
    sqlx::query("INSERT INTO challenge_imports VALUES(?,?,?)")
        .bind(workspace.to_string())
        .bind(exploration.to_string())
        .bind(serde_json::to_string(&origin).map_err(|_| "invalid_state")?)
        .execute(&mut *connection)
        .await
        .map_err(storage::failure)?;
    queries::select(
        connection,
        &workspace.to_string(),
        &exploration.to_string(),
        &first.to_string(),
    )
    .await?;
    Ok(json!({"explorationId":exploration,"stageId":first}))
}

pub(super) async fn origin(
    connection: &mut SqliteConnection,
    workspace: &str,
    exploration: &str,
) -> Result<Option<Origin>> {
    let value: Option<String> = sqlx::query_scalar(
        "SELECT origin FROM challenge_imports WHERE workspace_id=? AND exploration_id=?",
    )
    .bind(workspace)
    .bind(exploration)
    .fetch_optional(connection)
    .await
    .map_err(storage::failure)?;
    value
        .map(|v| serde_json::from_str(&v).map_err(|_| "corrupt_store"))
        .transpose()
}
