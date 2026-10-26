use super::{
    Database,
    records::{Metadata, Note, Progress, SavedView},
};
use crate::{
    AppState,
    workspace::{FileError, Result},
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, State, rejection::JsonRejection},
    routing::{get, put},
};
use serde::Serialize;

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/investigation", get(snapshot).put(metadata))
        .route("/investigation/view", put(view))
        .route("/investigation/sessions", get(sessions))
        .route("/investigation/notes", get(notes).put(note))
        .route("/investigation/progress", get(progress).put(save_progress))
        .route("/investigation/artifacts", get(artifacts))
        .layer(DefaultBodyLimit::max(512 * 1024))
}
async fn database(state: &AppState) -> Result<std::sync::Arc<Database>> {
    state.database().await?.ok_or(FileError(
        axum::http::StatusCode::CONFLICT,
        "persistence_unavailable",
        "Open a workspace with managed storage through cube play to preserve investigation state.",
    ))
}
fn body<T>(value: std::result::Result<Json<T>, JsonRejection>) -> Result<T> {
    value.map(|Json(value)| value).map_err(|_| super::invalid())
}
#[derive(Serialize)]
struct Revision {
    revision: i64,
}
async fn snapshot(State(state): State<AppState>) -> Result<Json<super::Snapshot>> {
    Ok(Json(database(&state).await?.snapshot().await?))
}
async fn metadata(
    State(state): State<AppState>,
    value: std::result::Result<Json<Metadata>, JsonRejection>,
) -> Result<Json<Revision>> {
    Ok(Json(Revision {
        revision: database(&state).await?.metadata(body(value)?).await?,
    }))
}
async fn view(
    State(state): State<AppState>,
    value: std::result::Result<Json<SavedView>, JsonRejection>,
) -> Result<Json<Revision>> {
    Ok(Json(Revision {
        revision: database(&state).await?.save_view(body(value)?).await?,
    }))
}
async fn notes(State(state): State<AppState>) -> Result<Json<Vec<Note>>> {
    Ok(Json(database(&state).await?.notes().await?))
}
async fn note(
    State(state): State<AppState>,
    value: std::result::Result<Json<Note>, JsonRejection>,
) -> Result<Json<Revision>> {
    Ok(Json(Revision {
        revision: database(&state).await?.save_note(body(value)?).await?,
    }))
}
async fn progress(State(state): State<AppState>) -> Result<Json<Vec<serde_json::Value>>> {
    Ok(Json(database(&state).await?.progress().await?))
}
async fn save_progress(
    State(state): State<AppState>,
    value: std::result::Result<Json<Progress>, JsonRejection>,
) -> Result<Json<Revision>> {
    Ok(Json(Revision {
        revision: database(&state).await?.save_progress(body(value)?).await?,
    }))
}
async fn artifacts(State(state): State<AppState>) -> Result<Json<Vec<super::records::Artifact>>> {
    Ok(Json(database(&state).await?.artifacts().await?))
}

async fn sessions(State(state): State<AppState>) -> Result<Json<Vec<super::records::Session>>> {
    Ok(Json(database(&state).await?.sessions().await?))
}
