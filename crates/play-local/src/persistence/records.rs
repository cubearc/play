use super::{Database, FileError, conflict, failed, invalid, now};
use crate::workspace::{self, Result, Store, Workspace};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use sqlx::Row;
use uuid::Uuid;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct View {
    pub store: Store,
    pub path: String,
    pub offset: usize,
    pub selected: Option<Selection>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Selection {
    pub store: Store,
    pub path: String,
}
impl View {
    pub(super) fn validate(&self) -> Result<()> {
        workspace::validate(&self.path, true)?;
        if self.offset >= workspace::MAX_ENTRIES
            || !self.offset.is_multiple_of(workspace::PAGE_SIZE)
        {
            return Err(invalid());
        }
        if let Some(selected) = &self.selected {
            workspace::validate(&selected.path, false)?;
        }
        Ok(())
    }
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Metadata {
    pub revision: i64,
    pub title: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SavedView {
    pub revision: i64,
    pub view: View,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Note {
    pub id: Uuid,
    pub revision: i64,
    pub title: String,
    pub body: String,
    #[serde(default)]
    pub target: Option<Selection>,
}
#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Progress {
    pub challenge_key: String,
    pub stage_key: String,
    pub revision: i64,
    pub local_status: LocalStatus,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum LocalStatus {
    Unstarted,
    InProgress,
    Completed,
}
impl LocalStatus {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Unstarted => "unstarted",
            Self::InProgress => "in_progress",
            Self::Completed => "completed",
        }
    }
}
#[derive(Serialize)]
pub(super) struct Artifact {
    id: String,
    path: String,
    byte_size: i64,
    status: String,
}
#[derive(Serialize, sqlx::FromRow)]
pub(super) struct Session {
    id: String,
    started_ms: i64,
    ended_ms: Option<i64>,
    status: String,
}
fn bounded(value: &str, limit: usize) -> bool {
    !value.is_empty() && value.len() <= limit && !value.contains('\0')
}

impl Database {
    pub(super) async fn sessions(&self) -> Result<Vec<Session>> {
        Ok(sqlx::query_as("SELECT id,started_ms,ended_ms,status FROM sessions ORDER BY started_ms DESC,id DESC LIMIT 50").fetch_all(&self.pool).await?)
    }

    pub(super) async fn metadata(&self, value: Metadata) -> Result<i64> {
        if !bounded(&value.title, 512) || value.revision < 1 || value.revision == i64::MAX {
            return Err(invalid());
        }
        let changed=sqlx::query("UPDATE investigation SET title=?,revision=revision+1,updated_ms=? WHERE singleton=1 AND revision=?")
            .bind(value.title).bind(now()?).bind(value.revision).execute(&self.pool).await?.rows_affected();
        if changed != 1 {
            return Err(conflict());
        }
        Ok(value.revision + 1)
    }
    pub(super) async fn save_view(&self, value: SavedView) -> Result<i64> {
        value.view.validate()?;
        let encoded = serde_json::to_string(&value.view).map_err(|_| invalid())?;
        if encoded.len() > 12288 || value.revision < 0 || value.revision == i64::MAX {
            return Err(invalid());
        }
        let changed=sqlx::query("UPDATE investigation SET view_json=?,view_revision=view_revision+1,updated_ms=? WHERE singleton=1 AND view_revision=?")
            .bind(encoded).bind(now()?).bind(value.revision).execute(&self.pool).await?.rows_affected();
        if changed != 1 {
            return Err(conflict());
        }
        Ok(value.revision + 1)
    }
    pub(super) async fn notes(&self) -> Result<Vec<Note>> {
        let rows = sqlx::query(
            "SELECT id,revision,title,body,target_json FROM notes ORDER BY updated_ms,id LIMIT 100",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|row| {
                Ok(Note {
                    id: Uuid::parse_str(row.try_get("id")?).map_err(|_| failed())?,
                    revision: row.try_get("revision")?,
                    title: row.try_get("title")?,
                    body: row.try_get("body")?,
                    target: row
                        .try_get::<Option<String>, _>("target_json")?
                        .map(|v| serde_json::from_str(&v).map_err(|_| failed()))
                        .transpose()?,
                })
            })
            .collect()
    }
    pub(super) async fn save_note(&self, value: Note) -> Result<i64> {
        if let Some(target) = &value.target {
            workspace::validate(&target.path, false)?;
        }
        let target = value
            .target
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(|_| invalid())?;
        if value.id.get_version_num() != 4
            || !bounded(&value.title, 512)
            || value.body.len() > 65536
            || value.revision < 0
            || value.revision == i64::MAX
        {
            return Err(invalid());
        }
        let mut transaction = self.pool.begin().await?;
        if value.revision == 0 {
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM notes")
                .fetch_one(&mut *transaction)
                .await?;
            if count >= 100 {
                return Err(FileError(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "record_limit",
                    "This investigation reached its 100-note storage limit.",
                ));
            }
        }
        let changed=sqlx::query("INSERT INTO notes(id,investigation_id,title,body,target_json,revision,updated_ms) SELECT ?,?,?,?,?,1,? WHERE ?=0 ON CONFLICT(id) DO NOTHING")
            .bind(value.id.to_string()).bind(&self.investigation_id).bind(&value.title).bind(&value.body).bind(&target).bind(now()?).bind(value.revision).execute(&mut *transaction).await?.rows_affected();
        let changed = if value.revision > 0 {
            sqlx::query("UPDATE notes SET title=?,body=?,target_json=?,revision=revision+1,updated_ms=? WHERE id=? AND revision=?")
            .bind(value.title).bind(value.body).bind(target).bind(now()?).bind(value.id.to_string()).bind(value.revision).execute(&mut *transaction).await?.rows_affected()
        } else {
            changed
        };
        if changed != 1 {
            return Err(conflict());
        }
        transaction.commit().await?;
        Ok(value.revision + 1)
    }
    pub(super) async fn progress(&self) -> Result<Vec<serde_json::Value>> {
        let rows=sqlx::query("SELECT challenge_key,stage_key,revision,local_status FROM progress ORDER BY challenge_key,stage_key LIMIT 1000").fetch_all(&self.pool).await?;
        rows.into_iter().map(|row|Ok(serde_json::json!({"challenge_key":row.try_get::<String,_>("challenge_key")?,"stage_key":row.try_get::<String,_>("stage_key")?,"revision":row.try_get::<i64,_>("revision")?,"local_status":row.try_get::<String,_>("local_status")?}))).collect()
    }
    pub(super) async fn save_progress(&self, value: Progress) -> Result<i64> {
        if !bounded(&value.challenge_key, 128)
            || !bounded(&value.stage_key, 128)
            || value.revision < 0
            || value.revision == i64::MAX
        {
            return Err(invalid());
        }
        let mut transaction = self.pool.begin().await?;
        if value.revision == 0 {
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM progress")
                .fetch_one(&mut *transaction)
                .await?;
            if count >= 1000 {
                return Err(FileError(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "record_limit",
                    "This investigation reached its 1,000-stage local progress storage limit.",
                ));
            }
        }
        let changed = if value.revision == 0 {
            sqlx::query("INSERT INTO progress(challenge_key,stage_key,investigation_id,local_status,revision,updated_ms) VALUES(?,?,?,?,1,?) ON CONFLICT(challenge_key,stage_key) DO NOTHING")
                .bind(&value.challenge_key).bind(&value.stage_key).bind(&self.investigation_id).bind(value.local_status.as_str()).bind(now()?).execute(&mut *transaction).await?.rows_affected()
        } else {
            sqlx::query("UPDATE progress SET local_status=?,revision=revision+1,updated_ms=? WHERE challenge_key=? AND stage_key=? AND revision=?")
                .bind(value.local_status.as_str()).bind(now()?).bind(&value.challenge_key).bind(&value.stage_key).bind(value.revision).execute(&mut *transaction).await?.rows_affected()
        };
        if changed != 1 {
            return Err(conflict());
        }
        transaction.commit().await?;
        Ok(value.revision + 1)
    }
    pub(super) async fn artifacts(&self) -> Result<Vec<Artifact>> {
        let rows = sqlx::query(
            "SELECT id,path,byte_size,status FROM artifacts ORDER BY created_ms,id LIMIT 4096",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|row| {
                Ok(Artifact {
                    id: row.try_get("id")?,
                    path: row.try_get("path")?,
                    byte_size: row.try_get("byte_size")?,
                    status: row.try_get("status")?,
                })
            })
            .collect()
    }
    pub(crate) async fn reserve_import(
        &self,
        name: &str,
        temporary: &str,
        size: u64,
    ) -> Result<String> {
        let mut transaction = self.pool.begin().await?;
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM artifacts")
            .fetch_one(&mut *transaction)
            .await?;
        if count >= 4096 {
            return Err(FileError(
                StatusCode::PAYLOAD_TOO_LARGE,
                "record_limit",
                "Managed storage reached its 4,096-artifact reference limit.",
            ));
        }
        let id = Uuid::new_v4().to_string();
        let changed=sqlx::query("INSERT INTO artifacts(id,investigation_id,path,byte_size,status,temporary,created_ms) VALUES(?,?,?,?,'pending',?,?) ON CONFLICT(path) DO NOTHING")
            .bind(&id).bind(&self.investigation_id).bind(name).bind(i64::try_from(size).map_err(|_|invalid())?).bind(temporary).bind(now()?).execute(&mut *transaction).await?.rows_affected();
        if changed != 1 {
            return Err(FileError(
                StatusCode::CONFLICT,
                "file_exists",
                "An artifact already uses that name. Choose another name; its reference was preserved.",
            ));
        }
        transaction.commit().await?;
        Ok(id)
    }
    pub(crate) async fn finish_import(&self, id: &str) -> Result<()> {
        sqlx::query(
            "UPDATE artifacts SET status='ready',temporary=NULL WHERE id=? AND status='pending'",
        )
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
    pub(crate) async fn cancel_import(&self, id: &str) -> Result<()> {
        sqlx::query("DELETE FROM artifacts WHERE id=? AND status='pending'")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    pub(crate) async fn authorize_import(&self, path: &str) -> Result<()> {
        let ready: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM artifacts WHERE path=? AND status='ready')",
        )
        .bind(path)
        .fetch_one(&self.pool)
        .await?;
        if !ready {
            return Err(FileError(
                StatusCode::CONFLICT,
                "artifact_unavailable",
                "This imported artifact is missing or incomplete. Reopen the workspace to reconcile managed storage.",
            ));
        }
        Ok(())
    }
    pub(super) async fn reconcile(&self, workspace: &Workspace) -> Result<()> {
        // Only shallow bounded metadata, never source files, decoding or execution.
        let root = workspace.root(Store::Imports)?.clone();
        let files=tokio::task::spawn_blocking(move ||->Result<Vec<(String,u64)>> {
            let mut files=Vec::new();
            for (index,entry) in root.entries()?.take(workspace::MAX_ENTRIES+1).enumerate() {
                if index==workspace::MAX_ENTRIES {return Err(FileError(StatusCode::PAYLOAD_TOO_LARGE,"recovery_limit","Managed storage exceeds 4,096 entries. Preserve the data and reduce the directory size before reopening."));}
                let entry=entry?;
                let name=entry.file_name();
                let Some(name)=name.to_str() else {continue;};
                if name.starts_with(workspace::TEMP_PREFIX) || workspace::validate(name,false).is_err() {continue;}
                let metadata=root.symlink_metadata(name)?;
                if metadata.is_file() && !metadata.file_type().is_symlink() {files.push((name.into(),metadata.len()));}
            } Ok(files)
        }).await.map_err(|_|failed())??;
        let mut transaction = self.pool.begin().await?;
        // Missing references survive; a user can restore the original bytes later.
        sqlx::query("UPDATE artifacts SET status='missing' WHERE status='ready'")
            .execute(&mut *transaction)
            .await?;
        for (name, size) in files {
            sqlx::query("INSERT INTO artifacts(id,investigation_id,path,byte_size,status,created_ms) VALUES(?,?,?,?,'ready',?) ON CONFLICT(path) DO UPDATE SET status=CASE WHEN artifacts.byte_size=excluded.byte_size THEN 'ready' ELSE 'missing' END,temporary=NULL")
                .bind(Uuid::new_v4().to_string()).bind(&self.investigation_id).bind(name).bind(i64::try_from(size).map_err(|_|failed())?).bind(now()?).execute(&mut *transaction).await?;
        }
        // An unpublished file was never visible; its abandoned reservation is safe to release.
        sqlx::query("DELETE FROM artifacts WHERE status='pending'")
            .execute(&mut *transaction)
            .await?;
        transaction.commit().await?;
        Ok(())
    }
}
