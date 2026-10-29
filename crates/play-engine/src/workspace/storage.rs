use super::model::Identity;
use crate::Result;
use serde_json::Value;
use sqlx::{ConnectOptions, Connection, Row, SqliteConnection, sqlite::SqliteConnectOptions};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};
use uuid::Uuid;

pub const APPLICATION_ID: u32 = 0x4355_504e;
pub const GENERATION: u32 = 2;
pub const SCHEMA_VERSION: u32 = 4;
pub const RUN_SCHEMA: &str = include_str!("runs.sql");
pub const COPY_SCHEMA: &str = include_str!("copies.sql");
pub const ARCHIVE_SCHEMA: &str = include_str!("archive.sql");
pub type Schemas = Vec<Vec<(String, String, String)>>;
pub const SCHEMA: &str = include_str!("schema.sql");

pub fn failure(error: sqlx::Error) -> &'static str {
    if let sqlx::Error::Database(database) = &error {
        if database.is_foreign_key_violation() {
            return "ownership_mismatch";
        }
        if database.is_unique_violation() {
            return "state_conflict";
        }
        if database.code().as_deref() == Some("13") {
            return "storage_quota";
        }
    }
    "storage_unavailable"
}

pub fn regular(path: &Path) -> Result<()> {
    let metadata = fs::symlink_metadata(path).map_err(|_| "store_missing")?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("unsafe_store_path");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.nlink() != 1
            || metadata.mode() & 0o077 != 0
            || metadata.uid() != rustix::process::geteuid().as_raw()
        {
            return Err("unsafe_store_permissions");
        }
    }
    Ok(())
}

pub fn private_directory(path: &Path) -> Result<()> {
    if !path.exists() {
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(path).map_err(|_| "storage_unavailable")?;
    }
    let metadata = fs::symlink_metadata(path).map_err(|_| "store_missing")?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("unsafe_store_path");
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if metadata.mode() & 0o077 != 0 || metadata.uid() != rustix::process::geteuid().as_raw() {
            return Err("unsafe_store_permissions");
        }
    }
    Ok(())
}

pub fn create_file(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|_| "storage_unavailable")?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|_| "storage_unavailable")
}

pub fn sync_directory(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        File::open(path)
            .and_then(|file| file.sync_all())
            .map_err(|_| "storage_unavailable")?;
    }
    let _ = path;
    Ok(())
}

pub fn atomic_json(path: &Path, value: &impl serde::Serialize) -> Result<()> {
    let parent = path.parent().ok_or("unsafe_store_path")?;
    if path.exists() {
        regular(path)?;
    }
    let temporary = parent.join(format!(".write-{}", Uuid::new_v4()));
    let bytes = serde_json::to_vec(value).map_err(|_| "invalid_state")?;
    create_file(&temporary, &bytes)?;
    if fs::rename(&temporary, path).is_err() {
        let _ = fs::remove_file(&temporary);
        return Err("storage_unavailable");
    }
    sync_directory(parent)
}

pub fn read_json(path: &Path) -> Result<Value> {
    regular(path)?;
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|_| "store_missing")?
        .take(4097)
        .read_to_end(&mut bytes)
        .map_err(|_| "unsupported_store")?;
    if bytes.len() > 4096 {
        return Err("unsupported_store");
    }
    let text = std::str::from_utf8(&bytes).map_err(|_| "unsupported_store")?;
    cube_core::pipeline::json::parse(text).map_err(|_| "unsupported_store")
}

pub fn identity(directory: &Path, id: Uuid) -> Result<Identity> {
    private_existing_directory(directory)?;
    let identity: Identity = serde_json::from_value(read_json(&directory.join("workspace.json"))?)
        .map_err(|_| "unsupported_store")?;
    if identity.product != "cubearc.play.workspace"
        || identity.generation != GENERATION
        || identity.workspace_id != id
    {
        return Err("unsupported_store");
    }
    Ok(identity)
}

fn private_existing_directory(path: &Path) -> Result<()> {
    if !path.exists() {
        return Err("store_missing");
    }
    private_directory(path)
}

