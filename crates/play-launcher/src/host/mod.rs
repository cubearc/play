//! A separate host origin owns folder authorization and runtime startup.
mod control;
mod picker;
use crate::{
    Error, Result,
    runtime::{self, Options},
    state,
};
use axum::{
    Json, Router,
    body::Body,
    extract::{Request, State},
    http::{HeaderValue, Method, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
pub use control::{ensure, stop};
use serde::Deserialize;
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use subtle::ConstantTimeEq;
use tokio::sync::Semaphore;
use uuid::Uuid;

#[derive(Clone)]
struct Host {
    options: Options,
    home: PathBuf,
    control: control::Control,
    stopped: Arc<AtomicBool>,
    slots: Arc<Semaphore>,
    reads: Arc<Semaphore>,
    active: Arc<Mutex<Option<Arc<AtomicBool>>>>,
}
fn failure(error: Error) -> Response {
    let status = match error.code {
        "workspace_busy" => StatusCode::TOO_MANY_REQUESTS,
        "cancelled" => StatusCode::CONFLICT,
        "invalid_name" | "invalid_intent" => StatusCode::BAD_REQUEST,
        _ => StatusCode::CONFLICT,
    };
    (
        status,
        Json(json!({"schema_version":1,"error":{"code":error.code,"message":error.message}})),
    )
        .into_response()
}
async fn boundary(State(host): State<Host>, request: Request, next: Next) -> Response {
    let h = request.headers();
    let authority = format!("127.0.0.1:{}", host.control.port);
    let origin = format!("http://{authority}");
    let trusted = h.get_all(header::HOST).iter().count() == 1
        && h.get(header::HOST).and_then(|h| h.to_str().ok()) == Some(authority.as_str())
        && h.get_all(header::ORIGIN).iter().count() <= 1
        && h.get(header::ORIGIN)
            .is_none_or(|h| h.to_str().ok() == Some(origin.as_str()))
        && (matches!(*request.method(), Method::GET | Method::HEAD)
            || h.contains_key(header::ORIGIN))
        && h.get("sec-fetch-site").is_none_or(|h| h != "cross-site")
        && !["forwarded", "x-forwarded-host", "x-forwarded-proto"]
            .iter()
            .any(|k| h.contains_key(*k));
    let authenticated = h.get_all(header::AUTHORIZATION).iter().count() == 1
        && h.get(header::AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .is_some_and(|v| bool::from(v.as_bytes().ct_eq(host.control.secret.as_bytes())));
    let mut response = if request.uri().to_string().len() > 4096 {
        (
            StatusCode::URI_TOO_LONG,
            "The local request exceeds its URI limit",
        )
            .into_response()
    } else if !trusted {
        (
            StatusCode::FORBIDDEN,
            "Use the private local workspace chooser link",
        )
            .into_response()
    } else if request.uri().path().starts_with("/api/") && !authenticated {
        (StatusCode::UNAUTHORIZED,Json(json!({"schema_version":1,"error":{"code":"session_required","message":"Open the private workspace chooser link shown by cube play to authorize this tab."}}))).into_response()
    } else {
        next.run(request).await
    };
    for (name, value) in [
        (
            "content-security-policy",
            "default-src 'none'; script-src 'self'; style-src 'self'; connect-src 'self'; img-src 'self' data:; frame-ancestors 'none'; base-uri 'none'; form-action 'none'; object-src 'none'",
        ),
        ("referrer-policy", "no-referrer"),
        ("cache-control", "no-store"),
        ("x-content-type-options", "nosniff"),
        (
            "permissions-policy",
            "camera=(), microphone=(), geolocation=()",
        ),
    ] {
        response
            .headers_mut()
            .insert(name, HeaderValue::from_static(value));
    }
    response
}
fn assets(path: &str) -> Response {
    let (mime, data): (&str, &[u8]) = match path {
        "/" => ("text/html; charset=utf-8", include_bytes!("index.html")),
        "/launcher.js" => (
            "text/javascript; charset=utf-8",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../frontend/launcher-dist/launcher.js"
            )),
        ),
        "/launcher.css" => (
            "text/css; charset=utf-8",
            include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../frontend/launcher-dist/launcher.css"
            )),
        ),
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    ([(header::CONTENT_TYPE, mime)], Body::from(data)).into_response()
}
fn router(host: Host) -> Router {
    Router::new()
        .route("/", get(|| async { assets("/") }))
        .route("/launcher.js", get(|| async { assets("/launcher.js") }))
        .route("/launcher.css", get(|| async { assets("/launcher.css") }))
        .route("/healthz", get(|| async { Json(json!({"status":"ok"})) }))
        .route(
            "/api/health",
            get(|State(host): State<Host>| async move {
                Json(json!({"schema_version":1,"instance":host.control.instance}))
            }),
        )
        .route("/api/workspaces", get(recent))
        .route("/api/intents", post(intent))
        .route("/api/shutdown", post(shutdown))
        .layer(axum::extract::DefaultBodyLimit::max(4096))
        .layer(middleware::from_fn_with_state(host.clone(), boundary))
        .with_state(host)
}
async fn recent(State(host): State<Host>) -> Response {
    let Ok(permit) = host.reads.clone().try_acquire_owned() else {
        return failure(Error::new(
            "workspace_busy",
            "Recent workspaces are already being read. Retry shortly.",
        ));
    };
    let result = tokio::task::spawn_blocking(move || -> Result<Value> {
        let _permit = permit;
        let rows = state::recent_workspaces(&host.home, &host.stopped)?
            .into_iter()
            .map(|w| {
                let available = w.root.canonicalize().is_ok_and(|root| root == w.root)
                    && w.root.is_dir()
                    && std::fs::read_dir(&w.root).is_ok();
                json!({"id":w.id,"name":w.name,"available":available})
            })
            .collect::<Vec<_>>();
        Ok(json!({"schema_version":1,"workspaces":rows}))
    })
    .await;
    match result {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(error)) => failure(error),
        Err(_) => failure(Error::new(
            "state_unavailable",
            "Recent workspaces could not be read; existing state was preserved",
        )),
    }
}
#[derive(Deserialize)]
#[serde(tag = "intent", rename_all = "snake_case", deny_unknown_fields)]
enum Intent {
    Open { schema_version: u32 },
    New { schema_version: u32, name: String },
    Recent { schema_version: u32, id: Uuid },
}
impl Intent {
    fn version(&self) -> u32 {
        match self {
            Self::Open { schema_version }
            | Self::New { schema_version, .. }
            | Self::Recent { schema_version, .. } => *schema_version,
        }
    }
}
struct Cancel(Arc<AtomicBool>);
impl Drop for Cancel {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Relaxed);
    }
}
struct Active(Host);
impl Drop for Active {
    fn drop(&mut self) {
        if let Ok(mut active) = self.0.active.lock() {
            *active = None;
        }
    }
}
async fn intent(State(host): State<Host>, request: Request) -> Response {
    if request
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(';').next())
        != Some("application/json")
    {
        return failure(Error::new("invalid_intent", "Use a JSON workspace intent"));
    }
    let bytes=match tokio::time::timeout(std::time::Duration::from_secs(5),axum::body::to_bytes(request.into_body(),4096)).await {
        Ok(Ok(bytes))=>bytes,
        Ok(Err(_))=>return failure(Error::new("invalid_intent","Workspace intents are limited to 4096 bytes")),
        Err(_)=>return (StatusCode::REQUEST_TIMEOUT,Json(json!({"schema_version":1,"error":{"code":"request_timeout","message":"The workspace intent body did not arrive within five seconds."}}))).into_response(),
    };
    let intent: Intent = match serde_json::from_slice(&bytes) {
        Ok(intent) => intent,
        Err(_) => {
            return failure(Error::new(
                "invalid_intent",
                "Use a versioned workspace intent without unknown fields, duplicate keys or host paths",
            ));
        }
    };
    if intent.version() != 1 {
        return failure(Error::new(
            "invalid_intent",
            "Use workspace intent protocol 1",
        ));
    }
    if let Intent::New { name, .. } = &intent
        && let Err(error) = picker::leaf(name)
    {
        return failure(error);
    }
    let Ok(permit) = host.slots.clone().try_acquire_owned() else {
        return failure(Error::new(
            "workspace_busy",
            "Another workspace action is in progress. Finish it before starting another.",
        ));
    };
    let cancelled = Arc::new(AtomicBool::new(false));
    if let Ok(mut active) = host.active.lock() {
        *active = Some(cancelled.clone());
    } else {
        return failure(Error::io(std::io::Error::other("Host state unavailable")));
    }
    let _cancel = Cancel(cancelled.clone());
    let task=tokio::task::spawn_blocking(move||->Result<Value>{
        let _permit=permit;let _active=Active(host.clone());
        let root=match intent {
            Intent::Recent{id,..}=>{
                let root=state::recent_workspaces(&host.home,&cancelled)?.into_iter().find(|w|w.id==id).ok_or_else(||Error::new("workspace_missing","This recent workspace is not registered in this launcher"))?.root;
                if !root.canonicalize().is_ok_and(|current|current==root) {return Err(Error::new("source_unavailable","This recent source moved, is unavailable or now resolves elsewhere. Use Open workspace to authorize a directory."));}
                root
            },
            Intent::Open{..}=>match picker::choose(&host.home,false,cancelled.clone())?{Some(path)=>path,None=>return Ok(json!({"schema_version":1,"cancelled":true}))},
            Intent::New{name,..}=>{
                let Some(parent)=picker::choose(&host.home,true,cancelled.clone())? else{return Ok(json!({"schema_version":1,"cancelled":true}));};
                let root=parent.join(name);
                if root.starts_with(&host.home)||host.home.starts_with(&root){return Err(Error::new("unsafe_source","Choose a source directory separate from the private launcher state"));}
                let mut builder=std::fs::DirBuilder::new();
                #[cfg(unix)]{use std::os::unix::fs::DirBuilderExt;builder.mode(0o700);}
                builder.create(&root).map_err(|e|if e.kind()==std::io::ErrorKind::AlreadyExists{Error::new("folder_exists","This folder already exists. Use Open workspace or choose a different name; existing files were preserved.")}else{Error::io(e)})?;
                root
            }
        };
        let mut options=host.options.clone();options.path=Some(root);options.stop=false;options.install_runtime=false;options.json_errors=true;
        let opened=runtime::run(&options,cancelled)?.ok_or_else(||Error::new("startup_failed","The workspace did not report readiness"))?;
        Ok(json!({"schema_version":1,"url":opened.url}))
    }).await;
    match task {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(error)) => failure(error),
        Err(_) => failure(Error::new(
            "startup_failed",
            "The workspace action stopped unexpectedly; existing data was preserved",
        )),
    }
}
async fn shutdown(State(host): State<Host>) -> Response {
    if host.active.lock().map_or(true, |a| a.is_some()) {
        return failure(Error::new(
            "workspace_busy",
            "Finish the current workspace action before stopping the chooser",
        ));
    }
    host.stopped.store(true, Ordering::Relaxed);
    StatusCode::NO_CONTENT.into_response()
}
pub fn serve(options: Options, stopped: Arc<AtomicBool>) -> Result<()> {
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .max_blocking_threads(4)
        .enable_all()
        .build()
        .map_err(Error::io)?;
    runtime.block_on(async move {
        let home = runtime::home(&options)?;
        let _instance = state::lock(&home.join("chooser-instance.lock"), &stopped)?;
        let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .map_err(Error::io)?;
        let control =
            control::Control::new(listener.local_addr().map_err(Error::io)?.port(), &options)?;
        state::write_json(&home.join("chooser.json"), &control)?;
        let host = Host {
            options,
            home,
            control,
            stopped: stopped.clone(),
            slots: Arc::new(Semaphore::new(1)),
            reads: Arc::new(Semaphore::new(2)),
            active: Arc::new(Mutex::new(None)),
        };
        let active = host.active.clone();
        axum::serve(listener, router(host))
            .with_graceful_shutdown(async move {
                while !stopped.load(Ordering::Relaxed) {
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                }
                if let Ok(a) = active.lock()
                    && let Some(a) = a.as_ref()
                {
                    a.store(true, Ordering::Relaxed);
                }
            })
            .await
            .map_err(Error::io)
    })
}
