use super::storage::{self, Schemas};
use crate::Result;
use serde::{Deserialize, Serialize};
use sqlx::{Connection, SqliteConnection};
use std::{
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
};
use uuid::Uuid;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Pending {
    product: String,
    workspace_id: Uuid,
    generation: u32,
    from_schema: u32,
    to_schema: u32,
    temporary: Uuid,
}
impl Pending {
    fn path(&self, directory: &Path) -> PathBuf {
        directory.join(format!(".upgrade-{}.sqlite3", self.temporary))
    }
    fn files(&self, directory: &Path) -> Vec<PathBuf> {
        let path = self.path(directory);
        let name = path.to_string_lossy();
        vec![
            path.clone(),
            PathBuf::from(format!("{name}-journal")),
            PathBuf::from(format!("{name}-wal")),
            PathBuf::from(format!("{name}-shm")),
        ]
    }
}

pub fn pending(directory: &Path, id: Uuid) -> Result<Option<Pending>> {
    let marker = directory.join("upgrade.json");
    if fs::symlink_metadata(&marker).is_err() {
        return Ok(None);
    }
    let value: Pending =
        serde_json::from_value(storage::read_json(&marker)?).map_err(|_| "unsupported_store")?;
    if value.product != "cubearc.play.workspace.upgrade"
        || value.workspace_id != id
        || value.generation != storage::GENERATION
        || value.from_schema < 1
        || value.from_schema >= value.to_schema
        || value.to_schema > storage::SCHEMA_VERSION
    {
        return Err("unsupported_store");
    }
    for path in value.files(directory) {
        if fs::symlink_metadata(&path).is_ok() {
            storage::regular(&path)?;
        }
    }
    Ok(Some(value))
}

pub fn clean(directory: &Path, id: Uuid) -> Result<()> {
    if let Some(value) = pending(directory, id)? {
        // Only this marker's unpublished native copy is disposable; the published store is untouched.
        for path in value.files(directory) {
            if fs::symlink_metadata(&path).is_ok() {
                fs::remove_file(path).map_err(|_| "storage_unavailable")?;
            }
        }
        fs::remove_file(directory.join("upgrade.json")).map_err(|_| "storage_unavailable")?;
        storage::sync_directory(directory)?;
    }
    Ok(())
}

pub async fn publish(
    directory: &Path,
    id: Uuid,
    from_schema: u32,
    schemas: &Schemas,
) -> Result<()> {
    let pending = Pending {
        product: "cubearc.play.workspace.upgrade".into(),
        workspace_id: id,
        generation: storage::GENERATION,
        from_schema,
        to_schema: storage::SCHEMA_VERSION,
        temporary: Uuid::new_v4(),
    };
    storage::atomic_json(&directory.join("upgrade.json"), &pending)?;
    let path = pending.path(directory);
    let result = async {
        storage::create_file(&path, b"")?;
        fs::copy(directory.join("workspace.sqlite3"), &path).map_err(|_| "storage_unavailable")?;
        let mut copy = SqliteConnection::connect_with(&storage::options(&path))
            .await
            .map_err(storage::failure)?;
        let migrated = async {
            sqlx::query("PRAGMA max_page_count=4096")
                .execute(&mut copy)
                .await
                .map_err(storage::failure)?;
            storage::migrate(&mut copy).await
        }
        .await;
        copy.close().await.map_err(storage::failure)?;
        migrated?;
        if storage::inspect(&path, id, schemas).await? != storage::SCHEMA_VERSION {
            return Err("unsupported_store");
        }
        OpenOptions::new()
            .write(true)
            .open(&path)
            .and_then(|file| file.sync_all())
            .map_err(|_| "storage_unavailable")?;
        fs::rename(&path, directory.join("workspace.sqlite3"))
            .map_err(|_| "storage_unavailable")?;
        storage::sync_directory(directory)
    }
    .await;
    let cleaned = clean(directory, id);
    result?;
    cleaned
}
