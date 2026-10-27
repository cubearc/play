//! Explicit bounded string-field selection; no expressions or implicit decoding.
use super::types::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Projection {
    pub pointer: String,
    pub conversion: Conversion,
}
#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Conversion {
    Text,
    Base64Bytes,
}
impl Projection {
    pub(crate) fn valid(&self) -> bool {
        self.pointer.starts_with('/')
            && self.pointer.len() <= 512
            && self.pointer.split('/').count() <= 17
            && self
                .pointer
                .split('~')
                .skip(1)
                .all(|s| s.starts_with(['0', '1']))
    }
    pub(crate) fn semantic_type(&self) -> &'static str {
        match self.conversion {
            Conversion::Text => "text",
            Conversion::Base64Bytes => "bytes",
        }
    }
    pub(crate) fn select(&self, value: &Value) -> Result<Value> {
        if !self.valid() {
            return Err(Error::invalid());
        }
        let text = value.pointer(&self.pointer).and_then(Value::as_str)
            .ok_or_else(|| Error::new("contract_mismatch", "Select an existing string field with an explicit text or Base64-byte conversion."))?;
        Ok(match self.conversion {
            Conversion::Text => json!(text),
            Conversion::Base64Bytes => {
                let bytes =
                    cube_core::encoding::base64_decode(text).map_err(|_| Error::invalid())?;
                json!({"base64":cube_core::encoding::base64_encode(&bytes)})
            }
        })
    }
}
