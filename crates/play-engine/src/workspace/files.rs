use super::{
    model::{Content, Context, FilePreview},
    mutations,
};
use crate::{Result, protocol};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::Value;
use sqlx::SqliteConnection;

pub(super) async fn add(
    connection: &mut SqliteConnection,
    context: &Context,
    title: &str,
    encoded: &str,
) -> Result<Value> {
    let maximum = protocol::contract().file_bytes;
    if encoded.len() > maximum.div_ceil(3) * 4 {
        return Err("file_limit");
    }
    let data = STANDARD.decode(encoded).map_err(|_| "invalid_file")?;
    if data.len() > maximum {
        return Err("file_limit");
    }
    mutations::add_block(connection, context, title, "file", &data, None).await
}

pub(super) fn content(data: &[u8], sha256: String) -> Result<Content> {
    let limits = protocol::contract();
    if data.len() > limits.file_bytes {
        return Err("corrupt_store");
    }
    let end = data.len().min(limits.file_preview_bytes);
    let preview = match std::str::from_utf8(data) {
        Ok(text)
            if text
                .chars()
                .all(|c| !c.is_control() || matches!(c, '\n' | '\r' | '\t')) =>
        {
            let end = text.floor_char_boundary(end);
            FilePreview::Utf8 {
                text: text[..end].into(),
                truncated: end < data.len(),
            }
        }
        _ => FilePreview::Hex {
            text: cube_core::encoding::hex_encode(&data[..end]),
            truncated: end < data.len(),
        },
    };
    Ok(Content::File {
        byte_length: data.len(),
        sha256,
        preview,
    })
}
