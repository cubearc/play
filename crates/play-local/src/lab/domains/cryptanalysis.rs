//! Explicit typed Core calls; no execution through operation metadata.
use cube_core::{
    cryptanalysis as a,
    errors::{ErrorCode, OperationError},
};
use serde::Deserialize;
use serde_json::{Value, json};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    request: Value,
}
pub(super) fn supported(op: &str) -> bool {
    matches!(op, "analysis.language.score" | "analysis.period.estimate")
}
pub(super) fn execute(op: &str, inputs: Value, parameters: Value) -> Result<Value, OperationError> {
    if parameters != json!({}) {
        return Err(OperationError::new(
            ErrorCode::InvalidParameter,
            "analysis options belong to the typed request",
        ));
    }
    let input: Input = serde_json::from_value(inputs).map_err(|_| {
        OperationError::new(
            ErrorCode::InvalidInput,
            "use exactly the named request input",
        )
    })?;
    let document = serde_json::to_string(&input.request)
        .map_err(|_| OperationError::new(ErrorCode::InvalidInput, "invalid request JSON"))?;
    match op {
        "analysis.language.score" => {
            let request: a::scoring::Request = a::parse_request(&document)?;
            Ok(json!({"report":a::scoring::score(&request,None)?}))
        }
        "analysis.period.estimate" => {
            let request: a::period::Request = a::parse_request(&document)?;
            Ok(json!({"report":a::period::estimate(&request,None)?}))
        }
        _ => Err(OperationError::new(
            ErrorCode::UnsupportedFormat,
            "unsupported classical analysis",
        )),
    }
}