pub fn preflight(directory: &Path, id: Uuid) -> Result<Identity> {
    let identity = identity(directory, id)?;
    let path = directory.join("workspace.sqlite3");
    regular(&path)?;
    if fs::metadata(&path).map_err(|_| "store_missing")?.len()
        > crate::protocol::contract().store_bytes as u64
    {
        return Err("storage_quota");
    }
    regular(&directory.join("owner.lock"))?;
    let journal = directory.join("workspace.sqlite3-journal");
    if fs::symlink_metadata(&journal).is_ok() {
        regular(&journal)?;
    }
    if directory.join("workspace.sqlite3-wal").exists()
        || directory.join("workspace.sqlite3-shm").exists()
    {
        return Err("unsupported_store");
    }
    let mut header = [0u8; 100];
    File::open(&path)
        .and_then(|mut file| file.read_exact(&mut header))
        .map_err(|_| "unsupported_store")?;
    if &header[..16] != b"SQLite format 3\0"
        || u32::from_be_bytes(header[68..72].try_into().map_err(|_| "unsupported_store")?)
            != APPLICATION_ID
        || !(1..=SCHEMA_VERSION).contains(&u32::from_be_bytes(
            header[60..64].try_into().map_err(|_| "unsupported_store")?,
        ))
        || header[16..18] != [0x10, 0x00]
    {
        return Err("unsupported_store");
    }
    Ok(identity)
}

pub fn lock(path: &Path, create: bool) -> Result<File> {
    if !path.exists() && create {
        match create_file(path, b"") {
            Ok(()) => (),
            Err(_) if path.exists() => (),
            Err(code) => return Err(code),
        }
    }
    regular(path)?;
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(|_| "storage_unavailable")?;
    file.try_lock().map_err(|_| "workspace_busy")?;
    Ok(file)
}

pub fn options(path: &Path) -> SqliteConnectOptions {
    SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(false)
        .foreign_keys(true)
        .busy_timeout(Duration::ZERO)
        .pragma("trusted_schema", "OFF")
        .pragma("cache_size", "-2048")
        .disable_statement_logging()
}

async fn schema_rows(connection: &mut SqliteConnection) -> Result<Vec<(String, String, String)>> {
    let rows = sqlx::query(
        "SELECT name,type,sql FROM sqlite_master WHERE name NOT LIKE 'sqlite_%' ORDER BY name",
    )
    .fetch_all(connection)
    .await
    .map_err(failure)?;
    Ok(rows
        .into_iter()
        .map(|row| (row.get(0), row.get(1), row.get(2)))
        .collect())
}

pub async fn expected_schema() -> Result<Schemas> {
    let mut connection = SqliteConnection::connect(":memory:")
        .await
        .map_err(failure)?;
    sqlx::raw_sql(SCHEMA)
        .execute(&mut connection)
        .await
        .map_err(failure)?;
    let mut versions = vec![schema_rows(&mut connection).await?];
    for version in 2..=SCHEMA_VERSION {
        migrate_to(&mut connection, version).await?;
        versions.push(schema_rows(&mut connection).await?);
    }
    connection.close().await.map_err(failure)?;
    Ok(versions)
}

pub async fn inspect(path: &Path, id: Uuid, expected: &Schemas) -> Result<u32> {
    let mut check = SqliteConnection::connect_with(&options(path).read_only(true).immutable(true))
        .await
        .map_err(|_| "unsupported_store")?;
    let result = async {
        let version: i64 = sqlx::query_scalar("PRAGMA user_version")
            .fetch_one(&mut check)
            .await
            .map_err(|_| "unsupported_store")?;
        let expected = expected
            .get(usize::try_from(version - 1).map_err(|_| "unsupported_store")?)
            .ok_or("unsupported_store")?;
        let schema = schema_rows(&mut check)
            .await
            .map_err(|_| "unsupported_store")?;
        if &schema != expected {
            return Err("unsupported_store");
        }
        let row = sqlx::query("SELECT id,generation,schema_version FROM workspace_meta")
            .fetch_one(&mut check)
            .await
            .map_err(|_| "unsupported_store")?;
        let stored_id: String = row.get(0);
        if stored_id != id.to_string()
            || row.get::<i64, _>(1) != i64::from(GENERATION)
            || row.get::<i64, _>(2) != version
        {
            return Err("unsupported_store");
        }
        u32::try_from(version).map_err(|_| "unsupported_store")
    }
    .await;
    check.close().await.map_err(|_| "unsupported_store")?;
    result
}

