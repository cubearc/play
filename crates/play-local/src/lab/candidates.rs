//! Local selection/conversion only: no answer checking, submission or verification.
use super::{execute, read_permit, types::*};
use crate::{AppState, challenges::AnswerType};
use axum::{
    Json, Router,
    extract::{DefaultBodyLimit, Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;
#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub(super) enum Selection {
    Whole,
    Field {
        pointer: String,
        conversion: Conversion,
    },
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Conversion {
    Text,
    Base64Bytes,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Candidate {
    key: String,
    stage: String,
    result_id: Uuid,
    revision: i64,
    selection: Selection,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct QueryStage {
    key: String,
    stage: String,
}
fn invalid() -> Error {
    Error::new(
        "invalid_candidate",
        "Select a named text/byte artifact compatible with this stage, or an explicit string field as text or Core-decoded Base64 bytes. Candidates are limited to 64 KiB; images, numeric coercion and arbitrary conversions are unsupported.",
    )
}
pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/challenges/candidates", get(list).put(select))
        .layer(DefaultBodyLimit::max(16384))
}
async fn database(app: &AppState) -> Result<std::sync::Arc<crate::persistence::Database>> {
    app.database().await?.ok_or_else(Error::storage)
}
pub(super) fn value(
    record: &Record,
    bytes: &[u8],
    selection: &Selection,
) -> Result<(Representation, Vec<u8>)> {
    let (representation, bytes) = match selection {
        Selection::Whole
            if matches!(
                record.representation,
                Representation::Text | Representation::Bytes
            ) =>
        {
            (record.representation, bytes.to_vec())
        }
        Selection::Field {
            pointer,
            conversion,
        } if record.representation == Representation::Structured
            && pointer.len() <= 512
            && pointer.starts_with('/') =>
        {
            let json: Value = serde_json::from_slice(bytes).map_err(|_| Error::storage())?;
            let selected = json
                .pointer(pointer)
                .and_then(Value::as_str)
                .ok_or_else(invalid)?;
            match conversion {
                Conversion::Text => (Representation::Text, selected.as_bytes().to_vec()),
                Conversion::Base64Bytes => (
                    Representation::Bytes,
                    cube_core::encoding::base64_decode(selected).map_err(|_| invalid())?,
                ),
            }
        }
        _ => return Err(invalid()),
    };
    if bytes.len() > 65536
        || (representation == Representation::Text && std::str::from_utf8(&bytes).is_err())
    {
        return Err(invalid());
    }
    Ok((representation, bytes))
}
async fn select(
    State(app): State<AppState>,
    Json(candidate): Json<Candidate>,
) -> Result<Json<Value>> {
    let db = database(&app).await?;
    let definition = db.challenge(&candidate.key).await?.0;
    let stage = definition
        .manifest
        .stages
        .iter()
        .find(|v| v.id == candidate.stage)
        .ok_or_else(invalid)?;
    if candidate.revision < 0 || candidate.revision == i64::MAX {
        return Err(invalid());
    }
    let answer = stage.answer_type;
    let storage = app.lab.storage(&app).await?;
    let _permit = read_permit(&app).await?;
    let result_id = candidate.result_id;
    let selection = candidate.selection.clone();
    let revision = candidate.revision + 1;
    let record=tokio::task::spawn_blocking(move ||->Result<Value>{
        let (record,bytes)=storage.load(result_id)?;
        let (representation,selected)=value(&record,&bytes,&selection)?;
        if (answer==AnswerType::Text)!=(representation==Representation::Text){return Err(invalid());}
        Ok(json!({"revision":revision,"result_id":result_id,"selection":selection,"representation":representation,"byte_size":selected.len(),"sha256":cube_core::hashing::sha256(&selected),"artifact_sha256":record.sha256,"artifact_operation":record.provenance.operation,"core_version":execute::core_version(),"status":"private_unverified"}))
    }).await.map_err(|_|Error::storage())??;
    db.append_candidate(
        &candidate.key,
        &candidate.stage,
        candidate.result_id,
        candidate.revision,
        record,
    )
    .await?;
    snapshot(&app, &candidate.key, &candidate.stage)
        .await
        .map(Json)
}
async fn list(State(app): State<AppState>, Query(query): Query<QueryStage>) -> Result<Json<Value>> {
    snapshot(&app, &query.key, &query.stage).await.map(Json)
}
async fn snapshot(app: &AppState, key: &str, stage: &str) -> Result<Value> {
    let records = database(app).await?.candidate_history(key, stage).await?;
    let mut error = None;
    if let Some(record) = records.first() {
        let storage = app.lab.storage(app).await?;
        let record = record.clone();
        let _permit = read_permit(app).await?;
        let checked = tokio::task::spawn_blocking(move || -> Result<()> {
            let id: Uuid = serde_json::from_value(record["result_id"].clone())
                .map_err(|_| Error::storage())?;
            let selection: Selection = serde_json::from_value(record["selection"].clone())
                .map_err(|_| Error::storage())?;
            let (artifact, bytes) = storage.load(id)?;
            let (representation, selected) = value(&artifact, &bytes, &selection)?;
            if record["artifact_sha256"] != artifact.sha256
                || record["sha256"] != cube_core::hashing::sha256(&selected)
                || record["byte_size"] != selected.len()
                || record["representation"]
                    != serde_json::to_value(representation).map_err(|_| Error::storage())?
            {
                return Err(Error::storage());
            }
            Ok(())
        })
        .await
        .map_err(|_| Error::storage())?;
        if let Err(failure) = checked {
            error = Some(failure.message);
        }
    }
    Ok(json!({"schema_version":1,"candidate":records.first(),"history":records,"error":error}))
}
