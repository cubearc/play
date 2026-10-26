//! One local investigation per authorized workspace; binary content stays on disk.
mod candidates;
mod challenges;
mod http;
pub(crate) mod library;
mod paths;
mod records;

use crate::{
    config::Config,
    workspace::{FileError, Result, Workspace},
};
use axum::http::StatusCode;
use serde::Serialize;
use sqlx::{
    ConnectOptions, Connection, Row, SqliteConnection, SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous},
};
use std::{
    fs::File,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::sync::OnceCell;
use uuid::Uuid;

pub(crate) use http::routes;
pub(crate) use records::{LocalStatus, Progress, View};
static MIGRATIONS: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");
const APPLICATION_ID: i64 = 1129335897;

pub(crate) fn failed() -> FileError {
    FileError(
        StatusCode::SERVICE_UNAVAILABLE,
        "storage_unavailable",
        "Local state could not be read or saved. Check storage permissions and free space, then reopen the workspace. Existing data was preserved; do not delete the database.",
    )
}
fn invalid() -> FileError {
    FileError(
        StatusCode::BAD_REQUEST,
        "invalid_state",
        "Use bounded, valid investigation fields and relative workspace paths.",
    )
}
fn conflict() -> FileError {
    FileError(
        StatusCode::CONFLICT,
        "state_conflict",
        "This record changed in another request. Reload the investigation before saving again.",
    )
}
impl From<sqlx::Error> for FileError {
    fn from(_: sqlx::Error) -> Self {
        failed()
    }
}
fn now() -> Result<i64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|v| v.as_millis().try_into().ok())
        .ok_or_else(failed)
}

#[derive(Clone, Default)]
pub struct PersistenceHandle(Arc<OnceCell<Result<Option<Arc<Database>>>>>);
impl PersistenceHandle {
    pub(crate) async fn get(
        &self,
        config: &Config,
        workspace: Option<&Arc<Workspace>>,
    ) -> Result<Option<Arc<Database>>> {
        self.0
            .get_or_init(|| async {
                let (Some(context), Some(imports), Some(workspace)) =
                    (&config.workspace, &config.data_dir, workspace)
                else {
                    return Ok(None);
                };
                let directory = config
                    .state_dir
                    .clone()
                    .or_else(|| imports.parent().map(|p| p.join("state")))
                    .ok_or_else(failed)?;
                let (path, lock) = paths::prepare(
                    &directory,
                    config.source_dir.as_deref().ok_or_else(failed)?,
                    imports,
                )?;
                Database::open(path, lock, context.id, &context.name, workspace)
                    .await
                    .map(|db| Some(Arc::new(db)))
            })
            .await
            .clone()
    }
    /// Called after HTTP requests drain. An abrupt exit is recorded on next open.
    pub async fn close(&self) -> Result<()> {
        if let Some(Ok(Some(db))) = self.0.get() {
            sqlx::query(
                "UPDATE sessions SET status='closed', ended_ms=? WHERE id=? AND status='running'",
            )
            .bind(now()?)
            .bind(&db.session_id)
            .execute(&db.pool)
            .await?;
            db.pool.close().await;
        }
        Ok(())
    }
}

pub(crate) struct Database {
    pool: SqlitePool,
    _lock: File,
    investigation_id: String,
    session_id: String,
}

