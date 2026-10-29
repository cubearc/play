mod assets;
mod blocks;
mod copies;
mod model;
mod mutations;
mod order;
mod queries;
mod run_plan;
mod runs;
mod storage;
mod upgrade;

use crate::{Result, protocol};
use model::Identity;
pub use model::{Command, Context, RunRequest};
pub use runs::{PreparedRun, RunIdentity};
use serde_json::{Value, json};
use sqlx::{Connection, Row, SqliteConnection};
use std::{
    fs,
    path::{Path, PathBuf},
};
use storage::Owned;
use tokio::runtime::Runtime;
use uuid::Uuid;

pub struct Manager {
    root: PathBuf,
    runtime: Runtime,
    expected: storage::Schemas,
    active: Option<Owned>,
}

impl Manager {
    pub fn new(root: PathBuf) -> Result<Self> {
        if !root.is_absolute() {
            return Err("unsafe_store_path");
        }
        storage::private_directory(&root)?;
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| "storage_unavailable")?;
        let expected = runtime.block_on(storage::expected_schema())?;
        Ok(Self {
            root,
            runtime,
            expected,
            active: None,
        })
    }

    fn id_directory(&self, id: Uuid) -> PathBuf {
        self.root.join(id.to_string())
    }

    fn list(&self) -> Result<Value> {
        let mut workspaces = Vec::new();
        for entry in fs::read_dir(&self.root).map_err(|_| "storage_unavailable")? {
            let entry = entry.map_err(|_| "storage_unavailable")?;
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let Ok(id) = Uuid::parse_str(&name) else {
                continue;
            };
            if name != id.to_string() {
                continue;
            }
            let row = match storage::identity(&entry.path(), id) {
                Ok(identity) => {
                    json!({"id":id,"title":identity.title,"revision":1,"available":true})
                }
                Err(code) => {
                    json!({"id":id,"title":"Unavailable workspace","revision":1,"available":false,"error":code})
                }
            };
            workspaces.push(row);
            if workspaces.len() > protocol::contract().workspace_limit {
                return Err("storage_quota");
            }
        }
        workspaces.sort_by(|a, b| a["title"].as_str().cmp(&b["title"].as_str()));
        let recent = self.root.join("recent.json");
        let last = if recent.exists() {
            storage::read_json(&recent)?["workspace_id"]
                .as_str()
                .and_then(|value| Uuid::parse_str(value).ok())
        } else {
            None
        };
        Ok(json!({"workspaces":workspaces,"lastWorkspaceId":last}))
    }

    fn activate(&mut self, id: Uuid) -> Result<Value> {
        if self
            .active
            .as_ref()
            .is_some_and(|store| store.identity.workspace_id == id)
        {
            return self
                .runtime
                .block_on(queries::view(
                    self.active.as_mut().ok_or("workspace_closed")?,
                ))
                .and_then(to_value);
        }
        let mut next =
            self.runtime
                .block_on(Owned::open(self.id_directory(id), id, &self.expected))?;
        if let Some(mut previous) = self.active.take()
            && let Err(code) = self.runtime.block_on(previous.close())
        {
            let _ = self.runtime.block_on(next.close());
            return Err(code);
        }
        self.active = Some(next);
        storage::atomic_json(&self.root.join("recent.json"), &json!({"workspace_id":id}))?;
        self.runtime
            .block_on(queries::view(
                self.active.as_mut().ok_or("workspace_closed")?,
            ))
            .and_then(to_value)
    }

    fn create(&mut self, key: Uuid, name: &str) -> Result<Value> {
        let digest = cube_core::hashing::sha256(name.as_bytes());
        let name = mutations::title(name)?;
        let _creation_lock = storage::lock(&self.root.join("creation.lock"), true)?;
        let listing = self.list()?;
        let entries = listing["workspaces"].as_array().ok_or("corrupt_store")?;
        for entry in entries {
            let id = Uuid::parse_str(entry["id"].as_str().ok_or("corrupt_store")?)
                .map_err(|_| "corrupt_store")?;
            if let Ok(identity) = storage::identity(&self.id_directory(id), id)
                && identity.creation_key == key
            {
                if identity.creation_digest != digest {
                    return Err("idempotency_conflict");
                }
                return self.activate(id);
            }
        }
        let interrupted = fs::read_dir(&self.root)
            .map_err(|_| "storage_unavailable")?
            .filter_map(|entry| entry.ok())
            .filter(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".creating-")
            })
            .count();
        if entries.len() + interrupted >= protocol::contract().workspace_limit {
            return Err("storage_quota");
        }
        let id = Uuid::new_v4();
        let temporary = self.root.join(format!(".creating-{id}"));
        storage::private_directory(&temporary)?;
        let identity = Identity {
            product: "cubearc.play.workspace".into(),
            generation: storage::GENERATION,
            workspace_id: id,
            title: name.into(),
            creation_key: key,
            creation_digest: digest,
        };
        let result = self.runtime.block_on(initialize(&temporary, &identity));
        if let Err(code) = result {
            // Only this unpublished, newly created initialization directory is ours to discard.
            let _ = fs::remove_dir_all(&temporary);
            return Err(code);
        }
        fs::rename(&temporary, self.id_directory(id)).map_err(|_| "storage_unavailable")?;
        storage::sync_directory(&self.root)?;
        self.activate(id)
    }

    pub fn handle(&mut self, command: Command) -> Result<Value> {
        match command {
            Command::List => self.list(),
            Command::View { workspace_id } => {
                let store = self.active.as_mut().ok_or("workspace_closed")?;
                if store.identity.workspace_id != workspace_id {
                    return Err("workspace_mismatch");
                }
                self.runtime
                    .block_on(queries::view(store))
                    .and_then(to_value)
            }
            Command::Create { key, title } => self.create(key, &title),
            Command::Open { workspace_id } => self.activate(workspace_id),
            Command::OpenExploration {
                workspace_id,
                exploration_id,
                stage_id,
            } => {
                let store = self.active.as_mut().ok_or("workspace_closed")?;
                if store.identity.workspace_id != workspace_id {
                    return Err("workspace_mismatch");
                }
                self.runtime
                    .block_on(queries::exploration(store, exploration_id, stage_id))
                    .and_then(to_value)
            }
            Command::ListRuns {
                context,
                trail_id,
                before,
            } => {
                let store = self.active.as_mut().ok_or("workspace_closed")?;
                self.runtime
                    .block_on(runs::list(store, &context, trail_id, before))
            }
            Command::RunDetail { context, run_id } => self.run_detail(&context, run_id),
            Command::ReadImage {
                context,
                asset_id,
                revision,
            } => {
                let store = self.active.as_mut().ok_or("workspace_closed")?;
                self.runtime
                    .block_on(read_image(store, &context, asset_id, revision))
            }
            command => {
                let store = self.active.as_mut().ok_or("workspace_closed")?;
                self.runtime.block_on(mutations::apply(store, &command))
            }
        }
    }

    pub fn existing_run(&mut self, request: &RunRequest) -> Result<Option<Value>> {
        let store = self.active.as_mut().ok_or("workspace_closed")?;
        self.runtime.block_on(runs::existing(store, request))
    }
    pub fn prepare_run(&mut self, request: &RunRequest) -> Result<PreparedRun> {
        let store = self.active.as_mut().ok_or("workspace_closed")?;
        self.runtime.block_on(runs::prepare(store, request))
    }
    pub fn start_attempt(&mut self, run: &RunIdentity) -> Result<()> {
        let store = self.active.as_mut().ok_or("workspace_closed")?;
        self.runtime.block_on(runs::started(store, run))
    }
    pub fn finish_run(&mut self, run: &RunIdentity, result: Value) -> Result<Value> {
        let store = self.active.as_mut().ok_or("workspace_closed")?;
        self.runtime
            .block_on(runs::finish(store, run, result))
            .and_then(to_value)
    }
    pub fn run_detail(&mut self, context: &Context, id: Uuid) -> Result<Value> {
        let store = self.active.as_mut().ok_or("workspace_closed")?;
        self.runtime
            .block_on(runs::detail(store, context, id))
            .and_then(to_value)
    }
}

