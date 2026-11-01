//! Resolve selected immutable evidence; exports never execute an operation or edit history.
use super::{
    candidates, image_views,
    model::{CandidateSelection, Context},
    solutions, storage,
};
use crate::Result;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::SqliteConnection;
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Source {
    Asset {
        asset_id: Uuid,
        revision: i64,
    },
    RunOutput {
        run_id: Uuid,
        attempt_id: Uuid,
        port: String,
    },
    Solution {
        solution_id: Uuid,
        revision: i64,
    },
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Format {
    Data,
    Document,
}
pub(super) struct Payload {
    pub name: String,
    pub mime: &'static str,
    pub bytes: Vec<u8>,
}
fn json_bytes(value: &Value) -> Result<Vec<u8>> {
    serde_json::to_vec(value).map_err(|_| "invalid_state")
}
fn payload(name: String, kind: &str, bytes: Vec<u8>) -> Payload {
    let (extension, mime) = match kind {
        "text" | "page" => ("txt", "text/plain; charset=utf-8"),
        "image" => ("png", "image/png"),
        "json" => ("json", "application/json"),
        _ => ("bin", "application/octet-stream"),
    };
    Payload {
        name: format!("{name}.{extension}"),
        mime,
        bytes,
    }
}
async fn data(
    c: &mut SqliteConnection,
    context: &Context,
    selected: &CandidateSelection,
) -> Result<(Payload, Value)> {
    let evidence = candidates::evidence(c, context, selected).await?;
    let output = match selected {
        CandidateSelection::Asset { asset_id, revision } => {
            let kind = evidence["asset"]["content"]["kind"]
                .as_str()
                .ok_or("corrupt_store")?;
            let bytes = if kind == "structured" {
                json_bytes(&evidence["asset"]["content"]["value"])?
            } else {
                sqlx::query_scalar::<_,Vec<u8>>("SELECT data FROM asset_revisions WHERE workspace_id=? AND exploration_id=? AND asset_id=? AND revision=?")
                    .bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(asset_id.to_string()).bind(revision).fetch_one(c).await.map_err(storage::failure)?
            };
            payload(
                format!("asset-{asset_id}-r{revision}"),
                if kind == "structured" { "json" } else { kind },
                bytes,
            )
        }
        CandidateSelection::RunOutput { run_id, port, .. } => {
            let representation = evidence["representation"]
                .as_str()
                .ok_or("contract_mismatch")?;
            let value = &evidence["value"];
            let (kind, bytes) = match representation {
                "text" => (
                    "text",
                    value.as_str().ok_or("corrupt_store")?.as_bytes().to_vec(),
                ),
                "bytes" => (
                    "file",
                    STANDARD
                        .decode(value["base64"].as_str().ok_or("corrupt_store")?)
                        .map_err(|_| "corrupt_store")?,
                ),
                "byte_array" => (
                    "file",
                    serde_json::from_value::<Vec<u8>>(value.clone())
                        .map_err(|_| "corrupt_store")?,
                ),
                "json" => ("json", json_bytes(value)?),
                "image_samples" => ("image", image_views::encode(value)?),
                _ => return Err("contract_mismatch"),
            };
            payload(format!("run-{run_id}-{port}"), kind, bytes)
        }
    };
    Ok((output, evidence))
}
fn document(
    name: String,
    context: &Context,
    source: &Source,
    data: &Payload,
    evidence: Value,
) -> Result<Payload> {
    Ok(payload(
        name,
        "json",
        json_bytes(
            &json!({"format":"cube-play.evidence","formatVersion":1,"context":context,"source":source,"evidence":evidence,"content":{"mime":data.mime,"encoding":"base64","sha256":cube_core::hashing::sha256(&data.bytes),"base64":STANDARD.encode(&data.bytes)}}),
        )?,
    ))
}
pub(super) async fn prepare(
    c: &mut SqliteConnection,
    context: &Context,
    source: &Source,
    format: Format,
) -> Result<Payload> {
    let selected = match source {
        Source::Asset { asset_id, revision } => CandidateSelection::Asset {
            asset_id: *asset_id,
            revision: *revision,
        },
        Source::RunOutput {
            run_id,
            attempt_id,
            port,
        } => CandidateSelection::RunOutput {
            run_id: *run_id,
            attempt_id: *attempt_id,
            port: port.clone(),
        },
        Source::Solution {
            solution_id,
            revision,
        } => return walkthrough(c, context, *solution_id, *revision, format).await,
    };
    let (data, evidence) = data(c, context, &selected).await?;
    match format {
        Format::Data => Ok(data),
        Format::Document => document(
            format!("{}.evidence", data.name),
            context,
            source,
            &data,
            evidence,
        ),
    }
}
async fn walkthrough(
    c: &mut SqliteConnection,
    context: &Context,
    id: Uuid,
    revision: i64,
    format: Format,
) -> Result<Payload> {
    if !matches!(format, Format::Document) {
        return Err("invalid_command");
    }
    let solution = solutions::read(c, context, id, Some(revision)).await?;
    let selected = serde_json::from_value(solution["snapshot"]["selection"].clone())
        .map_err(|_| "corrupt_store")?;
    let content = if solution["evidence"]["kind"] == "unavailable" {
        solution["evidence"].clone()
    } else {
        let (data, _) = data(c, context, &selected).await?;
        json!({"mime":data.mime,"encoding":"base64","sha256":cube_core::hashing::sha256(&data.bytes),"base64":STANDARD.encode(&data.bytes)})
    };
    let value = json!({"format":"cube-play.walkthrough","formatVersion":1,"context":context,"solution":{"id":id,"revision":revision,"title":solution["title"],"explanation":solution["explanation"],"status":"private_unverified","snapshot":solution["snapshot"],"evidence":solution["evidence"],"content":content,"gaps":solution["gaps"]}});
    Ok(payload(
        format!("solution-{id}-r{revision}"),
        "json",
        json_bytes(&value)?,
    ))
}
