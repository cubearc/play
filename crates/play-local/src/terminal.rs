//! Browser authorization and byte-preserving forwarding to the private runner.
use crate::{
    AppState,
    workspace::{FileError, Result},
};
#[cfg(unix)]
use axum::extract::ws::{Message, WebSocket};
use axum::{
    Json, Router,
    extract::{Query, State, WebSocketUpgrade, rejection::QueryRejection},
    http::{HeaderMap, StatusCode},
    response::Response,
    routing::get,
};
use play_runner::protocol::{self, Location, Size, Store};
#[cfg(unix)]
use play_runner::protocol::{Client, Server};
use serde::{Deserialize, Serialize};

pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .route("/terminal", get(status))
        .route("/terminal/ws", get(upgrade))
}
fn unavailable() -> FileError {
    FileError(
        StatusCode::SERVICE_UNAVAILABLE,
        "terminal_unavailable",
        "The isolated runner is unavailable. Reopen this workspace through cube play; native API development does not start host terminals.",
    )
}
fn invalid() -> FileError {
    FileError(
        StatusCode::BAD_REQUEST,
        "invalid_terminal",
        "Choose an authorized relative directory and a terminal size of 20–240 columns and 5–80 rows.",
    )
}
#[derive(Serialize)]
struct Status {
    available: bool,
    active: usize,
    limit: usize,
    idle_seconds: u64,
    lifetime_seconds: u64,
    input_limit: usize,
    output_limit: usize,
}
async fn status(State(state): State<AppState>) -> Result<Json<Status>> {
    let active = if state.config.runner_socket.is_some() {
        probe(&state.config).await?
    } else {
        0
    };
    Ok(Json(Status {
        available: state.config.runner_socket.is_some(),
        active,
        limit: protocol::SESSION_LIMIT,
        idle_seconds: protocol::IDLE_SECONDS,
        lifetime_seconds: protocol::LIFETIME_SECONDS,
        input_limit: protocol::INPUT_LIMIT,
        output_limit: protocol::OUTPUT_LIMIT,
    }))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Selection {
    #[serde(default)]
    store: Store,
    #[serde(default)]
    path: String,
    cols: u16,
    rows: u16,
}
async fn upgrade(
    State(state): State<AppState>,
    headers: HeaderMap,
    query: std::result::Result<Query<Selection>, QueryRejection>,
    websocket: WebSocketUpgrade,
) -> Result<Response> {
    if headers.get("origin").and_then(|v| v.to_str().ok()) != Some(&state.config.origin) {
        return Err(FileError(
            StatusCode::FORBIDDEN,
            "terminal_origin",
            "Open the terminal from the authorized local Cube Play application.",
        ));
    }
    if state.config.workspace.is_none() {
        return Err(unavailable());
    }
    let Query(selection) = query.map_err(|_| invalid())?;
    let location = Location {
        store: selection.store,
        path: selection.path,
    };
    let size = Size {
        cols: selection.cols,
        rows: selection.rows,
    };
    if !location.valid() || !size.valid() {
        return Err(invalid());
    }
    #[cfg(unix)]
    {
        let mut channel = connect(&state.config).await?;
        play_runner::transport::send(&mut channel, &Client::Start { location, size })
            .await
            .map_err(|_| unavailable())?;
        let first = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            play_runner::transport::receive::<Server>(&mut channel),
        )
        .await
        .map_err(|_| unavailable())?
        .map_err(|_| unavailable())?
        .ok_or_else(unavailable)?;
        match first {
            ready @ Server::Ready { .. } => Ok(websocket
                .protocols(["cube-play.v1"])
                .max_message_size(protocol::FRAME_LIMIT)
                .max_frame_size(protocol::FRAME_LIMIT)
                .write_buffer_size(8192)
                .max_write_buffer_size(65536)
                .on_upgrade(move |socket| forward(socket, channel, ready))),
            Server::Error { code } if code == "session_limit" => Err(FileError(
                StatusCode::TOO_MANY_REQUESTS,
                "terminal_busy",
                "Two terminal sessions are already active. Close one before starting another.",
            )),
            Server::Error { code } if code == "path_denied" => Err(FileError(
                StatusCode::FORBIDDEN,
                "terminal_path_denied",
                "The working directory is missing, symbolic or outside the authorized roots.",
            )),
            _ => Err(unavailable()),
        }
    }
    #[cfg(not(unix))]
    {
        let _ = (websocket, location, size);
        Err(unavailable())
    }
}
#[cfg(unix)]
async fn connect(config: &crate::config::Config) -> Result<play_runner::transport::Channel> {
    let path = config.runner_socket.as_ref().ok_or_else(unavailable)?;
    let metadata = tokio::fs::symlink_metadata(path)
        .await
        .map_err(|_| unavailable())?;
    use std::os::unix::fs::{FileTypeExt, PermissionsExt};
    if !metadata.file_type().is_socket() || metadata.permissions().mode() & 0o077 != 0 {
        return Err(unavailable());
    }
    let socket = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        tokio::net::UnixStream::connect(path),
    )
    .await
    .map_err(|_| unavailable())?
    .map_err(|_| unavailable())?;
    Ok(play_runner::transport::channel(socket))
}
#[cfg(unix)]
pub(crate) async fn probe(config: &crate::config::Config) -> Result<usize> {
    use play_runner::transport::{receive, send};
    let mut socket = connect(config).await?;
    send(&mut socket, &Client::Probe)
        .await
        .map_err(|_| unavailable())?;
    match tokio::time::timeout(
        std::time::Duration::from_secs(2),
        receive::<Server>(&mut socket),
    )
    .await
    .map_err(|_| unavailable())?
    .map_err(|_| unavailable())?
    {
        Some(Server::Status { active, limit })
            if active <= protocol::SESSION_LIMIT && limit == protocol::SESSION_LIMIT =>
        {
            Ok(active)
        }
        _ => Err(unavailable()),
    }
}
#[cfg(not(unix))]
pub(crate) async fn probe(_: &crate::config::Config) -> Result<usize> {
    Err(unavailable())
}
#[cfg(unix)]
async fn forward(
    mut socket: WebSocket,
    mut runner: play_runner::transport::Channel,
    ready: Server,
) {
    use play_runner::transport::{receive, send};
    let result = async {
        let ready = serde_json::to_string(&ready).map_err(|_| ())?;
        socket.send(Message::Text(ready.into())).await.map_err(|_| ())?;
        loop {
            tokio::select! {
                message = socket.recv() => match message {
                    Some(Ok(Message::Text(text))) => {
                        let frame: Client = serde_json::from_str(&text).map_err(|_| ())?;
                        match &frame {
                            Client::Input { data } if !data.is_empty() && data.len() <= protocol::INPUT_FRAME => {},
                            Client::Resize { size } if size.valid() => {},
                            Client::Close => {
                                send(&mut runner, &frame).await.map_err(|_| ())?;
                                break;
                            },
                            _ => return Err(()),
                        }
                        send(&mut runner, &frame).await.map_err(|_| ())?;
                    },
                    Some(Ok(Message::Ping(_) | Message::Pong(_))) => {},
                    _ => break,
                },
                message = receive::<Server>(&mut runner) => {
                    let Some(frame) = message.map_err(|_| ())? else { break; };
                    let done = matches!(&frame, Server::End { .. } | Server::Error { .. });
                    let message = match frame {
                        Server::Output { data } if data.len() <= 4096 => Message::Binary(data.into()),
                        other => Message::Text(serde_json::to_string(&other).map_err(|_| ())?.into()),
                    };
                    tokio::time::timeout(std::time::Duration::from_secs(2), socket.send(message))
                        .await.map_err(|_| ())?.map_err(|_| ())?;
                    if done { break; }
                }
            }
        }
        Ok::<_, ()>(())
    }.await;
    let _ = result;
    let _ = send(&mut runner, &Client::Close).await;
    let _ = socket.send(Message::Close(None)).await;
}
