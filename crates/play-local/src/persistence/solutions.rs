//! Private immutable solution revisions share the existing investigation database.
use super::{Database, conflict, failed, invalid, now};
use crate::workspace::{FileError, Result};
use axum::http::StatusCode;
use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

pub(crate) const MAX_SNAPSHOT_BYTES: usize = 512 * 1024;
fn quota(message: &'static str) -> FileError {
    FileError(StatusCode::PAYLOAD_TOO_LARGE, "resource_limit", message)
}

fn checked(value: Value, id: Uuid, revision: i64) -> Result<Value> {
    if value["schema_version"] != 1
        || value["id"] != id.to_string()
        || value["revision"] != revision
        || value["privacy"] != "private"
        || value["verification"] != "unverified"
        || !matches!(value["local_status"].as_str(), Some("finding" | "resolved"))
        || !value["title"]
            .as_str()
            .is_some_and(|s| !s.trim().is_empty() && s.len() <= 512 && !s.contains('\0'))
        || !value["explanation"]
            .as_str()
            .is_some_and(|s| !s.trim().is_empty() && s.len() <= 8192 && !s.contains('\0'))
    {
        return Err(failed());
    }
    Ok(value)
}
impl Database {
    pub(crate) async fn solutions(&self) -> Result<Vec<Value>> {
        let rows=sqlx::query("SELECT s.id,s.revision,s.updated_ms,json_extract(r.snapshot_json,'$.title') title,json_extract(r.snapshot_json,'$.local_status') local_status FROM solutions s JOIN solution_revisions r ON r.solution_id=s.id AND r.revision=s.revision ORDER BY s.updated_ms DESC,s.id LIMIT 32").fetch_all(&self.pool).await?;
        rows.into_iter().map(|r|Ok(json!({"id":r.try_get::<String,_>("id")?,"revision":r.try_get::<i64,_>("revision")?,"updated_ms":r.try_get::<i64,_>("updated_ms")?,"title":r.try_get::<String,_>("title")?,"local_status":r.try_get::<String,_>("local_status")?,"privacy":"private","verification":"unverified"}))).collect()
    }
    pub(crate) async fn solution(
        &self,
        id: Uuid,
        revision: Option<i64>,
    ) -> Result<(Value, i64, Vec<Value>)> {
        if id.get_version_num() != 4 || revision.is_some_and(|r| r < 1) {
            return Err(invalid());
        }
        let latest: i64 = sqlx::query_scalar("SELECT revision FROM solutions WHERE id=?")
            .bind(id.to_string())
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(invalid)?;
        let wanted = revision.unwrap_or(latest);
        let encoded: String = sqlx::query_scalar(
            "SELECT snapshot_json FROM solution_revisions WHERE solution_id=? AND revision=?",
        )
        .bind(id.to_string())
        .bind(wanted)
        .fetch_optional(&self.pool)
        .await?
        .ok_or_else(invalid)?;
        if encoded.len() > MAX_SNAPSHOT_BYTES {
            return Err(failed());
        }
        let snapshot = checked(
            serde_json::from_str(&encoded).map_err(|_| failed())?,
            id,
            wanted,
        )?;
        let rows=sqlx::query("SELECT revision,created_ms,json_extract(snapshot_json,'$.title') title,json_extract(snapshot_json,'$.local_status') local_status FROM solution_revisions WHERE solution_id=? ORDER BY revision DESC LIMIT 128").bind(id.to_string()).fetch_all(&self.pool).await?;
        let history=rows.into_iter().map(|r|Ok(json!({"revision":r.try_get::<i64,_>("revision")?,"created_ms":r.try_get::<i64,_>("created_ms")?,"title":r.try_get::<String,_>("title")?,"local_status":r.try_get::<String,_>("local_status")?}))).collect::<Result<Vec<_>>>()?;
        Ok((snapshot, latest, history))
    }
    pub(crate) async fn save_solution(
        &self,
        id: Uuid,
        expected: i64,
        snapshot: Value,
    ) -> Result<()> {
        if id.get_version_num() != 4 || expected < 0 || expected == i64::MAX {
            return Err(invalid());
        }
        let snapshot = checked(snapshot, id, expected + 1).map_err(|_| invalid())?;
        let encoded = serde_json::to_string(&snapshot).map_err(|_| invalid())?;
        if encoded.len() > MAX_SNAPSHOT_BYTES {
            return Err(quota(
                "This solution exceeds the 512 KiB snapshot limit. Select a smaller derivation; existing revisions were preserved.",
            ));
        }
        let timestamp = now()?;
        let mut tx = self.pool.begin().await?;
        let changed = if expected == 0 {
            sqlx::query("INSERT INTO solutions(id,investigation_id,revision,updated_ms) SELECT ?,?,1,? WHERE (SELECT count(*) FROM solutions)<32 ON CONFLICT(id) DO NOTHING").bind(id.to_string()).bind(&self.investigation_id).bind(timestamp).execute(&mut *tx).await?.rows_affected()
        } else {
            sqlx::query(
                "UPDATE solutions SET revision=revision+1,updated_ms=? WHERE id=? AND revision=?",
            )
            .bind(timestamp)
            .bind(id.to_string())
            .bind(expected)
            .execute(&mut *tx)
            .await?
            .rows_affected()
        };
        if changed != 1 {
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM solutions")
                .fetch_one(&mut *tx)
                .await?;
            return Err(if expected == 0 && count >= 32 {
                quota(
                    "This workspace retains 32 private solutions. Use another workspace; existing solutions were preserved.",
                )
            } else {
                conflict()
            });
        }
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM solution_revisions")
            .fetch_one(&mut *tx)
            .await?;
        if count >= 128 {
            return Err(quota(
                "This workspace retains 128 immutable solution revisions. Use another workspace; earlier revisions were preserved.",
            ));
        }
        sqlx::query("INSERT INTO solution_revisions(solution_id,revision,snapshot_json,created_ms) VALUES(?,?,?,?)").bind(id.to_string()).bind(expected+1).bind(encoded).bind(timestamp).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::{Config, WorkspaceContext},
        persistence::PersistenceHandle,
        workspace::Workspace,
    };

