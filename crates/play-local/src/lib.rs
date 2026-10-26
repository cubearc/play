//! Local application boundary. Cube Core owns investigation algorithms.
pub mod config;
mod security;

use std::sync::Arc;

use axum::{
    Json, Router, middleware,
    routing::{get, post},
};
use serde::Serialize;
use tower_http::{limit::RequestBodyLimitLayer, services::ServeDir};

use config::Config;

#[derive(Clone)]
struct AppState {
    config: Arc<Config>,
    secret: Arc<str>,
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
    let frontend = ServeDir::new(&config.frontend_dir);
    let state = AppState {
        config: Arc::new(config),
        secret: secret.into(),
    };
    let api =
        Router::new()
            .route("/health", get(health))
            .route_layer(middleware::from_fn_with_state(
                state.clone(),
                security::require_session,
            ));
    Router::new()
        .nest("/api", api)
        .route("/api/session", post(security::open_session))
        .route("/healthz", get(health))
        .fallback_service(frontend)
        .layer(RequestBodyLimitLayer::new(1024))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            security::boundary,
        ))
        .with_state(state)
}

async fn health() -> Json<Health> {
    Json(Health {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
    })
}
