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
    matches!(
        op,
        "analysis.language.score"
            | "analysis.period.estimate"
            | "analysis.caesar.rank"
            | "analysis.affine.rank"
            | "analysis.vigenere.rank"
            | "analysis.transposition.rank"
            | "analysis.homophonic.inspect"
            | "analysis.crib.verify"
            | "analysis.frequency.compare"
    )
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
        "analysis.caesar.rank" => {
            let request: a::classical::Caesar = a::parse_request(&document)?;
            Ok(json!({"report":a::classical::caesar(&request,None)?}))
        }
        "analysis.affine.rank" => {
            let request: a::classical::Affine = a::parse_request(&document)?;
            Ok(json!({"report":a::classical::affine(&request,None)?}))
        }
        "analysis.vigenere.rank" => {
            let request: a::vigenere::Request = a::parse_request(&document)?;
            Ok(json!({"report":a::vigenere::rank(&request,None)?}))
        }
        "analysis.transposition.rank" => {
            let request: a::transposition::Request = a::parse_request(&document)?;
            Ok(json!({"report":a::transposition::rank(&request,None)?}))
        }
        "analysis.homophonic.inspect" => {
            let request: a::homophonic::Request = a::parse_request(&document)?;
            Ok(json!({"report":a::homophonic::inspect(&request,None)?}))
        }
        "analysis.crib.verify" => {
            let request: a::crib::Request = a::parse_request(&document)?;
            Ok(json!({"report":a::crib::verify(&request,None)?}))
        }
        "analysis.frequency.compare" => {
            let request: a::frequency::Request = a::parse_request(&document)?;
            Ok(json!({"report":a::frequency::compare(&request,None)?}))
        }
        _ => Err(OperationError::new(
            ErrorCode::UnsupportedFormat,
            "unsupported classical analysis",
        )),
    }
}
