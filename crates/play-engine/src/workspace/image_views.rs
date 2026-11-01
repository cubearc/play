//! Scoped image previews and storage encoding through the existing Core codec.
use super::{
    candidates,
    model::{CandidateSelection, Context},
    storage,
};
use crate::{Result, protocol};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use cube_core::operations::execution::{self as core, Budget, Representation};
use serde_json::{Value, json};
use sqlx::SqliteConnection;

/// PNG is the Asset storage/view format; encoding never creates a node attempt.
pub(super) fn encode(value: &Value) -> Result<Vec<u8>> {
    if value.to_string().len() > protocol::contract().output_bytes {
        return Err("output_limit");
    }
    let output = core::execute(
        "image.encode",
        1,
        &json!({"input":value}),
        &json!({}),
        Budget {
            input_bytes: protocol::contract().output_bytes,
            output_bytes: protocol::contract().image_bytes,
            ..Budget::default()
        },
    )
    .map_err(|e| e.0)?;
    let bytes = STANDARD
        .decode(
            output["output"]["base64"]
                .as_str()
                .ok_or("invalid_response")?,
        )
        .map_err(|_| "invalid_response")?;
    if bytes.len() > protocol::contract().image_bytes {
        return Err("output_limit");
    }
    Ok(bytes)
}

pub(super) async fn read(
    connection: &mut SqliteConnection,
    context: &Context,
    source: &CandidateSelection,
) -> Result<Value> {
    // Resolve the same exact, retained, Stage-scoped evidence as Candidate selection.
    let evidence = candidates::evidence(connection, context, source).await?;
    let bytes = match source {
        CandidateSelection::Asset { asset_id, revision } => {
            if evidence["asset"]["content"]["kind"] != "image" {
                return Err("invalid_image");
            }
            sqlx::query_scalar::<_,Vec<u8>>("SELECT data FROM asset_revisions WHERE workspace_id=? AND exploration_id=? AND asset_id=? AND revision=?")
                .bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(asset_id.to_string()).bind(revision)
                .fetch_one(connection).await.map_err(storage::failure)?
        }
        CandidateSelection::RunOutput { .. } => {
            let semantic = evidence["semanticType"].as_str().ok_or("corrupt_store")?;
            if crate::worker::asset_representation(semantic)? != Representation::ImageSamples {
                return Err("invalid_image");
            }
            encode(&evidence["value"])?
        }
    };
    if bytes.len() > protocol::contract().image_bytes {
        return Err("output_limit");
    }
    Ok(json!({"mime":"image/png","base64":STANDARD.encode(bytes)}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn png_storage_preserves_indexed_samples_palette_and_transparency() {
        let logical = json!({"width":2,"height":1,"color_model":"indexed","samples":{"base64":"AAE="},"palette":[[1,2,3],[4,5,6]],"transparency":{"base64":"AH8="}});
        let png = encode(&logical).unwrap();
        let decoded = core::execute(
            "image.decode",
            1,
            &json!({"input":{"base64":STANDARD.encode(&png)}}),
            &json!({}),
            Budget::default(),
        )
        .unwrap();
        assert_eq!(decoded["output"], logical);
        let mut oversized = logical;
        oversized["samples"]["base64"] = json!("A".repeat(protocol::contract().output_bytes));
        assert_eq!(encode(&oversized), Err("output_limit"));
    }
}
