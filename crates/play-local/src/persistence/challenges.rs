//! Local public definitions, navigation and progress share the investigation database.
use super::{Database, conflict, failed, now};
use crate::{
    challenges::Definition,
    workspace::{FileError, Result},
};
use axum::http::StatusCode;
use serde_json::{Value, json};
fn missing() -> FileError {
    FileError(
        StatusCode::NOT_FOUND,
        "challenge_missing",
        "This local challenge or stage is unavailable in the current workspace.",
    )
}
impl Database {
    pub(crate) async fn challenge(&self, key: &str) -> Result<(Definition, String, i64)> {
        crate::challenges::key(key)?;
        let (encoded, stage, revision): (String, String, i64) =
            sqlx::query_as("SELECT definition_json,stage_id,revision FROM challenges WHERE key=?")
                .bind(key)
                .fetch_optional(&self.pool)
                .await?
                .ok_or_else(missing)?;
        let definition: Definition = serde_json::from_str(&encoded).map_err(|_| failed())?;
        definition.validate()?;
        if definition.key != key || !definition.manifest.stages.iter().any(|v| v.id == stage) {
            return Err(failed());
        }
        Ok((definition, stage, revision))
    }
    pub(crate) async fn challenges(&self) -> Result<Value> {
        let keys: Vec<String> =
            sqlx::query_scalar("SELECT key FROM challenges ORDER BY created_ms,key LIMIT 16")
                .fetch_all(&self.pool)
                .await?;
        let mut entries = Vec::new();
        for key in keys {
            let (definition, _, _) = self.challenge(&key).await?;
            entries.push(
                json!({"key":key,"id":definition.manifest.id,"title":definition.manifest.title}),
            );
        }
        let active: Option<String> =
            sqlx::query_scalar("SELECT active_challenge FROM investigation WHERE singleton=1")
                .fetch_one(&self.pool)
                .await?;
        let active = if let Some(key) = active {
            let (definition, stage, revision) = self.challenge(&key).await?;
            let artifacts=definition.manifest.artifacts.iter().map(|artifact|Ok(json!({"id":artifact.id,"store":definition.store,"path":definition.artifact_path(&artifact.path)?}))).collect::<Result<Vec<_>>>()?;
            json!({"definition":definition,"stage":stage,"revision":revision,"artifacts":artifacts,"progress":self.progress().await?.into_iter().filter(|entry|entry["challenge_key"]==key).collect::<Vec<_>>()})
        } else {
            Value::Null
        };
        Ok(json!({"schema_version":1,"challenges":entries,"active":active}))
    }
    pub(crate) async fn import_challenge(&self, definition: &Definition) -> Result<()> {
        definition.validate()?;
        let encoded = serde_json::to_string(definition).map_err(|_| failed())?;
        if encoded.len() > 512 * 1024 {
            return Err(crate::workspace::limit());
        }
        let mut tx = self.pool.begin().await?;
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM challenges WHERE key=?)")
                .bind(&definition.key)
                .fetch_one(&mut *tx)
                .await?;
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM challenges")
            .fetch_one(&mut *tx)
            .await?;
        if !exists && count >= 16 {
            return Err(crate::workspace::limit());
        }
        sqlx::query("INSERT INTO challenges(key,investigation_id,definition_json,stage_id,created_ms) VALUES(?,?,?,?,?) ON CONFLICT(key) DO NOTHING").bind(&definition.key).bind(&self.investigation_id).bind(encoded).bind(&definition.manifest.stages[0].id).bind(now()?).execute(&mut *tx).await?;
        sqlx::query("UPDATE investigation SET active_challenge=? WHERE singleton=1")
            .bind(&definition.key)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }
    pub(crate) async fn activate_challenge(&self, key: &str) -> Result<()> {
        self.challenge(key).await?;
        sqlx::query("UPDATE investigation SET active_challenge=? WHERE singleton=1")
            .bind(key)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
    pub(crate) async fn navigate_challenge(
        &self,
        key: &str,
        stage: &str,
        revision: i64,
    ) -> Result<()> {
        let (definition, _, _) = self.challenge(key).await?;
        if !definition.manifest.stages.iter().any(|v| v.id == stage) {
            return Err(missing());
        }
        if revision <= 0 || revision == i64::MAX {
            return Err(super::invalid());
        }
        let changed=sqlx::query("UPDATE challenges SET stage_id=?,revision=revision+1 WHERE key=? AND revision=? AND key=(SELECT active_challenge FROM investigation WHERE singleton=1)").bind(stage).bind(key).bind(revision).execute(&self.pool).await?.rows_affected();
        if changed != 1 {
            return Err(conflict());
        }
        Ok(())
    }
}
