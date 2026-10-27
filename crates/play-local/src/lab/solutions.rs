//! A solution curates retained evidence; it never executes or verifies an answer.
use super::{candidates, execute, read_permit, replay, types::*};
use crate::{
    AppState,
    persistence::{Database, explorations::Definition, solutions::MAX_SNAPSHOT_BYTES},
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use uuid::Uuid;

#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Field {
    pointer: String,
    conversion: candidates::Conversion,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct FinalOutput {
    result_id: Uuid,
    #[serde(default)]
    port: Option<String>,
    #[serde(default)]
    field: Option<Field>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CandidateRef {
    key: String,
    stage: String,
    revision: i64,
}
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum Source {
    Result {
        selection: FinalOutput,
        #[serde(default)]
        candidate: Option<CandidateRef>,
    },
    Revision {
        revision: i64,
    },
}
#[derive(Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum LocalStatus {
    #[default]
    Finding,
    Resolved,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    id: Uuid,
    revision: i64,
    title: String,
    explanation: String,
    #[serde(default)]
    local_status: LocalStatus,
    source: Source,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    schema_version: u32,
    id: Uuid,
    revision: i64,
    title: String,
    explanation: String,
    local_status: LocalStatus,
    privacy: String,
    verification: String,
    final_output: FinalOutput,
    final_metadata: Value,
    evidence_path: Vec<Record>,
    configuration_scope: String,
    configurations: Vec<Definition>,
    executions: Vec<Value>,
    candidate: Option<Value>,
    replay_at_save: Value,
    derived_from_revision: Option<i64>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Revision {
    revision: Option<i64>,
}
fn invalid() -> Error {
    Error::new(
        "invalid_solution",
        "Choose retained local evidence, a named output where applicable, an explicit supported field conversion, and valid private solution fields.",
    )
}
fn incomplete() -> Error {
    Error::new(
        "resource_limit",
        "The evidence path exceeds the bounded solution-check budget. Existing solutions and artifacts were preserved; select a smaller derivation.",
    )
}
async fn database(app: &AppState) -> Result<std::sync::Arc<Database>> {
    app.database().await?.ok_or_else(Error::storage)
}
pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route("/solutions", get(list).put(save))
        .route("/solutions/{id}", get(load))
        .route("/solutions/{id}/final", get(final_content))
}
async fn list(State(app): State<AppState>) -> Result<Json<Value>> {
    Ok(Json(
        json!({"schema_version":1,"solutions":database(&app).await?.solutions().await?}),
    ))
}
fn read_snapshot(value: Value) -> Result<Snapshot> {
    let snapshot: Snapshot = serde_json::from_value(value).map_err(|_| Error::storage())?;
    if snapshot.schema_version != 1
        || snapshot.privacy != "private"
        || snapshot.verification != "unverified"
        || snapshot.evidence_path.len() > 64
        || snapshot.configurations.len() > 64
        || snapshot.executions.len() > 64
        || snapshot.configuration_scope != "executed_path"
        || snapshot.final_output.result_id.get_version_num() != 4
        || !snapshot
            .evidence_path
            .iter()
            .any(|r| r.id == snapshot.final_output.result_id)
    {
        return Err(Error::storage());
    }
    Ok(snapshot)
}
async fn current_report(app: &AppState, snapshot: &Snapshot) -> Result<Value> {
    let (mut report, current) = replay::retained_path(app, snapshot.final_output.result_id).await?;
    for expected in &snapshot.evidence_path {
        if !current
            .iter()
            .find(|r| r.id == expected.id)
            .is_some_and(|actual| {
                serde_json::to_value(actual).ok() == serde_json::to_value(expected).ok()
            })
        {
            report.gap(expected.id,None,"frozen_evidence_unavailable","Retained evidence is missing, corrupt or differs from this immutable solution revision. The saved snapshot was preserved.");
        }
    }
    serde_json::to_value(report).map_err(|_| Error::storage())
}
async fn response(app: &AppState, id: Uuid, revision: Option<i64>) -> Result<Value> {
    let (value, latest, history) = database(app).await?.solution(id, revision).await?;
    let snapshot = read_snapshot(value)?;
    let report = current_report(app, &snapshot).await?;
    Ok(
        json!({"schema_version":1,"solution":snapshot,"latest_revision":latest,"history":history,"replay_current":report}),
    )
}
async fn load(
    State(app): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<Revision>,
) -> Result<Json<Value>> {
    let _permit = read_permit(&app).await?;
    Ok(Json(response(&app, id, query.revision).await?))
}
async fn save(State(app): State<AppState>, request: axum::extract::Request) -> Result<Json<Value>> {
    let request: Request =
        serde_json::from_value(super::request_json(request).await?).map_err(|_| invalid())?;
    if request.id.get_version_num() != 4
        || request.revision < 0
        || request.revision == i64::MAX
        || request.title.trim().is_empty()
        || request.title.len() > 512
        || request.title.contains('\0')
        || request.explanation.len() > 8192
        || request.explanation.trim().is_empty()
        || request.explanation.contains('\0')
    {
        return Err(invalid());
    }
    let _permit = read_permit(&app).await?;
    let db = database(&app).await?;
    let mut snapshot = match request.source {
        Source::Result {
            selection,
            candidate,
        } => freeze(&app, selection, candidate).await?,
        Source::Revision { revision } => {
            if revision < 1 || revision > request.revision {
                return Err(invalid());
            }
            let (value, _, _) = db.solution(request.id, Some(revision)).await?;
            let mut previous = read_snapshot(value)?;
            previous.replay_at_save = current_report(&app, &previous).await?;
            previous.derived_from_revision = Some(revision);
            previous
        }
    };
    snapshot.id = request.id;
    snapshot.revision = request.revision + 1;
    snapshot.title = request.title;
    snapshot.explanation = request.explanation;
    snapshot.local_status = request.local_status;
    let bytes = execute::json_bytes(&snapshot, MAX_SNAPSHOT_BYTES).map_err(|_|Error::new("resource_limit","This solution exceeds the 512 KiB snapshot limit. Select a smaller derivation; existing revisions were preserved."))?;
    db.save_solution(
        request.id,
        request.revision,
        serde_json::from_slice(&bytes).map_err(|_| Error::storage())?,
    )
    .await?;
    let (_, latest, history) = db.solution(request.id, Some(request.revision + 1)).await?;
    Ok(Json(
        json!({"schema_version":1,"solution":snapshot,"latest_revision":latest,"history":history,"replay_current":snapshot.replay_at_save}),
    ))
}
fn pointer(port: &str) -> String {
    format!("/{}", port.replace('~', "~0").replace('/', "~1"))
}
struct Selected {
    metadata: Value,
    candidate: Option<candidates::Selection>,
    representation: Representation,
    content: Vec<u8>,
}
fn selection(record: &Record, bytes: &[u8], selected: &FinalOutput) -> Result<Selected> {
    let typed =
        record.semantic_type == "operation.outputs" && record.metadata["schema_version"] == 2;
    let mut prefix = String::new();
    let (value, semantic, schema) = if typed {
        let port = selected
            .port
            .as_ref()
            .filter(|p| !p.is_empty() && p.len() <= 128)
            .ok_or_else(invalid)?;
        let ports = record.metadata["ports"]
            .as_array()
            .ok_or_else(Error::storage)?;
        let matches = ports
            .iter()
            .filter(|p| p["name"] == port.as_str())
            .collect::<Vec<_>>();
        if matches.len() != 1 {
            return Err(invalid());
        }
        let output: Value = serde_json::from_slice(bytes).map_err(|_| Error::storage())?;
        let value = output.get(port).cloned().ok_or_else(invalid)?;
        prefix = pointer(port);
        (
            Some(value),
            matches[0]["semantic_type"]
                .as_str()
                .ok_or_else(Error::storage)?
                .to_owned(),
            Some(matches[0].clone()),
        )
    } else {
        if selected.port.is_some() {
            return Err(invalid());
        }
        (None, record.semantic_type.clone(), None)
    };
    let (representation, content, digest_representation, candidate) =
        if let Some(field) = &selected.field {
            if field.pointer.len() > 512
                || !(field.pointer.starts_with('/')
                    || (selected.port.is_some() && field.pointer.is_empty()))
            {
                return Err(invalid());
            }
            let candidate = candidates::Selection::Field {
                pointer: format!("{prefix}{}", field.pointer),
                conversion: field.conversion.clone(),
            };
            let (representation, content) = candidates::value(record, bytes, &candidate)?;
            (
                representation,
                content,
                "explicit_field_conversion",
                Some(candidate),
            )
        } else if let Some(value) = value {
            if (semantic == "text" || semantic.starts_with("text.")) && value.is_string() {
                (
                    Representation::Text,
                    value.as_str().ok_or_else(invalid)?.as_bytes().to_vec(),
                    "utf8_text",
                    Some(candidates::Selection::Field {
                        pointer: prefix,
                        conversion: candidates::Conversion::Text,
                    }),
                )
            } else if semantic == "bytes" || semantic == "byte_reader" {
                (
                    Representation::Bytes,
                    super::domains::bytes(&value)?,
                    "decoded_bytes",
                    Some(candidates::Selection::Field {
                        pointer: format!("{prefix}/base64"),
                        conversion: candidates::Conversion::Base64Bytes,
                    }),
                )
            } else {
                (
                    Representation::Structured,
                    execute::json_bytes(&value, MAX_OUTPUT)?,
                    "json_wire_value",
                    None,
                )
            }
        } else {
            let candidate = matches!(
                record.representation,
                Representation::Text | Representation::Bytes
            )
            .then_some(candidates::Selection::Whole);
            (
                record.representation,
                bytes.to_vec(),
                "artifact_bytes",
                candidate,
            )
        };
    let semantic = if selected.field.is_some() {
        match representation {
            Representation::Text => "text",
            Representation::Bytes => "bytes",
            _ => return Err(invalid()),
        }
        .to_owned()
    } else {
        semantic
    };
    Ok(Selected {
        metadata: json!({"representation":representation,"semantic_type":semantic,"byte_size":content.len(),"sha256":cube_core::hashing::sha256(&content),"digest_representation":digest_representation,"port_schema":schema}),
        candidate,
        representation,
        content,
    })
}
async fn final_content(
    State(app): State<AppState>,
    Path(id): Path<Uuid>,
    Query(query): Query<Revision>,
) -> Result<Json<Value>> {
    let _permit = read_permit(&app).await?;
    let (value, _, _) = database(&app).await?.solution(id, query.revision).await?;
    let snapshot = read_snapshot(value)?;
    let root = snapshot.final_output.result_id;
    let storage = app.lab.storage(&app).await?;
    let (record, bytes) = tokio::task::spawn_blocking(move || storage.load(root))
        .await
        .map_err(|_| Error::storage())??;
    let expected = snapshot
        .evidence_path
        .iter()
        .find(|r| r.id == root)
        .ok_or_else(Error::storage)?;
    if serde_json::to_value(&record).map_err(|_| Error::storage())?
        != serde_json::to_value(expected).map_err(|_| Error::storage())?
    {
        return Err(Error::new(
            "storage_unavailable",
            "The selected artifact differs from the immutable solution. Its snapshot was preserved.",
        ));
    }
    let selected = selection(&record, &bytes, &snapshot.final_output)?;
    if selected.metadata != snapshot.final_metadata {
        return Err(Error::new(
            "storage_unavailable",
            "The selected value no longer matches this solution revision. Its reference and explanation were preserved.",
        ));
    }
    let value = match selected.representation {
        Representation::Text => {
            json!(std::str::from_utf8(&selected.content).map_err(|_| Error::storage())?)
        }
        Representation::Bytes | Representation::Image => {
            json!({"base64":cube_core::encoding::base64_encode(&selected.content)})
        }
        Representation::Structured => {
            serde_json::from_slice(&selected.content).map_err(|_| Error::storage())?
        }
    };
    Ok(Json(
        json!({"schema_version":1,"solution_id":id,"revision":snapshot.revision,"selection":snapshot.final_output,"metadata":selected.metadata,"value":value}),
    ))
}
async fn freeze(
    app: &AppState,
    selected: FinalOutput,
    candidate: Option<CandidateRef>,
) -> Result<Snapshot> {
    let storage = app.lab.storage(app).await?;
    let id = selected.result_id;
    let (record, bytes) = tokio::task::spawn_blocking(move || storage.load(id))
        .await
        .map_err(|_| Error::storage())??;
    let selection = selection(&record, &bytes, &selected)?;
    let final_metadata = selection.metadata;
    let candidate_selection = selection.candidate;
    let db = database(app).await?;
    let candidate = if let Some(reference) = candidate {
        if reference.key.is_empty()
            || reference.key.len() > 128
            || reference.stage.is_empty()
            || reference.stage.len() > 128
            || reference.revision < 1
        {
            return Err(invalid());
        }
        let history = db
            .candidate_history(&reference.key, &reference.stage)
            .await?;
        let candidate = history
            .into_iter()
            .find(|c| c["revision"] == reference.revision)
            .ok_or_else(invalid)?;
        let chosen = candidate_selection.ok_or_else(invalid)?;
        let (representation, content) = candidates::value(&record, &bytes, &chosen)?;
        if candidate["result_id"] != id.to_string()
            || candidate["artifact_sha256"] != record.sha256
            || candidate["selection"] != serde_json::to_value(&chosen).map_err(|_| invalid())?
            || candidate["representation"]
                != serde_json::to_value(representation).map_err(|_| invalid())?
            || candidate["sha256"] != cube_core::hashing::sha256(&content)
            || candidate["byte_size"] != content.len()
        {
            return Err(invalid());
        }
        Some(json!({"reference":reference,"record":candidate}))
    } else {
        None
    };
    let (report, records) = replay::retained_path(app, id).await?;
    let report = serde_json::to_value(report).map_err(|_| Error::storage())?;
    if report["gaps"]
        .as_array()
        .is_some_and(|g| g.iter().any(|g| g["code"] == "check_limit"))
    {
        return Err(incomplete());
    }
    let mut configurations = Vec::new();
    let mut executions = Vec::new();
    let mut seen = BTreeSet::new();
    let mut used = BTreeMap::<(Uuid, i64), BTreeSet<Uuid>>::new();
    for record in &records {
        if let Some((definition, step)) = db.result_configuration(record.id).await? {
            let operation = definition
                .steps
                .iter()
                .find(|s| s.id == step)
                .ok_or_else(Error::storage)?;
            let run = db.execution(record.id).await?;
            if operation.operation != record.provenance.operation
                || run["evidence"]["sha256"] != record.sha256
                || run["evidence"]["ports"] != record.metadata["ports"]
                || run["evidence"]["retained"] != true
                || run["status"] != "succeeded"
            {
                return Err(Error::storage());
            }
            executions.push(run);
            used.entry((definition.id, definition.revision))
                .or_default()
                .insert(step);
            if seen.insert((definition.id, definition.revision)) {
                configurations.push(definition);
            }
        }
    }
    for configuration in &mut configurations {
        let selected = &used[&(configuration.id, configuration.revision)];
        configuration
            .steps
            .retain(|step| selected.contains(&step.id));
    }
    Ok(Snapshot {
        schema_version: 1,
        id: Uuid::nil(),
        revision: 1,
        title: String::new(),
        explanation: String::new(),
        local_status: LocalStatus::Finding,
        privacy: "private".into(),
        verification: "unverified".into(),
        final_output: selected,
        final_metadata,
        evidence_path: records,
        configuration_scope: "executed_path".into(),
        configurations,
        executions,
        candidate,
        replay_at_save: report,
        derived_from_revision: None,
    })
}
