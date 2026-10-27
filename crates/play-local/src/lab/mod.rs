//! Typed local execution with one bounded, cancellable Core worker process.
mod candidates;
pub(crate) mod capture;
mod domains;
mod execute;
mod library;
pub(crate) mod projection;
mod replay;
mod solutions;
mod storage;
mod typed;
pub(crate) mod types;
mod worker;
pub use worker::main as worker_main;

use crate::{AppState, workspace};
use axum::{
    Json, Router,
    body::Body,
    extract::{Path, State as AxumState},
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::{delete, get, post},
};
use serde_json::{Value, json};
use std::{
    process::Stdio,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{OnceCell, Semaphore, oneshot, watch},
};
use types::*;
use uuid::Uuid;

type Active = Option<(Uuid, watch::Sender<bool>, bool)>;
pub(crate) struct State {
    slots: Arc<Semaphore>,
    active: Mutex<Active>,
    storage: OnceCell<Result<Arc<storage::Storage>>>,
}
impl Default for State {
    fn default() -> Self {
        Self {
            slots: Arc::new(Semaphore::new(1)),
            active: Mutex::new(None),
            storage: OnceCell::new(),
        }
    }
}
impl State {
    pub(crate) async fn storage(&self, app: &AppState) -> Result<Arc<storage::Storage>> {
        self.storage
            .get_or_init(|| async {
                app.database().await?;
                let config = app.config.clone();
                tokio::task::spawn_blocking(move || storage::Storage::open(&config).map(Arc::new))
                    .await
                    .map_err(|_| Error::storage())?
            })
            .await
            .clone()
    }
}
pub(crate) fn routes() -> Router<AppState> {
    Router::new()
        .merge(library::routes())
        .merge(candidates::routes())
        .merge(capture::routes())
        .merge(solutions::routes())
        .merge(typed::routes())
        .route("/lab/operations", get(catalog))
        .route("/lab/operations/{id}", get(describe))
        .route(
            "/lab/execute",
            post(execution).layer(tower_http::limit::RequestBodyLimitLayer::new(
                cube_core::pipeline::json::MAX_DOCUMENT_BYTES,
            )),
        )
        .route("/lab/executions/{id}", delete(cancel))
        .route("/lab/results/{id}", get(result))
        .route("/lab/results/{id}/content", get(content))
        .route("/lab/results/{id}/replay", get(replay::inspect))
}
async fn catalog() -> Json<Value> {
    Json(
        json!({"schema_version":1,"core_version":execute::core_version(),"engine_api":cube_core::pipeline::ENGINE_API,"lab_adapter_api":ADAPTER_API,
        "limits":{"inline_input_bytes":MAX_INLINE,"file_input_bytes":MAX_INPUT,"output_bytes":MAX_OUTPUT,"inline_result_bytes":INLINE_RESULT,"concurrent_operations":1,"timeout_ms":TIMEOUT_MS,"pipeline":execute::pipeline_limits(MAX_OUTPUT)},
        "operations":cube_core::operations::catalog().into_iter().map(execute::discovery).collect::<Vec<_>>()}),
    )
}
async fn describe(Path(id): Path<String>) -> Result<Json<Value>> {
    Ok(Json(execute::discovery(execute::descriptor(&id)?)))
}
pub(crate) async fn request_json(request: axum::extract::Request) -> Result<Value> {
    if request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        != Some("application/json")
    {
        return Err(Error::invalid());
    }
    let body = tokio::time::timeout(
        Duration::from_millis(TIMEOUT_MS),
        axum::body::to_bytes(
            request.into_body(),
            cube_core::pipeline::json::MAX_DOCUMENT_BYTES,
        ),
    )
    .await
    .map_err(|_| {
        Error::new(
            "timeout",
            "The request body did not arrive within five seconds.",
        )
    })?
    .map_err(|_| Error::limit())?;
    let document = std::str::from_utf8(&body).map_err(|_| Error::invalid())?;
    let parsed = cube_core::pipeline::json::parse(document)?;
    Ok(parsed)
}
struct CancelOnDrop(watch::Sender<bool>);
impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        let _ = self.0.send(true);
    }
}
struct ActiveGuard {
    state: Arc<State>,
    id: Uuid,
}
impl Drop for ActiveGuard {
    fn drop(&mut self) {
        if let Ok(mut active) = self.state.active.lock()
            && active.as_ref().is_some_and(|(id, _, _)| *id == self.id)
        {
            *active = None;
        }
    }
}

