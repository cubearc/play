//! Bounded, non-executable public manifests; no operation/recipe evaluator is involved.
use crate::{
    AppState,
    workspace::{self, FileError, Result, Store},
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State},
    http::StatusCode,
    routing::{get, post, put},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::HashSet, io::Read};

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Artifact {
    pub id: String,
    pub path: String,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Step {
    pub id: String,
    pub instructions: String,
}
#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AnswerType {
    #[default]
    Text,
    Bytes,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Stage {
    pub id: String,
    pub title: String,
    pub instructions: String,
    #[serde(default)]
    pub steps: Vec<Step>,
    #[serde(default)]
    pub hints: Vec<String>,
    #[serde(default)]
    pub artifacts: Vec<String>,
    #[serde(default)]
    pub answer_type: AnswerType,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Manifest {
    pub kind: String,
    pub format_version: u32,
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub artifacts: Vec<Artifact>,
    pub stages: Vec<Stage>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Definition {
    pub key: String,
    pub store: Store,
    pub path: String,
    pub manifest: Manifest,
}
fn invalid() -> FileError {
    FileError(
        StatusCode::BAD_REQUEST,
        "invalid_challenge",
        "Use a version-1 cube-play-challenge JSON manifest with unique IDs, bounded inert instructions and authorized local artifacts. Workflow recipes, scripts and private answers are unsupported.",
    )
}
fn bounded(value: &str, max: usize) -> bool {
    !value.trim().is_empty() && value.len() <= max && !value.contains('\0')
}
fn id(value: &str) -> bool {
    bounded(value, 64)
        && value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-'))
}
fn unique<'a>(values: impl Iterator<Item = &'a str>) -> bool {
    let mut seen = HashSet::new();
    values.into_iter().all(|v| seen.insert(v))
}
pub(crate) fn key(value: &str) -> Result<()> {
    if value.len() == 64
        && value
            .bytes()
            .all(|v| v.is_ascii_hexdigit() && !v.is_ascii_uppercase())
    {
        Ok(())
    } else {
        Err(invalid())
    }
}
impl Manifest {
    pub(crate) fn validate(&self) -> Result<()> {
        if self.kind != "cube-play-challenge"
            || self.format_version != 1
            || !id(&self.id)
            || !bounded(&self.title, 512)
            || self.description.len() > 8192
            || self.artifacts.len() > 32
            || self.stages.is_empty()
            || self.stages.len() > 32
            || !unique(self.artifacts.iter().map(|v| v.path.as_str()))
            || !unique(self.artifacts.iter().map(|v| v.id.as_str()))
            || !unique(self.stages.iter().map(|v| v.id.as_str()))
        {
            return Err(invalid());
        }
        for artifact in &self.artifacts {
            if !id(&artifact.id) {
                return Err(invalid());
            }
            workspace::validate(&artifact.path, false)?;
        }
        for stage in &self.stages {
            if !id(&stage.id)
                || !bounded(&stage.title, 512)
                || !bounded(&stage.instructions, 8192)
                || stage.steps.len() > 16
                || stage.hints.len() > 16
                || stage.artifacts.len() > 32
                || !unique(stage.steps.iter().map(|v| v.id.as_str()))
                || !unique(stage.artifacts.iter().map(String::as_str))
                || stage
                    .steps
                    .iter()
                    .any(|v| !id(&v.id) || !bounded(&v.instructions, 8192))
                || stage.hints.iter().any(|v| !bounded(v, 8192))
                || stage
                    .artifacts
                    .iter()
                    .any(|v| !self.artifacts.iter().any(|a| a.id == *v))
            {
                return Err(invalid());
            }
        }
        Ok(())
    }
}
impl Definition {
    pub(crate) fn artifact_path(&self, path: &str) -> Result<String> {
        workspace::validate(path, false)?;
        let parent = self.path.rsplit_once('/').map(|v| v.0);
        let resolved = parent
            .map(|parent| format!("{parent}/{path}"))
            .unwrap_or_else(|| path.into());
        workspace::validate(&resolved, false)?;
        Ok(resolved)
    }
    pub(crate) fn validate(&self) -> Result<()> {
        key(&self.key)?;
        workspace::validate(&self.path, false)?;
        self.manifest.validate()?;
        for artifact in &self.manifest.artifacts {
            self.artifact_path(&artifact.path)?;
        }
        Ok(())
    }
}
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/challenges", get(list))
        .route("/challenges/import", post(import))
        .route("/challenges/active", put(activate))
        .route("/challenges/stage", put(navigate))
        .route("/challenges/progress", put(progress))
        .layer(DefaultBodyLimit::max(16384))
}
async fn database(app: &AppState) -> Result<std::sync::Arc<crate::persistence::Database>> {
    app.database().await?.ok_or_else(crate::persistence::failed)
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Import {
    store: Store,
    path: String,
}
async fn import(State(app): State<AppState>, Json(input): Json<Import>) -> Result<Json<Value>> {
    workspace::validate(&input.path, false)?;
    let db = database(&app).await?;
    if input.store == Store::Imports {
        db.authorize_import(&input.path).await?;
    }
    let workspace = app
        .workspace
        .as_ref()
        .map_err(|e| *e)?
        .clone()
        .ok_or_else(invalid)?;
    let _permit = workspace
        .slots
        .clone()
        .try_acquire_owned()
        .map_err(|_| workspace::limit())?;
    let imports = input.store == Store::Imports;
    let definition = tokio::task::spawn_blocking(move || -> Result<Definition> {
        let mut bytes = Vec::new();
        workspace
            .file(input.store, &input.path)?
            .take(256 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        if bytes.len() > 256 * 1024 {
            return Err(workspace::limit());
        }
        let text = std::str::from_utf8(&bytes).map_err(|_| invalid())?;
        let parsed = cube_core::pipeline::json::parse(text).map_err(|_| invalid())?;
        let manifest: Manifest = serde_json::from_value(parsed).map_err(|_| invalid())?;
        let mut identity =
            serde_json::to_vec(&(input.store, &input.path)).map_err(|_| invalid())?;
        identity.push(0);
        identity.extend_from_slice(&bytes);
        let definition = Definition {
            key: cube_core::hashing::sha256(&identity),
            manifest,
            store: input.store,
            path: input.path,
        };
        definition.validate()?;
        let mut total = 0u64;
        for artifact in &definition.manifest.artifacts {
            let path = definition.artifact_path(&artifact.path)?;
            let file = workspace.file(definition.store, &path)?;
            let size = file.metadata()?.len();
            total = total.checked_add(size).ok_or_else(workspace::limit)?;
            if size > 32 * 1024 * 1024 || total > 64 * 1024 * 1024 {
                return Err(workspace::limit());
            }
        }
        Ok(definition)
    })
    .await
    .map_err(|_| invalid())??;
    if imports {
        for artifact in &definition.manifest.artifacts {
            db.authorize_import(&definition.artifact_path(&artifact.path)?)
                .await?;
        }
    }
    db.import_challenge(&definition).await?;
    Ok(Json(db.challenges().await?))
}
async fn list(State(app): State<AppState>) -> Result<Json<Value>> {
    Ok(Json(database(&app).await?.challenges().await?))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Active {
    key: String,
}
async fn activate(State(app): State<AppState>, Json(input): Json<Active>) -> Result<Json<Value>> {
    let db = database(&app).await?;
    db.activate_challenge(&input.key).await?;
    Ok(Json(db.challenges().await?))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Navigation {
    key: String,
    stage: String,
    revision: i64,
}
async fn navigate(
    State(app): State<AppState>,
    Json(input): Json<Navigation>,
) -> Result<Json<Value>> {
    let db = database(&app).await?;
    db.navigate_challenge(&input.key, &input.stage, input.revision)
        .await?;
    Ok(Json(db.challenges().await?))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Progress {
    key: String,
    stage: String,
    revision: i64,
    status: crate::persistence::LocalStatus,
}
async fn progress(State(app): State<AppState>, Json(input): Json<Progress>) -> Result<Json<Value>> {
    let db = database(&app).await?;
    let definition = db.challenge(&input.key).await?.0;
    if !definition
        .manifest
        .stages
        .iter()
        .any(|v| v.id == input.stage)
    {
        return Err(invalid());
    }
    db.save_progress(crate::persistence::Progress {
        challenge_key: input.key,
        stage_key: input.stage,
        revision: input.revision,
        local_status: input.status,
    })
    .await?;
    Ok(Json(db.challenges().await?))
}
