//! Private, bounded byte transport; AES and validation remain in Core.
use cube_core::{
    ciphers::aes_blocks as aes, errors::OperationError, operations::contracts::values::Bytes,
};
use serde_json::Value;
use zeroize::Zeroizing;
pub(super) fn supported(operation: &str) -> bool {
    matches!(operation, aes::ENCRYPT | aes::DECRYPT)
}
pub(super) fn execute(
    operation: &str,
    inputs: Value,
    parameters: Value,
) -> Result<Value, OperationError> {
    let id = if operation == aes::ENCRYPT {
        aes::ENCRYPT
    } else {
        aes::DECRYPT
    };
    let document = Zeroizing::new(parameters.to_string());
    let key = aes::parse_key(&document).map_err(|e| e.operation(id))?;
    let input = super::bytes(&inputs["input"])?;
    let output = if id == aes::ENCRYPT {
        aes::encrypt(&input, key.as_ref())?
    } else {
        aes::decrypt(&input, key.as_ref())?
    };
    Ok(serde_json::json!({"output":Bytes { data: output }}))
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    #[test]
    fn aes_blocks_use_typed_core_ports_and_private_policy() {
        let op = "cipher.aes.encrypt_blocks";
        assert!(super::super::supported(op));
        let p = json!({"key_hex":"000102030405060708090a0b0c0d0e0f"});
        let result = super::super::execute(
            op,
            json!({"input":{"base64":"ABEiM0RVZneImaq7zN3u/w=="}}),
            p,
        )
        .unwrap();
        assert_eq!(
            result,
            json!({"output":{"base64":"acTg2Gp7BDDYzbeAcLTFWg=="}})
        );
        let contract = cube_core::operations::require(op)
            .unwrap()
            .contract()
            .unwrap();
        contract.validate_outputs(&result).unwrap();
        assert!(contract.policy.private_input && contract.policy.private_output);
        assert!(!contract.policy.automatic_capture);
        assert_eq!(contract.policy.sensitive_parameters, ["key_hex"]);
        assert!(contract.pipeline_adapter.is_none());
    }
}