    #[tokio::test]
    async fn solution_quotas_and_failed_appends_preserve_prior_revisions_atomically() {
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
            name: "Private solutions".into(),
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
        let id = Uuid::new_v4();
        let snapshot = |id: Uuid, revision: i64| json!({"schema_version":1,"id":id,"revision":revision,"privacy":"private","verification":"unverified","local_status":"finding","title":"Known finding","explanation":"An explicit explanation."});
        database
            .save_solution(id, 0, snapshot(id, 1))
            .await
            .unwrap();
        let original = database.solution(id, None).await.unwrap().0;
        assert_eq!(
            database
                .save_solution(id, 0, snapshot(id, 1))
                .await
                .unwrap_err()
                .0,
            StatusCode::CONFLICT
        );
        assert!(
            database
                .save_solution(Uuid::nil(), 0, snapshot(Uuid::nil(), 1))
                .await
                .is_err()
        );
        let mut oversized = snapshot(id, 2);
        oversized["evidence"] = json!("x".repeat(MAX_SNAPSHOT_BYTES));
        assert_eq!(
            database
                .save_solution(id, 1, oversized)
                .await
                .unwrap_err()
                .1,
            "resource_limit"
        );
        let mut invalid = snapshot(id, 2);
        invalid["explanation"] = json!("");
        assert!(database.save_solution(id, 1, invalid).await.is_err());
        sqlx::query("PRAGMA max_page_count=1")
            .execute(&database.pool)
            .await
            .unwrap();
        let mut large = snapshot(id, 2);
        large["evidence"] = json!("x".repeat(128 * 1024));
        assert_eq!(
            database.save_solution(id, 1, large).await.unwrap_err().1,
            "storage_unavailable"
        );
        assert_eq!(database.solution(id, None).await.unwrap().0, original);
        sqlx::query("PRAGMA max_page_count=32768")
            .execute(&database.pool)
            .await
            .unwrap();
        for _ in 1..32 {
            let id = Uuid::new_v4();
            database
                .save_solution(id, 0, snapshot(id, 1))
                .await
                .unwrap();
        }
        let extra = Uuid::new_v4();
        assert_eq!(
            database
                .save_solution(extra, 0, snapshot(extra, 1))
                .await
                .unwrap_err()
                .1,
            "resource_limit"
        );
        assert!(database.solution(extra, None).await.is_err());
        assert_eq!(database.solutions().await.unwrap().len(), 32);
        for revision in 2..=97 {
            database
                .save_solution(id, revision - 1, snapshot(id, revision))
                .await
                .unwrap();
        }
        assert_eq!(
            database
                .save_solution(id, 97, snapshot(id, 98))
                .await
                .unwrap_err()
                .1,
            "resource_limit"
        );
        let (latest, revision, history) = database.solution(id, None).await.unwrap();
        assert_eq!(revision, 97);
        assert_eq!(latest["revision"], 97);
        assert_eq!(history.len(), 97);
        assert_eq!(database.solution(id, Some(1)).await.unwrap().0, original);
        assert!(database.solution(id, Some(0)).await.is_err());
        assert!(database.solution(id, Some(98)).await.is_err());
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM solution_revisions")
            .fetch_one(&database.pool)
            .await
            .unwrap();
        assert_eq!(count, 128);
        handle.close().await.unwrap();
    }
}
