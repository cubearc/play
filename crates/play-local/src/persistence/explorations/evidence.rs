//! References identify actual local attempts, independently of editable graphs.
use super::{Database, failed};
use crate::workspace::{FileError, Result};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

fn invalid_reference() -> FileError {
    FileError(
        StatusCode::BAD_REQUEST,
        "invalid_evidence_reference",
        "Select an actual execution from this workspace and a recorded retained output port, or associate with the whole attempt.",
    )
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct EvidenceTarget {
    pub execution_id: Uuid,
    pub output_port: Option<String>,
}
impl EvidenceTarget {
    pub(crate) async fn validate(&self, database: &Database) -> Result<()> {
        if self.execution_id.get_version_num() != 4 {
            return Err(invalid_reference());
        }
        let execution = database.execution(self.execution_id).await?;
        if let Some(port) = &self.output_port
            && (port.is_empty()
                || port.len() > 128
                || port.contains('\0')
                || execution["status"] != "succeeded"
                || execution["evidence"]["retained"] != true
                || !execution["evidence"]["ports"]
                    .as_array()
                    .is_some_and(|ports| {
                        ports.iter().filter(|p| p["name"] == port.as_str()).count() == 1
                    }))
        {
            return Err(invalid_reference());
        }
        Ok(())
    }
}
impl Database {
    pub(crate) async fn execution(&self, id: Uuid) -> Result<Value> {
        let row=sqlx::query("SELECT x.exploration_id,x.configuration_revision,x.step_id,x.status,x.evidence_json,x.started_ms,x.ended_ms,e.revision FROM exploration_executions x JOIN explorations e ON e.id=x.exploration_id WHERE x.id=?")
            .bind(id.to_string()).fetch_optional(&self.pool).await?.ok_or_else(invalid_reference)?;
        let revision: i64 = row.try_get("configuration_revision")?;
        let evidence: Value =
            serde_json::from_str(row.try_get("evidence_json")?).map_err(|_| failed())?;
        Ok(
            json!({"schema_version":1,"id":id,"exploration_id":row.try_get::<String,_>("exploration_id")?,"configuration_revision":revision,"step":row.try_get::<String,_>("step_id")?,"status":row.try_get::<String,_>("status")?,"stale":revision!=row.try_get::<i64,_>("revision")?,"evidence":evidence,"started_ms":row.try_get::<i64,_>("started_ms")?,"ended_ms":row.try_get::<Option<i64>,_>("ended_ms")?}),
        )
    }
}
