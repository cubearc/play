//! Private typed stream transport. Core owns algorithms, bounds and profiles.
use cube_core::{
    ciphers::stream::{self, Profile},
    errors::{ErrorCode, OperationError},
};
use serde_json::{Value, json};
pub(super) fn supported(op: &str) -> bool {
    op == "cipher.vernam" || Profile::from_id(op).is_some()
}
fn invalid() -> OperationError {
    OperationError::new(
        ErrorCode::InvalidParameter,
        "use exact declared stream inputs and explicit nonce/counter",
    )
}
pub(super) fn execute(op: &str, inputs: Value, p: Value) -> Result<Value, OperationError> {
    let input = super::bytes(&inputs["input"])?;
    let key = super::bytes(&inputs["key"])?;
    let output = if let Some(profile) = Profile::from_id(op) {
        let nonce: Vec<u8> = super::field(&p, "nonce")?;
        let counter = p["counter"]
            .as_u64()
            .or_else(|| p["counter"].as_str()?.parse().ok())
            .ok_or_else(invalid)?;
        stream::transform(profile, &input, &key, &nonce, counter)?
    } else if op == "cipher.vernam" {
        stream::vernam(&input, &key)?
    } else {
        return Err(invalid());
    };
    Ok(json!({"output":{"base64":cube_core::encoding::base64_encode(&output)}}))
}
