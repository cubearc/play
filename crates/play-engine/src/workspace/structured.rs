//! Typed structured content retains its exact value; files never parse implicitly.
use super::{
    model::{Content, Context},
    mutations,
};
use crate::{Result, protocol};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::SqliteConnection;

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Document {
    semantic_type: String,
    value: Value,
}
fn valid_semantic(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}
pub(super) fn encode(semantic: &str, value: &Value) -> Result<Vec<u8>> {
    if !valid_semantic(semantic) {
        return Err("invalid_asset_type");
    }
    if serde_json::to_vec(value)
        .map_err(|_| "invalid_binding")?
        .len()
        > protocol::contract().output_bytes
    {
        return Err("output_limit");
    }
    serde_json::to_vec(&Document {
        semantic_type: semantic.into(),
        value: value.clone(),
    })
    .map_err(|_| "invalid_binding")
}
pub(super) fn content(data: &[u8]) -> Result<Content> {
    if data.len() > protocol::contract().output_bytes + 256 {
        return Err("corrupt_store");
    }
    let document: Document = serde_json::from_slice(data).map_err(|_| "corrupt_store")?;
    if encode(&document.semantic_type, &document.value).map_err(|_| "corrupt_store")? != data {
        return Err("corrupt_store");
    }
    Ok(Content::Structured {
        semantic_type: document.semantic_type,
        value: document.value,
    })
}
pub(super) async fn add(
    connection: &mut SqliteConnection,
    context: &Context,
    title: &str,
    semantic: &str,
    value: &Value,
) -> Result<Value> {
    if crate::worker::asset_representation(semantic)?
        != cube_core::operations::execution::Representation::Json
    {
        return Err("invalid_asset_type");
    }
    let data = encode(semantic, value)?;
    mutations::add_block(connection, context, title, "structured", &data, None)
        .await
        .map(mutations::AddedBlock::value)
}
