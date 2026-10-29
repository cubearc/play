//! Native transport for Core-owned structured and integer composition.
use crate::{Result, protocol};
use cube_core::{
    composition::structured as core,
    errors::{ErrorCode, OperationError},
};
use serde_json::{Value, json};
pub(super) fn supported(id: &str) -> bool {
    matches!(
        id,
        "json.parse"
            | "json.select"
            | "json.text"
            | "text.integer"
            | "number.maximum"
            | "number.add"
    )
}
pub(super) fn validate_inputs(id: &str, inputs: &Value, parameters: &Value) -> Result<()> {
    let values = inputs.as_object().ok_or("contract_mismatch")?;
    let bytes = values.values().try_fold(0usize, |sum, value| {
        sum.checked_add(
            value
                .as_str()
                .map_or_else(|| value.to_string().len(), str::len),
        )
        .ok_or("input_limit")
    })?;
    let pointer = if id == "json.select" {
        parameters
            .get("pointer")
            .and_then(Value::as_str)
            .unwrap_or("")
            .len()
    } else {
        0
    };
    if bytes
        .checked_add(pointer)
        .is_none_or(|n| n > protocol::contract().input_bytes)
    {
        return Err("input_limit");
    }
    Ok(())
}
fn failure(error: OperationError) -> &'static str {
    match error.code {
        ErrorCode::ResourceLimit
            if matches!(
                error.operation,
                Some("number.add" | "number.maximum" | "text.integer")
            ) =>
        {
            "numeric_overflow"
        }
        ErrorCode::ResourceLimit => "input_limit",
        ErrorCode::InvalidParameter => "invalid_pointer",
        _ => match error.operation {
            Some("json.select") => "missing_field",
            Some("json.parse") => "invalid_json",
            Some("json.text") => "invalid_type",
            _ => "invalid_number",
        },
    }
}
pub(super) fn execute(id: &str, inputs: &Value, parameters: &Value) -> Result<Value> {
    validate_inputs(id, inputs, parameters)?;
    let input = &inputs["input"];
    let value = match id {
        "json.parse" => {
            core::parse_json(input.as_str().ok_or("contract_mismatch")?).map_err(failure)?
        }
        "json.select" => core::select_field(
            input,
            parameters
                .get("pointer")
                .and_then(Value::as_str)
                .unwrap_or(""),
        )
        .map_err(failure)?,
        "json.text" => return Ok(json!({"text":core::json_text(input).map_err(failure)?})),
        "text.integer" => {
            json!(core::parse_integer(input.as_str().ok_or("contract_mismatch")?).map_err(failure)?)
        }
        "number.maximum" => {
            let values: Vec<i64> =
                serde_json::from_value(input.clone()).map_err(|_| "invalid_number")?;
            json!(core::maximum(&values).map_err(failure)?)
        }
        "number.add" => json!(
            core::add(
                inputs["left"].as_i64().ok_or("invalid_number")?,
                inputs["right"].as_i64().ok_or("invalid_number")?
            )
            .map_err(failure)?
        ),
        _ => return Err("unsupported_operation"),
    };
    Ok(json!({"value":value}))
}
