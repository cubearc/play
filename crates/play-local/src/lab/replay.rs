//! Inspect retained evidence and current inputs without executing an operation.
use super::{execute, read_permit, storage::Storage, typed, types::*};
use crate::{AppState, persistence::explorations::Source};
use axum::{
    Json,
    extract::{Path, State},
};
use cube_core::operations;
use serde::Serialize;
use std::{
    collections::{BTreeSet, VecDeque},
    sync::Arc,
};
use uuid::Uuid;

const MAX_CHECKED_BYTES: usize = 64 * 1024 * 1024;
#[derive(Serialize)]
pub(super) struct Gap {
    artifact_id: Uuid,
    input_port: Option<String>,
    code: &'static str,
    message: &'static str,
}
#[derive(Serialize)]
pub(super) struct Report {
    schema_version: u32,
    artifact_id: Uuid,
    status: &'static str,
    checked_artifacts: usize,
    checked_bytes: usize,
    gaps: Vec<Gap>,
}
impl Report {
    pub(super) fn gap(
        &mut self,
        id: Uuid,
        port: Option<&str>,
        code: &'static str,
        message: &'static str,
    ) {
        // A stored bundle has at most 32 inputs; keep failures bounded too.
        if self.gaps.len() < 256 {
            self.gaps.push(Gap {
                artifact_id: id,
                input_port: port.map(str::to_owned),
                code,
                message,
            });
        }
        self.status = "gaps";
    }
}
pub(super) async fn inspect(
    State(app): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Report>> {
    let _permit = read_permit(&app).await?;
    check(&app, id).await.map(Json)
}
pub(super) async fn check(app: &AppState, id: Uuid) -> Result<Report> {
    Ok(retained_path(app, id).await?.0)
}
pub(super) async fn retained_path(app: &AppState, id: Uuid) -> Result<(Report, Vec<Record>)> {
    let storage = app.lab.storage(app).await?;
    let mut report = Report {
        schema_version: 1,
        artifact_id: id,
        status: "inputs_available",
        checked_artifacts: 0,
        checked_bytes: 0,
        gaps: Vec::new(),
    };
    let mut queue = VecDeque::from([id]);
    let mut seen = BTreeSet::new();
    let mut records = Vec::new();
    while let Some(id) = queue.pop_front() {
        if !seen.insert(id) {
            continue;
        }
        if seen.len() > 64 || report.checked_bytes >= MAX_CHECKED_BYTES {
            report.gap(
                id,
                None,
                "check_limit",
                "The bounded evidence check could not inspect the complete dependency path.",
            );
            break;
        }
        let copy = storage.clone();
        let loaded = tokio::task::spawn_blocking(move || copy.load(id))
            .await
            .map_err(|_| Error::storage())?;
        let (record, bytes) = match loaded {
            Ok(value) => value,
            Err(_) => {
                report.gap(id, None, "artifact_unavailable", "A retained artifact is missing or failed its integrity check. Existing references were preserved.");
                continue;
            }
        };
        report.checked_artifacts += 1;
        report.checked_bytes += bytes.len();
        records.push(record.clone());
        if record.semantic_type != "operation.outputs" || record.metadata["schema_version"] != 2 {
            report.gap(id, None, "legacy_history", "Legacy history has no complete typed configuration or schema evidence; it remains unassigned.");
            if record.provenance.inputs.len() > 32 {
                report.gap(
                    id,
                    None,
                    "check_limit",
                    "Legacy dependency metadata exceeds the bounded path limit.",
                );
            }
            for input in record.provenance.inputs.iter().take(32) {
                if input.reference["type"] == "artifact" {
                    if let Ok(parent) =
                        serde_json::from_value::<Uuid>(input.reference["id"].clone())
                    {
                        queue.push_back(parent);
                    } else {
                        report.gap(id,None,"artifact_unavailable","A legacy dependency reference is invalid. Existing history was preserved.");
                    }
                }
            }
            continue;
        }
        inspect_record(app, storage.clone(), &record, &mut report, &mut queue).await?;
    }
    Ok((report, records))
}
async fn inspect_record(
    app: &AppState,
    storage: Arc<Storage>,
    record: &Record,
    report: &mut Report,
    queue: &mut VecDeque<Uuid>,
) -> Result<()> {
    let id = record.id;
    let contract =
        operations::describe(&record.provenance.operation).and_then(|d| d.contract().ok());
    if record.metadata["private_inputs_redacted"] == true {
        report.gap(id,None,"private_inputs_required","Private inputs were not retained or fingerprinted; supply them explicitly for a new execution.");
    }
    let ports = record.metadata["ports"].as_array();
    let current = contract.as_ref().is_some_and(|c| {
        ports.is_some_and(|ports| {
            ports.len() == c.outputs.len()
                && c.outputs.iter().all(|p| {
                    let hash = execute::json_bytes(&p.schema, 128 * 1024)
                        .map(|v| cube_core::hashing::sha256(&v))
                        .ok();
                    ports.iter().any(|old| {
                        old["name"] == p.name
                            && old["operation_version"] == c.operation_version
                            && old["schema_version"] == c.schema_version
                            && old["schema_sha256"].as_str() == hash.as_deref()
                    })
                })
        })
    });
    if !current || record.provenance.core_version != execute::core_version() {
        report.gap(id, None, "contract_changed", "The recorded operation/schema or Core version differs from the installed version. Viewing historical output does not prove replay compatibility.");
    }
    if let Some(c) = &contract
        && c.inputs.iter().any(|port| {
            let count = record
                .provenance
                .inputs
                .iter()
                .filter(|input| input.reference["input_port"] == port.name)
                .count();
            count > 1 || (port.required && count != 1)
        })
    {
        report.gap(
            id,
            None,
            "input_evidence_missing",
            "The recorded inputs do not completely identify the required named ports.",
        );
    }
    if record
        .provenance
        .parameters
        .iter()
        .any(|p| p["redacted"] == true)
    {
        report.gap(id, None, "private_parameters_required", "Private parameters were redacted. Supply them explicitly for a new run; they cannot be recovered from this history.");
    }
    let configuration = app
        .database()
        .await?
        .ok_or_else(Error::storage)?
        .result_configuration(id)
        .await?;
    let step = configuration
        .as_ref()
        .and_then(|(definition, step)| definition.steps.iter().find(|s| s.id == *step));
    if configuration.is_some() && step.is_none() {
        report.gap(
            id,
            None,
            "configuration_unavailable",
            "The recorded step is unavailable in its immutable configuration.",
        );
    }
    if record.provenance.inputs.len() > 32 {
        return Err(Error::storage());
    }
    for input in &record.provenance.inputs {
        if report.checked_bytes >= MAX_CHECKED_BYTES {
            report.gap(
                id,
                None,
                "check_limit",
                "The bounded evidence check could not inspect the complete dependency path.",
            );
            break;
        }
        let port = input.reference["input_port"].as_str();
        if let Some(c) = &contract {
            let schema = port.and_then(|name| c.inputs.iter().find(|p| p.name == name));
            let hash = schema
                .and_then(|p| execute::json_bytes(&p.schema, 128 * 1024).ok())
                .map(|v| cube_core::hashing::sha256(&v));
            if hash.is_none() || hash.as_deref() != input.reference["schema_sha256"].as_str() {
                report.gap(
                    id,
                    port,
                    "input_schema_changed",
                    "The input schema identity is missing or differs from the installed contract.",
                );
            }
        }
        let binding = match input.reference["type"].as_str() {
            Some("file") if input.reference["digest_representation"] == "file_bytes" => {
                let store = serde_json::from_value(input.reference["store"].clone())
                    .map_err(|_| Error::storage())?;
                let path = input.reference["path"]
                    .as_str()
                    .ok_or_else(Error::storage)?
                    .to_owned();
                // Compare original bytes, without decoding an image or analyzing content.
                Some(typed::Binding::File {
                    store,
                    path,
                    representation: Representation::Bytes,
                })
            }
            Some("artifact") if input.reference["digest_representation"] == "json_wire_value" => {
                let upstream = serde_json::from_value(input.reference["id"].clone())
                    .map_err(|_| Error::storage())?;
                queue.push_back(upstream);
                let port = input.reference["port"]
                    .as_str()
                    .ok_or_else(Error::storage)?
                    .to_owned();
                Some(typed::Binding::Artifact { id: upstream, port })
            }
            Some("literal") if input.reference["digest_representation"] == "json_wire_value" => {
                port.and_then(|p| step.and_then(|step| step.inputs.get(p)))
                    .and_then(|source| match source {
                        Source::Literal { value } => Some(typed::Binding::Literal {
                            value: value.clone(),
                        }),
                        _ => None,
                    })
            }
            _ => None,
        };
        let Some(binding) = binding else {
            report.gap(id, port, "input_unavailable", "This input was not retained in a tracked configuration. Replay needs an explicit source; none is inferred.");
            continue;
        };
        match typed::resolve(app, storage.clone(), binding).await {
            Ok((_, current)) => {
                report.checked_bytes += current.byte_size;
                if current.sha256 != input.sha256 || current.byte_size != input.byte_size {
                    report.gap(id, port, "source_changed", "The current input differs from the recorded digest. The original result was preserved; no source was modified or copied.");
                }
            }
            Err(_) => report.gap(id, port, "source_unavailable", "An authorized input is missing, inaccessible, corrupt or exceeds the current reading budget. The original result remains available."),
        }
    }
    Ok(())
}