async fn execution(
    AxumState(app): AxumState<AppState>,
    request: axum::extract::Request,
) -> Result<Json<Value>> {
    let parsed = request_json(request).await?;
    let request: Request = serde_json::from_value(parsed).map_err(|_| Error::invalid())?;
    execute::validate(&request)?;
    let storage = app.lab.storage(&app).await?;
    let permit = app.lab.slots.clone().try_acquire_owned().map_err(|_| {
        Error::new(
            "busy",
            "One Lab operation is already active. Cancel it or retry after completion.",
        )
    })?;
    let (cancel, rx) = watch::channel(false);
    *app.lab.active.lock().map_err(|_| Error::storage())? =
        Some((request.id, cancel.clone(), false));
    let _cancel = CancelOnDrop(cancel);
    let (reply, done) = oneshot::channel();
    tokio::spawn(async move {
        let _permit = permit;
        let _active = ActiveGuard {
            state: app.lab.clone(),
            id: request.id,
        };
        let result = job(app, storage, request, rx).await;
        let _ = reply.send(result);
    });
    Ok(Json(done.await.map_err(|_| {
        Error::new(
            "worker_unavailable",
            "The local operation worker stopped unexpectedly.",
        )
    })??))
}
async fn cancel(AxumState(app): AxumState<AppState>, Path(id): Path<String>) -> Result<StatusCode> {
    let id = reference_id(&id)?;
    let active = app.lab.active.lock().map_err(|_| Error::storage())?;
    let Some((_, cancel, committing)) = active.as_ref().filter(|(active, _, _)| *active == id)
    else {
        return Err(Error::new(
            "result_missing",
            "This execution is not active in this workspace.",
        ));
    };
    if *committing {
        return Err(Error::new(
            "commit_started",
            "The result is being published. Read it using this execution ID.",
        ));
    }
    let _ = cancel.send(true);
    Ok(StatusCode::ACCEPTED)
}
async fn input(
    app: &AppState,
    storage: Arc<storage::Storage>,
    input: Input,
) -> Result<(Data, InputRecord)> {
    if let Input::File {
        store: workspace::Store::Imports,
        path,
        ..
    } = &input
        && let Some(database) = app.database().await?
    {
        database.authorize_import(path).await?;
    }
    let workspace = app.workspace.as_ref().map_err(|e| Error::from(*e))?.clone();
    tokio::task::spawn_blocking(move || {
        let (data, reference) = match input {
            Input::Text { data } => {
                if data.len() > MAX_INLINE {
                    return Err(Error::limit());
                }
                (
                    Data::new(Representation::Text, "text", data.as_bytes(), Value::Null),
                    json!({"type":"inline"}),
                )
            }
            Input::Bytes { base64 } => {
                let data = Data {
                    representation: Representation::Bytes,
                    semantic_type: "bytes".into(),
                    base64,
                    metadata: Value::Null,
                };
                data.bytes(MAX_INLINE)?;
                (data, json!({"type":"inline"}))
            }
            Input::Artifact { id } => {
                let (record, bytes) = storage.load(id)?;
                (
                    Data::new(
                        record.representation,
                        &record.semantic_type,
                        &bytes,
                        record.metadata,
                    ),
                    json!({"type":"artifact","id":id}),
                )
            }
            Input::File {
                store,
                path,
                representation,
            } => {
                if representation == Representation::Structured {
                    return Err(Error::invalid());
                }
                let file = workspace
                    .as_ref()
                    .ok_or_else(Error::invalid)?
                    .file(store, &path)?;
                let mut bytes = Vec::new();
                use std::io::Read;
                file.take(MAX_INPUT as u64 + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|_| Error::invalid())?;
                if bytes.len() > MAX_INPUT {
                    return Err(Error::limit());
                }
                let semantic = match representation {
                    Representation::Text => {
                        std::str::from_utf8(&bytes).map_err(|_| Error::invalid())?;
                        "text"
                    }
                    Representation::Image => "image.samples",
                    _ => "bytes",
                };
                (
                    Data::new(representation, semantic, &bytes, Value::Null),
                    json!({"type":"file","store":store,"path":path}),
                )
            }
        };
        let bytes = data.bytes(MAX_INPUT)?;
        let record = InputRecord {
            reference,
            semantic_type: data.semantic_type.clone(),
            byte_size: bytes.len(),
            sha256: cube_core::hashing::sha256(&bytes),
        };
        Ok((data, record))
    })
    .await
    .map_err(|_| Error::invalid())?
}
async fn job(
    app: AppState,
    storage: Arc<storage::Storage>,
    request: Request,
    mut cancellation: watch::Receiver<bool>,
) -> Result<Value> {
    let (data, first) = input(&app, storage.clone(), request.input.clone()).await?;
    let mut inputs = vec![first];
    let other = if let Some(other) = request.other.clone() {
        let (data, record) = input(&app, storage.clone(), other).await?;
        inputs.push(record);
        Some(data)
    } else {
        None
    };
    if *cancellation.borrow() {
        return Err(Error::cancelled());
    }
    let wire = execute::json_bytes(
        &WorkerRequest {
            request: request.clone(),
            input: data,
            other,
        },
        MAX_WIRE,
    )?;
    let output = worker_output(wire, request.timeout_ms, &mut cancellation).await?;
    if *cancellation.borrow() {
        return Err(Error::cancelled());
    }
    let provenance = Provenance {
        operation: request.operation.clone(),
        adapter_api: ADAPTER_API,
        pipeline_adapter: cube_core::pipeline::adapter_version(&request.operation),
        engine_api: cube_core::pipeline::ENGINE_API,
        core_version: execute::core_version(),
        parameters: output.parameters.clone(),
        inputs,
        created_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| Error::invalid())?
            .as_millis()
            .try_into()
            .map_err(|_| Error::invalid())?,
    };
    let id = request.id;
    if !request.retain_result {
        let bytes = output.data.bytes(MAX_OUTPUT)?;
        let record = storage.record(id, output, provenance, &bytes)?;
        return response(record, &bytes, false);
    }
    {
        let mut active = app.lab.active.lock().map_err(|_| Error::storage())?;
        if *cancellation.borrow() {
            return Err(Error::cancelled());
        }
        if let Some((_, _, committing)) = active.as_mut().filter(|(active, _, _)| *active == id) {
            *committing = true;
        }
    }
    tokio::task::spawn_blocking(move || {
        let record = storage.save(id, output, provenance)?;
        let (_, bytes) = storage.load(id)?;
        response(record, &bytes, true)
    })
    .await
    .map_err(|_| Error::storage())?
}
async fn worker_output(
    wire: Vec<u8>,
    timeout_ms: u64,
    cancellation: &mut watch::Receiver<bool>,
) -> Result<WorkerOutput> {
    let executable = std::env::current_exe().map_err(|_| Error::invalid())?;
    let mut child = tokio::process::Command::new(executable)
        .arg("--lab-worker")
        .env_clear()
        .current_dir(std::env::temp_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| {
            Error::new(
                "worker_unavailable",
                "Could not start the bounded local Core worker.",
            )
        })?;
    let mut stdin = child.stdin.take().ok_or_else(Error::invalid)?;
    let stdout = child.stdout.take().ok_or_else(Error::invalid)?;
    let run = async {
        stdin.write_all(&wire).await.map_err(|_| Error::limit())?;
        drop(stdin);
        let mut bytes = Vec::new();
        stdout
            .take(MAX_WIRE as u64 + 1)
            .read_to_end(&mut bytes)
            .await
            .map_err(|_| Error::limit())?;
        if bytes.len() > MAX_WIRE {
            return Err(Error::limit());
        }
        let status = child.wait().await.map_err(|_| Error::limit())?;
        if !status.success() {
            return Err(Error::limit());
        }
        serde_json::from_slice::<Result<WorkerOutput>>(&bytes).map_err(|_| {
            Error::new(
                "worker_unavailable",
                "The worker returned an invalid structured response.",
            )
        })?
    };
    let output = tokio::select! {
        value=tokio::time::timeout(Duration::from_millis(timeout_ms),run)=>value.unwrap_or_else(|_|Err(Error::new("timeout","The operation reached its wall-clock limit; its worker was terminated."))),
        _=cancellation.changed()=>Err(Error::cancelled()),
    };
    let _ = child.kill().await;
    let _ = child.wait().await;
    output
}
fn response(record: Record, bytes: &[u8], retained: bool) -> Result<Value> {
    let inline = if !retained || bytes.len() <= INLINE_RESULT {
        Some(match record.representation {
            Representation::Text => {
                json!({"type":"text","data":std::str::from_utf8(bytes).map_err(|_|Error::storage())?})
            }
            Representation::Structured => {
                json!({"type":"structured","data":serde_json::from_slice::<Value>(bytes).map_err(|_|Error::storage())?})
            }
            _ => json!({"type":"bytes","base64":cube_core::encoding::base64_encode(bytes)}),
        })
    } else {
        None
    };
    Ok(
        json!({"schema_version":1,"retained":retained,"content_url":retained.then(||format!("/api/lab/results/{}/content",record.id)),"result":record,"inline":inline}),
    )
}
async fn result(
    AxumState(app): AxumState<AppState>,
    Path(id): Path<String>,
) -> Result<Json<Value>> {
    let id = reference_id(&id)?;
    let _permit = read_permit(&app).await?;
    let storage = app.lab.storage(&app).await?;
    let value = tokio::task::spawn_blocking(move || {
        let (record, data) = storage.load(id)?;
        response(record, &data, true)
    })
    .await
    .map_err(|_| Error::storage())??;
    Ok(Json(value))
}
async fn content(AxumState(app): AxumState<AppState>, Path(id): Path<String>) -> Result<Response> {
    let id = reference_id(&id)?;
    let permit = read_permit(&app).await?;
    let storage = app.lab.storage(&app).await?;
    let (record, bytes) = tokio::task::spawn_blocking(move || storage.load(id))
        .await
        .map_err(|_| Error::storage())??;
    let mime = match record.representation {
        Representation::Text => "text/plain; charset=utf-8",
        Representation::Image => "image/png",
        Representation::Structured => "application/json",
        Representation::Bytes => "application/octet-stream",
    };
    use futures_util::StreamExt;
    let stream = tokio_util::io::ReaderStream::with_capacity(std::io::Cursor::new(bytes), 65536)
        .map(move |chunk| {
            let _ = &permit;
            chunk
        });
    Ok((
        [
            (header::CONTENT_TYPE, mime),
            (header::CONTENT_DISPOSITION, "inline"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        Body::from_stream(stream),
    )
        .into_response())
}

async fn read_permit(app: &AppState) -> Result<tokio::sync::OwnedSemaphorePermit> {
    let workspace = app
        .workspace
        .as_ref()
        .map_err(|e| Error::from(*e))?
        .as_ref()
        .ok_or_else(Error::invalid)?;
    crate::workspace::permit(workspace)
        .await
        .map_err(Error::from)
}

fn reference_id(value: &str) -> Result<Uuid> {
    let id = Uuid::parse_str(value).map_err(|_| Error::invalid())?;
    if value.len() != 36 || id.get_version_num() != 4 {
        return Err(Error::invalid());
    }
    Ok(id)
}
