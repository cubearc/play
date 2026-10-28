//! Explicit adapters call typed Core APIs; descriptors never dispatch algorithms.
mod audio;
mod blockchain;
mod brainfuck;
mod cryptanalysis;
mod files;
mod fraction;
mod image_analysis;
mod images;
mod linear;
mod mono;
mod numbers;
mod poly;
mod puzzle_codes;
mod reasoning;
mod symbols;
mod text;
mod text_analysis;
mod video;
mod workflows;

use cube_core::errors::{ErrorCode, OperationError};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

pub(super) fn supported(operation: &str) -> bool {
    cryptanalysis::supported(operation)
        || workflows::supported(operation)
        || video::supported(operation)
        || blockchain::supported(operation)
        || brainfuck::supported(operation)
        || audio::supported(operation)
        || image_analysis::supported(operation)
        || files::supported(operation)
        || text_analysis::supported(operation)
        || reasoning::supported(operation)
        || symbols::supported(operation)
        || puzzle_codes::supported(operation)
        || fraction::supported(operation)
        || cube_core::ciphers::mono::OPERATIONS
            .iter()
            .any(|d| d.id == operation)
        || cube_core::ciphers::poly::OPERATIONS
            .iter()
            .any(|d| d.id == operation)
        || linear::supported(operation)
        || numbers::supported(operation)
        || text::supported(operation)
        || images::supported(operation)
        || matches!(
            operation,
            "encoding.base58check.encode" | "encoding.base58check.decode"
        )
}
fn field<T: DeserializeOwned>(value: &Value, name: &str) -> Result<T, OperationError> {
    serde_json::from_value(value.get(name).cloned().unwrap_or(Value::Null)).map_err(|_| {
        OperationError::new(ErrorCode::InvalidInput, "use the exact typed input fields")
    })
}
pub(super) fn bytes(value: &Value) -> Result<Vec<u8>, OperationError> {
    let encoded: String = field(value, "base64")?;
    cube_core::encoding::base64_decode(&encoded)
        .map_err(|_| OperationError::new(ErrorCode::InvalidInput, "use canonical Base64 bytes"))
}
pub(super) fn execute(
    operation: &str,
    inputs: Value,
    parameters: Value,
) -> Result<Value, OperationError> {
    match operation {
        _ if cryptanalysis::supported(operation) => {
            cryptanalysis::execute(operation, inputs, parameters)
        }
        "encoding.base58check.encode" => {
            let encoded: String = field(&inputs["input"], "base64")?;
            let bytes = cube_core::encoding::base64_decode(&encoded).map_err(|_| {
                OperationError::new(ErrorCode::InvalidInput, "use canonical Base64 bytes")
            })?;
            let version: u8 = field(&parameters, "version")?;
            Ok(json!({"output":cube_core::encoding::base58check::encode(version,&bytes)?}))
        }
        "encoding.base58check.decode" => {
            let input: String = field(&inputs, "input")?;
            Ok(json!({"output":cube_core::encoding::base58check::decode(&input)?}))
        }
        _ if puzzle_codes::supported(operation) => {
            puzzle_codes::execute(operation, inputs, parameters)
        }
        _ if symbols::supported(operation) => symbols::execute(operation, inputs, parameters),
        _ if reasoning::supported(operation) => reasoning::execute(operation, inputs, parameters),
        _ if text_analysis::supported(operation) => {
            text_analysis::execute(operation, inputs, parameters)
        }
        _ if files::supported(operation) => files::execute(operation, inputs, parameters),
        _ if audio::supported(operation) => audio::execute(operation, inputs, parameters),
        _ if brainfuck::supported(operation) => brainfuck::execute(operation, inputs, parameters),
        _ if blockchain::supported(operation) => blockchain::execute(operation, inputs, parameters),
        _ if video::supported(operation) => video::execute(operation, inputs, parameters),
        _ if workflows::supported(operation) => workflows::execute(operation, inputs, parameters),
        _ if image_analysis::supported(operation) => {
            image_analysis::execute(operation, inputs, parameters)
        }
        _ if fraction::supported(operation) => fraction::execute(operation, inputs, parameters),
        _ if linear::supported(operation) => linear::execute(operation, inputs, parameters),
        _ if numbers::supported(operation) => numbers::execute(operation, inputs, parameters),
        _ if text::supported(operation) => text::execute(operation, inputs, parameters),
        _ if images::supported(operation) => images::execute(operation, inputs, parameters),
        _ if cube_core::ciphers::poly::OPERATIONS
            .iter()
            .any(|d| d.id == operation) =>
        {
            poly::execute(operation, inputs, parameters)
        }
        _ => mono::execute(operation, inputs, parameters),
    }
}
