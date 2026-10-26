//! Immutable local candidate revisions point to existing named artifact bundles.
use super::{Database, conflict, invalid, now};
use crate::workspace::{FileError, Result};
use axum::http::StatusCode;
use serde_json::Value;
use uuid::Uuid;
impl Database {
    pub(crate) async fn candidate_history(&self, key: &str, stage: &str) -> Result<Vec<Value>> {
        let definition = self.challenge(key).await?.0;
        if !definition
            .manifest
            .stages
            .iter()
            .any(|item| item.id == stage)
        {
            return Err(invalid());
        }
        let rows:Vec<(i64,String,String)>=sqlx::query_as("SELECT revision,result_id,record_json FROM candidates WHERE challenge_key=? AND stage_key=? ORDER BY revision DESC LIMIT 256").bind(key).bind(stage).fetch_all(&self.pool).await?;
        rows.into_iter()
            .map(|(revision, id, encoded)| {
                let value: Value = serde_json::from_str(&encoded).map_err(|_| super::failed())?;
                if value["revision"] != revision
                    || value["result_id"] != id
                    || value["status"] != "private_unverified"
                {
                    return Err(super::failed());
                }
                Ok(value)
            })
            .collect()
    }
    pub(crate) async fn append_candidate(
        &self,
        key: &str,
        stage: &str,
        result: Uuid,
        expected: i64,
        record: Value,
    ) -> Result<()> {
        if expected < 0 || expected == i64::MAX {
            return Err(invalid());
        }
        let encoded = serde_json::to_string(&record).map_err(|_| invalid())?;
        if encoded.len() > 16384 {
            return Err(invalid());
        }
        let mut tx = self.pool.begin().await?;
        let saved: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM saved_results WHERE id=?)")
                .bind(result.to_string())
                .fetch_one(&mut *tx)
                .await?;
        if !saved {
            return Err(FileError(
                StatusCode::CONFLICT,
                "artifact_not_saved",
                "Save this result as a named artifact before associating a candidate.",
            ));
        }
        let latest: Option<i64> = sqlx::query_scalar(
            "SELECT max(revision) FROM candidates WHERE challenge_key=? AND stage_key=?",
        )
        .bind(key)
        .bind(stage)
        .fetch_one(&mut *tx)
        .await?;
        if latest.unwrap_or(0) != expected {
            return Err(conflict());
        }
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM candidates")
            .fetch_one(&mut *tx)
            .await?;
        if count >= 256 {
            return Err(FileError(
                StatusCode::PAYLOAD_TOO_LARGE,
                "candidate_limit",
                "This workspace retains 256 candidate revisions. Existing associations were preserved.",
            ));
        }
        sqlx::query("INSERT INTO candidates(challenge_key,stage_key,revision,result_id,record_json,created_ms) VALUES(?,?,?,?,?,?)").bind(key).bind(stage).bind(expected+1).bind(result.to_string()).bind(encoded).bind(now()?).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
}
