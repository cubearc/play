//! Typed composition adapter; all data transformations belong to Core.
use super::{bytes, field};
use cube_core::{composition, errors::OperationError, operations::contracts::values};
use serde_json::{Value, json};
pub(super) fn supported(operation: &str) -> bool {
    matches!(operation, "text.concat" | "bytes.concat")
}
pub(super) fn execute(
    operation: &str,
    inputs: Value,
    parameters: Value,
) -> Result<Value, OperationError> {
    Ok(match operation {
        "text.concat" => {
            json!({"output":composition::concat_text(&field::<String>(&inputs,"left")?, &field::<String>(&inputs,"right")?, &field::<String>(&parameters,"separator")?)?})
        }
        _ => {
            json!({"output":values::Bytes { data:composition::concat_bytes(&bytes(&inputs["left"])?, &bytes(&inputs["right"])?)? }})
        }
    })
}