#[derive(Serialize)]
pub(crate) struct Snapshot {
    id: String,
    workspace_id: String,
    title: String,
    revision: i64,
    view: View,
    view_revision: i64,
    session_id: String,
    interrupted_sessions: i64,
    warnings: Vec<&'static str>,
}
impl Database {
    async fn open(
        path: std::path::PathBuf,
        lock: File,
        workspace_id: Uuid,
        title: &str,
        workspace: &Workspace,
    ) -> Result<Self> {
        let options = SqliteConnectOptions::new()
            .filename(&path)
            .busy_timeout(Duration::from_secs(2))
            .foreign_keys(true)
            .pragma("trusted_schema", "OFF")
            .pragma("max_page_count", "32768")
            .pragma("journal_size_limit", "4194304")
            .disable_statement_logging();
        // Inspect identity read-only before any journal-mode change or migration.
        let mut check = SqliteConnection::connect_with(&options.clone().read_only(true)).await?;
        let app_id: i64 = sqlx::query_scalar("PRAGMA application_id")
            .fetch_one(&mut check)
            .await?;
        let page_size: i64 = sqlx::query_scalar("PRAGMA page_size")
            .fetch_one(&mut check)
            .await?;
        if page_size != 4096 {
            return Err(failed());
        }
        let tables: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'",
        )
        .fetch_one(&mut check)
        .await?;
        if tables != 0 {
            if app_id != APPLICATION_ID {
                return Err(failed());
            }
            let id: Option<String> =
                sqlx::query_scalar("SELECT id FROM workspace WHERE singleton=1")
                    .fetch_optional(&mut check)
                    .await?;
            let records: i64 = sqlx::query_scalar("SELECT count(*) FROM investigation")
                .fetch_one(&mut check)
                .await?;
            if id
                .as_deref()
                .is_some_and(|id| id != workspace_id.to_string())
                || (id.is_none() && records != 0)
            {
                return Err(FileError(
                    StatusCode::CONFLICT,
                    "workspace_identity_mismatch",
                    "This database belongs to another workspace. Reopen its original workspace; records were not reassigned.",
                ));
            }
        }
        check.close().await?;
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .acquire_timeout(Duration::from_secs(3))
            .connect_with(
                options
                    .journal_mode(SqliteJournalMode::Wal)
                    .synchronous(SqliteSynchronous::Full),
            )
            .await?;
        if MIGRATIONS.run(&pool).await.is_err() {
            pool.close().await;
            return Err(FileError(
                StatusCode::SERVICE_UNAVAILABLE,
                "migration_failed",
                "Local database migrations are incompatible or damaged. Preserve the database and use a compatible Cube Play version; no reset was performed.",
            ));
        }
        let mut transaction = pool.begin().await?;
        let timestamp = now()?;
        sqlx::query("INSERT OR IGNORE INTO workspace(singleton,id,created_ms) VALUES(1,?,?)")
            .bind(workspace_id.to_string())
            .bind(timestamp)
            .execute(&mut *transaction)
            .await?;
        let title: String = title
            .chars()
            .scan(0usize, |length, c| {
                *length += c.len_utf8();
                (*length <= 512).then_some(c)
            })
            .collect();
        sqlx::query("INSERT OR IGNORE INTO investigation(singleton,id,workspace_id,title,created_ms,updated_ms) VALUES(1,?,?,?,?,?)")
            .bind(Uuid::new_v4().to_string()).bind(workspace_id.to_string()).bind(title).bind(timestamp).bind(timestamp).execute(&mut *transaction).await?;
        let investigation_id: String =
            sqlx::query_scalar("SELECT id FROM investigation WHERE singleton=1")
                .fetch_one(&mut *transaction)
                .await?;
        sqlx::query("UPDATE sessions SET status='interrupted' WHERE status='running'")
            .execute(&mut *transaction)
            .await?;
        let session_id = Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO sessions(id,investigation_id,started_ms,status) VALUES(?,?,?,'running')",
        )
        .bind(&session_id)
        .bind(&investigation_id)
        .bind(timestamp)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;
        let database = Self {
            pool,
            _lock: lock,
            investigation_id,
            session_id,
        };
        database.reconcile(workspace).await?;
        Ok(database)
    }
    pub(crate) async fn snapshot(&self) -> Result<Snapshot> {
        let row = sqlx::query("SELECT id,workspace_id,title,revision,view_json,view_revision FROM investigation WHERE singleton=1").fetch_one(&self.pool).await?;
        let mut warnings = Vec::new();
        let view = serde_json::from_str::<View>(row.try_get("view_json")?).ok().filter(|view| view.validate().is_ok()).unwrap_or_else(|| {
            warnings.push("Saved navigation is invalid. The explorer opened at the source root; existing saved bytes were retained."); View::default()
        });
        let missing: i64 =
            sqlx::query_scalar("SELECT count(*) FROM artifacts WHERE status!='ready'")
                .fetch_one(&self.pool)
                .await?;
        if missing > 0 {
            warnings.push("Some imported artifacts are missing or incomplete. Restore the original files to managed storage and reopen; references were retained.");
        }
        Ok(Snapshot {
            id: row.try_get("id")?,
            workspace_id: row.try_get("workspace_id")?,
            title: row.try_get("title")?,
            revision: row.try_get("revision")?,
            view,
            view_revision: row.try_get("view_revision")?,
            session_id: self.session_id.clone(),
            interrupted_sessions: sqlx::query_scalar(
                "SELECT count(*) FROM sessions WHERE status='interrupted'",
            )
            .fetch_one(&self.pool)
            .await?,
            warnings,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::WorkspaceContext;
    #[tokio::test]
    async fn sqlite_full_rolls_back_records_without_replacing_existing_data() {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        let source = root.join("source");
        std::fs::create_dir(&source).unwrap();
        let mut config = Config::parse(
            "127.0.0.1:7331",
            "http://127.0.0.1:7331",
            "unused".into(),
            false,
        )
        .unwrap();
        config.workspace = Some(WorkspaceContext {
            id: Uuid::new_v4(),
            name: "Storage budget".into(),
        });
        config.source_dir = Some(source);
        config.data_dir = Some(root.join("imports"));
        let workspace = Workspace::open(&config).unwrap().unwrap();
        let handle = PersistenceHandle::default();
        let database = handle
            .get(&config, Some(&workspace))
            .await
            .unwrap()
            .unwrap();
        let pages: i64 = sqlx::query_scalar("PRAGMA page_count")
            .fetch_one(&database.pool)
            .await
            .unwrap();
        sqlx::query("PRAGMA max_page_count=1")
            .execute(&database.pool)
            .await
            .unwrap();
        assert!(pages > 0);
        let error = database
            .save_note(records::Note {
                id: Uuid::new_v4(),
                title: "No room".into(),
                target: None,
                body: "x".repeat(65536),
                revision: 0,
            })
            .await
            .unwrap_err();
        assert_eq!(error.1, "storage_unavailable");
        assert!(database.notes().await.unwrap().is_empty());
        assert_eq!(database.snapshot().await.unwrap().title, "Storage budget");
        sqlx::query("PRAGMA max_page_count=32768")
            .execute(&database.pool)
            .await
            .unwrap();
        handle.close().await.unwrap();
    }
}
