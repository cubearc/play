//! Typed native adapters; encoding and ordered composition execute only in Core.
use crate::{Result, protocol};
use serde_json::{Value, json};

pub(super) fn supported(operation: &str) -> bool {
    matches!(
        operation,
        "text.utf8.encode" | "text.utf8.decode" | "text.concat" | "bytes.concat"
    )
}
pub(super) fn parameters(operation: &str) -> Result<Value> {
    if !supported(operation) {
        return Err("unsupported_operation");
    }
    let contract = cube_core::operations::require(operation)
        .map_err(|_| "unsupported_operation")?
        .contract()
        .map_err(|_| "contract_mismatch")?;
    let properties = contract.parameters.as_value()["properties"]
        .as_object()
        .ok_or("contract_mismatch")?;
    Ok(Value::Object(
        properties
            .iter()
            .filter_map(|(name, schema)| schema.get("default").map(|v| (name.clone(), v.clone())))
            .collect(),
    ))
}
fn text<'a>(inputs: &'a Value, port: &str) -> Result<&'a str> {
    inputs[port].as_str().ok_or("contract_mismatch")
}
fn bytes(inputs: &Value, port: &str) -> Result<Vec<u8>> {
    let value = inputs[port]["base64"].as_str().ok_or("contract_mismatch")?;
    if value.len() > protocol::contract().input_bytes.div_ceil(3) * 4 {
        return Err("input_limit");
    }
    cube_core::encoding::base64_decode(value).map_err(|_| "invalid_bytes")
}
pub(super) fn validate_inputs(operation: &str, inputs: &Value, parameters: &Value) -> Result<()> {
    let lengths = match operation {
        "text.utf8.encode" => vec![text(inputs, "input")?.len()],
        "text.utf8.decode" => vec![bytes(inputs, "input")?.len()],
        "text.concat" => vec![
            text(inputs, "left")?.len(),
            text(inputs, "right")?.len(),
            parameters
                .get("separator")
                .map_or(Ok(""), |v| v.as_str().ok_or("invalid_parameters"))?
                .len(),
        ],
        "bytes.concat" => vec![bytes(inputs, "left")?.len(), bytes(inputs, "right")?.len()],
        _ => return Err("unsupported_operation"),
    };
    let total = lengths.iter().try_fold(0usize, |sum, length| {
        sum.checked_add(*length).ok_or("input_limit")
    })?;
    if total > protocol::contract().input_bytes {
        return Err("input_limit");
    }
    Ok(())
}
pub(super) fn execute(operation: &str, inputs: &Value, parameters: &Value) -> Result<Value> {
    validate_inputs(operation, inputs, parameters)?;
    match operation {
        "text.utf8.encode" => Ok(
            json!({"bytes":cube_core::text::to_utf8(text(inputs,"input")?).map_err(|_| "operation_failed")?}),
        ),
        "text.utf8.decode" => Ok(
            json!({"text":cube_core::text::from_utf8(&bytes(inputs,"input")?).map_err(|_| "invalid_encoding")?}),
        ),
        "text.concat" => Ok(
            json!({"text":cube_core::composition::concat_text(text(inputs,"left")?, text(inputs,"right")?, parameters.get("separator").and_then(Value::as_str).unwrap_or("")).map_err(|_| "operation_failed")?}),
        ),
        "bytes.concat" => Ok(
            json!({"bytes":cube_core::composition::concat_bytes(&bytes(inputs,"left")?, &bytes(inputs,"right")?).map_err(|_| "operation_failed")?}),
        ),
        _ => Err("unsupported_operation"),
    }
}