impl Drop for Manager {
    fn drop(&mut self) {
        if let Some(mut active) = self.active.take() {
            let _ = self.runtime.block_on(active.close());
        }
    }
}

fn to_value(value: impl serde::Serialize) -> Result<Value> {
    let value = serde_json::to_value(value).map_err(|_| "invalid_response")?;
    if serde_json::to_vec(&value)
        .map_err(|_| "invalid_response")?
        .len()
        + 128
        > protocol::contract().frame_bytes
    {
        return Err("output_limit");
    }
    Ok(value)
}

async fn initialize(directory: &Path, identity: &Identity) -> Result<()> {
    storage::create_file(&directory.join("owner.lock"), b"")?;
    storage::create_file(&directory.join("workspace.sqlite3"), b"")?;
    let mut connection =
        SqliteConnection::connect_with(&storage::options(&directory.join("workspace.sqlite3")))
            .await
            .map_err(storage::failure)?;
    let initialized: Result<()> = async {
        sqlx::query("PRAGMA max_page_count=4096")
            .execute(&mut connection)
            .await
            .map_err(storage::failure)?;
        sqlx::raw_sql(storage::SCHEMA)
            .execute(&mut connection)
            .await
            .map_err(storage::failure)?;
        sqlx::query("INSERT INTO workspace_meta VALUES(?,2,1)")
            .bind(identity.workspace_id.to_string())
            .execute(&mut connection)
            .await
            .map_err(storage::failure)?;
        storage::migrate(&mut connection).await
    }
    .await;
    connection.close().await.map_err(storage::failure)?;
    initialized?;
    storage::create_file(
        &directory.join("workspace.json"),
        &serde_json::to_vec(identity).map_err(|_| "invalid_state")?,
    )?;
    storage::sync_directory(directory)
}

async fn read_image(
    store: &mut Owned,
    context: &Context,
    asset_id: Uuid,
    revision: i64,
) -> Result<Value> {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let owner = store.identity.workspace_id;
    let connection = store.connection()?;
    queries::context(connection, owner, context).await?;
    let row=sqlx::query("SELECT r.data,r.digest FROM asset_revisions r JOIN assets a ON (a.workspace_id,a.exploration_id,a.id)=(r.workspace_id,r.exploration_id,r.asset_id) WHERE r.workspace_id=? AND r.exploration_id=? AND r.asset_id=? AND r.revision=? AND a.kind='image'")
        .bind(owner.to_string()).bind(context.exploration_id.to_string()).bind(asset_id.to_string()).bind(revision).fetch_optional(connection).await.map_err(storage::failure)?.ok_or("not_found")?;
    let bytes: Vec<u8> = row.get(0);
    if cube_core::hashing::sha256(&bytes) != row.get::<String, _>(1) {
        return Err("corrupt_store");
    }
    if bytes.len() > protocol::contract().image_bytes {
        return Err("storage_quota");
    }
    Ok(json!({"mime":"image/png","base64":STANDARD.encode(bytes)}))
}