pub struct Owned {
    pub identity: Identity,
    pub connection: Option<SqliteConnection>,
    pub _lock: File,
}
impl Owned {
    pub async fn open(directory: PathBuf, id: Uuid, expected: &Schemas) -> Result<Self> {
        let identity = preflight(&directory, id)?;
        inspect(&directory.join("workspace.sqlite3"), id, expected).await?;
        let lock = lock(&directory.join("owner.lock"), false)?;
        // Repeat identity/schema checks under the owner lock before writable recovery.
        preflight(&directory, id)?;
        let version = inspect(&directory.join("workspace.sqlite3"), id, expected).await?;
        super::upgrade::pending(&directory, id)?;
        let mut connection =
            SqliteConnection::connect_with(&options(&directory.join("workspace.sqlite3")))
                .await
                .map_err(failure)?;
        let checked = async {
            sqlx::query("PRAGMA max_page_count=4096")
                .execute(&mut connection)
                .await
                .map_err(failure)?;
            sqlx::query("PRAGMA foreign_key_check")
                .fetch_optional(&mut connection)
                .await
                .map_err(failure)?
                .map_or(Ok(()), |_| Err("corrupt_store"))?;
            Ok(())
        }
        .await;
        if let Err(code) = checked {
            connection.close().await.map_err(failure)?;
            return Err(code);
        }
        let cleaned = super::upgrade::clean(&directory, id);
        if let Err(code) = cleaned {
            connection.close().await.map_err(failure)?;
            return Err(code);
        }
        if version < SCHEMA_VERSION {
            connection.close().await.map_err(failure)?;
            super::upgrade::publish(&directory, id, version, expected).await?;
            connection =
                SqliteConnection::connect_with(&options(&directory.join("workspace.sqlite3")))
                    .await
                    .map_err(failure)?;
            if let Err(error) = sqlx::query("PRAGMA max_page_count=4096")
                .execute(&mut connection)
                .await
            {
                let code = failure(error);
                connection.close().await.map_err(failure)?;
                return Err(code);
            }
        }
        if let Err(code) = super::runs::recover(&mut connection).await {
            connection.close().await.map_err(failure)?;
            return Err(code);
        }
        Ok(Self {
            identity,
            connection: Some(connection),
            _lock: lock,
        })
    }
    pub fn connection(&mut self) -> Result<&mut SqliteConnection> {
        self.connection.as_mut().ok_or("workspace_closed")
    }
    pub async fn close(&mut self) -> Result<()> {
        if let Some(connection) = self.connection.take() {
            connection.close().await.map_err(failure)?;
        }
        Ok(())
    }
}

pub async fn migrate(connection: &mut SqliteConnection) -> Result<()> {
    migrate_to(connection, SCHEMA_VERSION).await
}

async fn migrate_to(connection: &mut SqliteConnection, target: u32) -> Result<()> {
    let version: i64 = sqlx::query_scalar("PRAGMA user_version")
        .fetch_one(&mut *connection)
        .await
        .map_err(failure)?;
    if version < 1 || version > i64::from(target) || target > SCHEMA_VERSION {
        return Err("unsupported_store");
    }
    sqlx::query("PRAGMA foreign_keys=OFF")
        .execute(&mut *connection)
        .await
        .map_err(failure)?;
    let result = async {
        let mut transaction = connection.begin().await.map_err(failure)?;
        for (next, schema) in [(2, RUN_SCHEMA), (3, COPY_SCHEMA), (4, ARCHIVE_SCHEMA)] {
            if version < i64::from(next) && next <= target {
                sqlx::raw_sql(schema)
                    .execute(&mut *transaction)
                    .await
                    .map_err(failure)?;
            }
        }
        sqlx::query("PRAGMA foreign_key_check")
            .fetch_optional(&mut *transaction)
            .await
            .map_err(failure)?
            .map_or(Ok(()), |_| Err("corrupt_store"))?;
        transaction.commit().await.map_err(failure)
    }
    .await;
    sqlx::query("PRAGMA foreign_keys=ON")
        .execute(connection)
        .await
        .map_err(failure)?;
    result
}
