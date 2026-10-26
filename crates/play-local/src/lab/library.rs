//! Private result naming, history and explicit browser-mediated exports.
use super::{read_permit, reference_id, types::*};
use crate::{AppState, persistence::library::SavedResult};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, Query, State},
    http::{HeaderValue, header},
    response::{IntoResponse, Response},
    routing::get,
};
use serde::Deserialize;
use serde_json::{Value, json};

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/lab/library", get(list).put(save))
        .route("/lab/history", get(history))
        .route("/lab/results/{id}/export", get(export))
        .layer(DefaultBodyLimit::max(16 * 1024))
}
async fn database(app: &AppState) -> Result<std::sync::Arc<crate::persistence::Database>> {
    app.database().await?.ok_or_else(Error::storage)
}
async fn list(State(app): State<AppState>) -> Result<Json<Value>> {
    let entries = database(&app).await?.saved_results().await?;
    let storage = app.lab.storage(&app).await?;
    let _permit = read_permit(&app)?;
    let values = tokio::task::spawn_blocking(move || {
        entries
            .into_iter()
            .map(|entry| match storage.load(entry.id) {
                Ok((record, _)) => json!({"entry":entry,"record":record,"error":null}),
                Err(error) => json!({"entry":entry,"record":null,"error":error.message}),
            })
            .collect::<Vec<_>>()
    })
    .await
    .map_err(|_| Error::storage())?;
    Ok(Json(json!({"schema_version":1,"entries":values})))
}
async fn save(State(app): State<AppState>, Json(entry): Json<SavedResult>) -> Result<Json<Value>> {
    let storage = app.lab.storage(&app).await?;
    let id = entry.id;
    let _permit = read_permit(&app)?;
    tokio::task::spawn_blocking(move || storage.load(id))
        .await
        .map_err(|_| Error::storage())??;
    let revision = database(&app).await?.save_result(entry).await?;
    Ok(Json(json!({"schema_version":1,"revision":revision})))
}
async fn history(State(app): State<AppState>) -> Result<Json<Value>> {
    let storage = app.lab.storage(&app).await?;
    let _permit = read_permit(&app)?;
    let records = tokio::task::spawn_blocking(move || storage.history())
        .await
        .map_err(|_| Error::storage())??;
    Ok(Json(json!({"schema_version":1,"records":records})))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Export {
    name: String,
}
async fn export(
    State(app): State<AppState>,
    Path(id): Path<String>,
    Query(query): Query<Export>,
) -> Result<Response> {
    crate::workspace::validate(&query.name, false)?;
    if query.name.len() > 128
        || !query
            .name
            .bytes()
            .all(|v| v.is_ascii_alphanumeric() || matches!(v, b'.' | b'_' | b'-'))
    {
        return Err(Error::invalid());
    }
    let id = reference_id(&id)?;
    let storage = app.lab.storage(&app).await?;
    let _permit = read_permit(&app)?;
    let (_, bytes) = tokio::task::spawn_blocking(move || storage.load(id))
        .await
        .map_err(|_| Error::storage())??;
    let disposition = HeaderValue::from_str(&format!("attachment; filename=\"{}\"", query.name))
        .map_err(|_| Error::invalid())?;
    Ok((
        [
            (
                header::CONTENT_TYPE,
                HeaderValue::from_static("application/octet-stream"),
            ),
            (header::CONTENT_DISPOSITION, disposition),
            (
                header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            ),
        ],
        bytes,
    )
        .into_response())
}
// The browser selects the export destination. Containers never receive a host write root.
