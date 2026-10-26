//! Names point to immutable Lab bundles; binary content never enters SQLite.
use super::{Database, conflict, invalid, now};
use crate::workspace::Result;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SavedResult {
    pub id: Uuid,
    pub title: String,
    pub revision: i64,
}
impl Database {
    pub(crate) async fn saved_results(&self) -> Result<Vec<SavedResult>> {
        let rows: Vec<(String, String, i64)> = sqlx::query_as(
            "SELECT id,title,revision FROM saved_results ORDER BY updated_ms DESC,id LIMIT 64",
        )
        .fetch_all(&self.pool)
        .await?;
        rows.into_iter()
            .map(|(id, title, revision)| {
                Ok(SavedResult {
                    id: Uuid::parse_str(&id).map_err(|_| invalid())?,
                    title,
                    revision,
                })
            })
            .collect()
    }
    pub(crate) async fn save_result(&self, value: SavedResult) -> Result<i64> {
        if value.id.get_version_num() != 4
            || value.title.trim().is_empty()
            || value.title.len() > 512
            || value.title.contains('\0')
            || value.revision < 0
            || value.revision == i64::MAX
        {
            return Err(invalid());
        }
        let mut tx = self.pool.begin().await?;
        if value.revision == 0 {
            let count: i64 = sqlx::query_scalar("SELECT count(*) FROM saved_results")
                .fetch_one(&mut *tx)
                .await?;
            if count >= 64 {
                return Err(crate::workspace::limit());
            }
        }
        let changed = if value.revision == 0 {
            sqlx::query("INSERT INTO saved_results(id,investigation_id,title,revision,updated_ms) VALUES(?,?,?,1,?) ON CONFLICT(id) DO NOTHING").bind(value.id.to_string()).bind(&self.investigation_id).bind(value.title).bind(now()?).execute(&mut *tx).await?.rows_affected()
        } else {
            sqlx::query("UPDATE saved_results SET title=?,revision=revision+1,updated_ms=? WHERE id=? AND revision=?").bind(value.title).bind(now()?).bind(value.id.to_string()).bind(value.revision).execute(&mut *tx).await?.rows_affected()
        };
        if changed != 1 {
            return Err(conflict());
        }
        tx.commit().await?;
        Ok(value.revision + 1)
    }
}
