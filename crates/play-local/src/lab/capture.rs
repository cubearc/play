//! Publish a validated declaration without executing, inferring or locating inputs.
use super::{execute, typed, types::*};
use crate::AppState;
use cube_core::investigation::session::typed_capture::TypedCapture;
use serde_json::{Value, json};
use uuid::Uuid;

pub(crate) async fn publish(
    app: &AppState,
    staged_id: Option<&str>,
    bytes: Vec<u8>,
) -> Result<Value> {
    let permit = app.lab.slots.clone().try_acquire_owned().map_err(|_| {
        Error::new(
            "busy",
            "One Lab operation is already active; retry publication.",
        )
    })?;
    let storage = app.lab.storage(app).await?;
    let staged_id = staged_id.map(str::to_owned);
    tokio::task::spawn_blocking(move || {
        let _permit = permit;
        let document = std::str::from_utf8(&bytes).map_err(|_| Error::invalid())?;
        let capture = TypedCapture::from_json(document).map_err(|_| Error::new(
            "invalid_capture",
            "The capture format, contract or content digest is invalid. Finish the command or regenerate the capture before publishing.",
        ))?;
        if staged_id.as_ref().is_some_and(|id| capture.capture_id != *id)
            || capture.core_version != execute::core_version()
        {
            return Err(Error::new(
                "contract_mismatch",
                "The staged capture identity or Core version does not match this runtime.",
            ));
        }
        let digest = cube_core::hashing::sha256(&bytes);
        let hash = cube_core::encoding::hex_decode(&digest).map_err(|_| Error::invalid())?;
        let mut identity = [0u8; 16];
        identity.copy_from_slice(&hash[..16]);
        identity[6] = (identity[6] & 15) | 0x40;
        identity[8] = (identity[8] & 63) | 0x80;
        let id = Uuid::from_bytes(identity);
        if let Ok((record, _)) = storage.load(id) {
            if record.metadata["terminal_capture"]["sha256"] != digest {
                return Err(Error::storage());
            }
            return Ok(json!({"type":"published","id":id,"operation":capture.operation}));
        }
        let contract = cube_core::operations::require(&capture.operation)?.contract()?;
        let mut output = typed::finish(&contract, &capture.parameters, capture.outputs)?;
        output.data.metadata["terminal_capture"] = json!({
            "capture_id":capture.capture_id,
            "sha256":digest,
            "trust":"unverified_cli_declaration",
            "replay_gaps":capture.replay_gaps
        });
        let inputs = capture.inputs.into_iter().map(|input| {
            let schema = contract.inputs.iter().find(|p| p.name == input.name)
                .ok_or_else(Error::invalid)?;
            Ok(InputRecord {
                reference: json!({
                    "type":"cli_capture",
                    "input_port":input.name,
                    "schema_sha256":cube_core::hashing::sha256(&execute::json_bytes(&schema.schema,128*1024)?),
                    "digest_representation":"json_wire_value",
                    "retained":false
                }),
                semantic_type: input.semantic_type,
                byte_size: usize::try_from(input.identity.size).map_err(|_| Error::limit())?,
                sha256: input.identity.digest.ok_or_else(Error::invalid)?.value,
            })
        }).collect::<Result<Vec<_>>>()?;
        let provenance = Provenance {
            operation: capture.operation.clone(),
            adapter_api: 2,
            pipeline_adapter: contract.pipeline_adapter,
            engine_api: capture.engine_api,
            core_version: capture.core_version,
            parameters: output.parameters.clone(),
            inputs,
            created_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| Error::storage())?
                .as_millis().try_into().map_err(|_| Error::storage())?,
        };
        storage.save(id, output, provenance)?;
        Ok(json!({"type":"published","id":id,"operation":capture.operation}))
    }).await.map_err(|_|Error::storage())?
}

pub(super) fn routes() -> axum::Router<AppState> {
    axum::Router::new().route("/lab/captures/import", axum::routing::post(import))
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Import {
    store: crate::workspace::Store,
    path: String,
}
async fn import(
    axum::extract::State(app): axum::extract::State<AppState>,
    request: axum::extract::Request,
) -> Result<axum::Json<Value>> {
    use std::io::Read;
    let selection: Import = serde_json::from_value(super::request_json(request).await?)
        .map_err(|_| Error::invalid())?;
    let workspace = app
        .workspace
        .as_ref()
        .map_err(|e| Error::from(*e))?
        .clone()
        .ok_or_else(Error::invalid)?;
    let permit = crate::workspace::permit(&workspace).await?;
    let bytes = tokio::task::spawn_blocking(move || -> Result<Vec<u8>> {
        let _permit = permit;
        let file = workspace.file(selection.store, &selection.path)?;
        let mut bytes = Vec::new();
        file.take(play_runner::protocol::CAPTURE_LIMIT as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Error::invalid())?;
        if bytes.len() > play_runner::protocol::CAPTURE_LIMIT {
            return Err(Error::limit());
        }
        Ok(bytes)
    })
    .await
    .map_err(|_| Error::storage())??;
    Ok(axum::Json(publish(&app, None, bytes).await?))
}
