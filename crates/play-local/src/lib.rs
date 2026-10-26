//! Local application boundary. Cube Core owns investigation algorithms.
pub mod config;
mod persistence;
mod security;
mod terminal;
pub use persistence::PersistenceHandle;
pub mod workspace;

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    middleware,
    routing::{get, post},
};
use serde::Serialize;
use tower_http::{limit::RequestBodyLimitLayer, services::ServeDir};

use config::Config;

#[derive(Clone)]
struct AppState {
    persistence: PersistenceHandle,
    config: Arc<Config>,
    secret: Arc<str>,
    workspace: workspace::Result<Option<Arc<workspace::Workspace>>>,
}

impl AppState {
    async fn database(&self) -> workspace::Result<Option<Arc<persistence::Database>>> {
        let workspace = self.workspace.as_ref().map_err(|error| *error)?;
        self.persistence.get(&self.config, workspace.as_ref()).await
    }
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
    version: &'static str,
}

/// Creates a new per-process capability. Do not persist or log it in diagnostics.
pub fn session_secret() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn app(config: Config, secret: String) -> Router {
    application(config, secret).0
}

pub fn application(config: Config, secret: String) -> (Router, PersistenceHandle) {
    let frontend = ServeDir::new(&config.frontend_dir);
    let persistence = PersistenceHandle::default();
    let state = AppState {
        persistence: persistence.clone(),
        workspace: workspace::Workspace::open(&config),
        config: Arc::new(config),
        secret: secret.into(),
    };
    let api = Router::new()
        .route("/health", get(workspace_health))
        .route("/workspace/files", get(workspace::files))
        .route("/workspace/inspect", get(workspace::inspection))
        .route("/workspace/media", get(workspace::media))
        .route("/workspace/import", post(workspace::import))
        .merge(persistence::routes())
        .merge(terminal::routes())
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            security::require_session,
        ));
    let router = Router::new()
        .nest("/api", api)
        .route(
            "/api/session",
            post(security::open_session).layer(RequestBodyLimitLayer::new(1024)),
        )
        .route("/healthz", get(health))
        .fallback_service(frontend)
        .layer(RequestBodyLimitLayer::new(workspace::MAX_IMPORT as usize))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            security::boundary,
        ))
        .with_state(state);
    (router, persistence)
}

async fn health(State(state): State<AppState>) -> Result<Json<Health>, workspace::FileError> {
    state.database().await?;
    if state.config.runner_socket.is_some() {
        terminal::probe(&state.config).await?;
    }
    Ok(Json(Health {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    }))
}

#[derive(Serialize)]
struct WorkspaceHealth {
    #[serde(flatten)]
    health: Health,
    #[serde(skip_serializing_if = "Option::is_none")]
    workspace: Option<config::WorkspaceContext>,
}

async fn workspace_health(
    State(state): State<AppState>,
) -> workspace::Result<Json<WorkspaceHealth>> {
    state.database().await?;
    Ok(Json(WorkspaceHealth {
        health: Health {
            status: "ok",
            version: env!("CARGO_PKG_VERSION"),
        },
        workspace: state.config.workspace.clone(),
    }))
}
