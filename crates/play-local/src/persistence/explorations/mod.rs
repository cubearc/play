//! Private bounded exploration histories in the existing SQLite database.
mod model;
use super::{Database, conflict, failed, invalid, now};
use crate::{
    AppState,
    workspace::{self, Result},
};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Path, State},
    routing::get,
};
pub(crate) use model::Context;
pub(crate) use model::{Definition, Source};
use serde_json::{Value, json};
use sqlx::Row;
use uuid::Uuid;

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/explorations", get(list).put(save))
        .route("/explorations/{id}", get(load))
        .route("/explorations/{id}/executions", get(executions))
        .layer(DefaultBodyLimit::max(65536))
}
async fn db(state: &AppState) -> Result<std::sync::Arc<Database>> {
    state.database().await?.ok_or_else(failed)
}
async fn list(State(state): State<AppState>) -> Result<Json<Value>> {
    Ok(Json(
        json!({"schema_version":1,"explorations":db(&state).await?.explorations().await?,"legacy_history":{"assignment":"unassigned","replay":"parameters may be missing or redacted"}}),
    ))
}
async fn save(
    State(state): State<AppState>,
    request: axum::extract::Request,
) -> Result<Json<Definition>> {
    let parsed = crate::lab::request_json(request).await.map_err(|error| {
        workspace::FileError(
            if error.code == "resource_limit" {
                axum::http::StatusCode::PAYLOAD_TOO_LARGE
            } else {
                axum::http::StatusCode::BAD_REQUEST
            },
            "invalid_state",
            "Use bounded JSON without duplicate keys or excessive nesting.",
        )
    })?;
    let value = serde_json::from_value(parsed).map_err(|_| invalid())?;
    Ok(Json(db(&state).await?.save_exploration(value).await?))
}
async fn load(State(state): State<AppState>, Path(id): Path<Uuid>) -> Result<Json<Definition>> {
    let db = db(&state).await?;
    let revision: i64 = sqlx::query_scalar("SELECT revision FROM explorations WHERE id=?")
        .bind(id.to_string())
        .fetch_optional(&db.pool)
        .await?
        .ok_or_else(invalid)?;
    Ok(Json(db.exploration(id, revision).await?))
}
async fn executions(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<Value>>> {
    Ok(Json(db(&state).await?.exploration_executions(id).await?))
}
impl Database {
    async fn exploration(&self, id: Uuid, revision: i64) -> Result<Definition> {
        let encoded:String=sqlx::query_scalar("SELECT definition_json FROM exploration_configurations WHERE exploration_id=? AND revision=?").bind(id.to_string()).bind(revision).fetch_optional(&self.pool).await?.ok_or_else(invalid)?;
        let definition: Definition = serde_json::from_str(&encoded).map_err(|_| failed())?;
        definition.validate_archive().map_err(|_| failed())?;
        if definition.id != id || definition.revision != revision {
            return Err(failed());
        }
        Ok(definition)
    }
    /// Historical configuration remains readable after an operation changes.
    pub(crate) async fn result_configuration(
        &self,
        result: Uuid,
    ) -> Result<Option<(Definition, Uuid)>> {
        let rows = sqlx::query("SELECT exploration_id,configuration_revision,step_id FROM exploration_executions WHERE status='succeeded' AND json_extract(evidence_json,'$.result_id')=? LIMIT 2")
            .bind(result.to_string()).fetch_all(&self.pool).await?;
        if rows.len() > 1 {
            return Err(failed());
        }
        let Some(row) = rows.first() else {
            return Ok(None);
        };
        let id = Uuid::parse_str(row.try_get("exploration_id")?).map_err(|_| failed())?;
        let step = Uuid::parse_str(row.try_get("step_id")?).map_err(|_| failed())?;
        Ok(Some((
            self.exploration(id, row.try_get("configuration_revision")?)
                .await?,
            step,
        )))
    }
    async fn explorations(&self) -> Result<Vec<Definition>> {
        let rows = sqlx::query(
            "SELECT id,revision FROM explorations ORDER BY updated_ms DESC,id LIMIT 32",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut definitions = Vec::new();
        for row in rows {
            definitions.push(
                self.exploration(
                    Uuid::parse_str(row.try_get("id")?).map_err(|_| failed())?,
                    row.try_get("revision")?,
                )
                .await?,
            );
        }
        Ok(definitions)
    }
    async fn save_exploration(&self, mut definition: Definition) -> Result<Definition> {
        definition.validate()?;
        if let Some(scope) = &definition.scope {
            let (challenge, _, _) = self.challenge(&scope.challenge).await?;
            if !challenge
                .manifest
                .stages
                .iter()
                .any(|s| s.id == scope.stage)
            {
                return Err(invalid());
            }
        }
        let expected = definition.revision;
        definition.revision += 1;
        let encoded = serde_json::to_string(&definition).map_err(|_| invalid())?;
        if encoded.len() > 65536 {
            return Err(workspace::limit());
        }
        let mut tx = self.pool.begin().await?;
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM explorations")
            .fetch_one(&mut *tx)
            .await?;
        let (versions,bytes):(i64,i64)=sqlx::query_as("SELECT count(*),coalesce(sum(length(CAST(definition_json AS BLOB))),0) FROM exploration_configurations").fetch_one(&mut *tx).await?;
        if (expected == 0 && count >= 32)
            || versions >= 256
            || bytes + encoded.len() as i64 > 8 * 1024 * 1024
        {
            return Err(workspace::limit());
        }
        let timestamp = now()?;
        let changed = if expected == 0 {
            sqlx::query("INSERT INTO explorations(id,investigation_id,revision,created_ms,updated_ms) VALUES(?,?,1,?,?) ON CONFLICT(id) DO NOTHING").bind(definition.id.to_string()).bind(&self.investigation_id).bind(timestamp).bind(timestamp).execute(&mut *tx).await?.rows_affected()
        } else {
            sqlx::query("UPDATE explorations SET revision=revision+1,updated_ms=? WHERE id=? AND revision=?").bind(timestamp).bind(definition.id.to_string()).bind(expected).execute(&mut *tx).await?.rows_affected()
        };
        if changed != 1 {
            return Err(conflict());
        }
        sqlx::query("INSERT INTO exploration_configurations(exploration_id,revision,definition_json,created_ms) VALUES(?,?,?,?)").bind(definition.id.to_string()).bind(definition.revision).bind(encoded).bind(timestamp).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(definition)
    }
    pub(crate) async fn begin_exploration_run(
        &self,
        context: &Context,
        id: Uuid,
        operation: &str,
        version: u32,
        inputs: &Value,
        parameters: &Value,
    ) -> Result<()> {
        let definition = self.exploration(context.id, context.revision).await?;
        let step = definition
            .steps
            .iter()
            .find(|s| s.id == context.step)
            .ok_or_else(invalid)?;
        if step.operation != operation || step.operation_version != version {
            return Err(conflict());
        }
        let mut expected = serde_json::Map::new();
        for (name, source) in &step.inputs {
            let value = match source {
                Source::Step {
                    step: upstream,
                    port,
                } => {
                    let evidence:Option<String>=sqlx::query_scalar("SELECT evidence_json FROM exploration_executions WHERE exploration_id=? AND configuration_revision=? AND step_id=? AND status='succeeded' ORDER BY started_ms DESC,id DESC LIMIT 1").bind(context.id.to_string()).bind(context.revision).bind(upstream.to_string()).fetch_optional(&self.pool).await?;
                    let evidence: Value = serde_json::from_str(&evidence.ok_or_else(conflict)?)
                        .map_err(|_| failed())?;
                    json!({"type":"artifact","id":evidence["result_id"],"port":port})
                }
                _ => serde_json::to_value(source).map_err(|_| failed())?,
            };
            expected.insert(name.clone(), value);
        }
        let mut supplied = parameters.as_object().ok_or_else(invalid)?.clone();
        for name in &step.private_parameters {
            supplied.remove(name);
        }
        if inputs != &Value::Object(expected) || Value::Object(supplied) != step.parameters {
            return Err(conflict());
        }
        let mut tx = self.pool.begin().await?;
        let count: i64 = sqlx::query_scalar("SELECT count(*) FROM exploration_executions")
            .fetch_one(&mut *tx)
            .await?;
        if count >= 512 {
            return Err(workspace::limit());
        }
        sqlx::query("INSERT INTO exploration_executions(id,exploration_id,configuration_revision,step_id,status,started_ms) VALUES(?,?,?,?,'running',?)").bind(id.to_string()).bind(context.id.to_string()).bind(context.revision).bind(context.step.to_string()).bind(now()?).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(())
    }
    pub(crate) async fn finish_exploration_run(
        &self,
        id: Uuid,
        outcome: &std::result::Result<Value, crate::lab::types::Error>,
    ) -> Result<()> {
        let (status, evidence) = match outcome {
            Ok(response) => {
                let retained = response["retained"] == true;
                let inputs=response["result"]["provenance"]["inputs"].as_array().map(|inputs|inputs.iter().map(|input|json!({"port":input["reference"]["input_port"],"semantic_type":input["semantic_type"],"sha256":input["sha256"],"byte_size":input["byte_size"]})).collect::<Vec<_>>()).unwrap_or_default();
                (
                    "succeeded",
                    json!({"result_id":if retained {response["result"]["id"].clone()} else {Value::Null},"retained":retained,"ports":response["result"]["metadata"]["ports"],"sha256":response["result"]["sha256"],"inputs":inputs,"provenance_reference":if retained {response["result"]["id"].clone()} else {Value::Null},"replay":if retained {"see artifact provenance"} else {"private output was not retained"}}),
                )
            }
            Err(error) => (
                if error.code == "cancelled" {
                    "cancelled"
                } else {
                    "failed"
                },
                json!({"error":error}),
            ),
        };
        let encoded = serde_json::to_string(&evidence).map_err(|_| failed())?;
        if encoded.len() > 32768 {
            return Err(workspace::limit());
        }
        let changed=sqlx::query("UPDATE exploration_executions SET status=?,evidence_json=?,ended_ms=? WHERE id=? AND status='running'").bind(status).bind(encoded).bind(now()?).bind(id.to_string()).execute(&self.pool).await?.rows_affected();
        if changed != 1 {
            return Err(conflict());
        }
        Ok(())
    }
    async fn exploration_executions(&self, id: Uuid) -> Result<Vec<Value>> {
        let rows=sqlx::query("SELECT x.id,x.configuration_revision,x.step_id,x.status,x.evidence_json,x.started_ms,x.ended_ms,e.revision FROM exploration_executions x JOIN explorations e ON e.id=x.exploration_id WHERE x.exploration_id=? ORDER BY x.started_ms,x.id LIMIT 512").bind(id.to_string()).fetch_all(&self.pool).await?;
        rows.into_iter().map(|row|{
            let revision:i64=row.try_get("configuration_revision")?;
            Ok(json!({"id":row.try_get::<String,_>("id")?,"configuration_revision":revision,"step":row.try_get::<String,_>("step_id")?,"status":row.try_get::<String,_>("status")?,"stale":revision!=row.try_get::<i64,_>("revision")?,"evidence":serde_json::from_str::<Value>(row.try_get("evidence_json")?).map_err(|_|failed())?,"started_ms":row.try_get::<i64,_>("started_ms")?,"ended_ms":row.try_get::<Option<i64>,_>("ended_ms")?}))
        }).collect()
    }
}
