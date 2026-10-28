//! Bounded typed bridge to Core; settings and state remain private by policy.
use cube_core::{
    ciphers::enigma,
    errors::{ErrorCode, OperationError},
};
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    text: String,
}

pub(super) fn execute(inputs: Value, parameters: Value) -> Result<Value, OperationError> {
    let input: Input = serde_json::from_value(inputs).map_err(|_| {
        OperationError::new(ErrorCode::InvalidInput, "use the exact named text input")
            .operation(enigma::ID)
    })?;
    let p = enigma::parse_parameters(&parameters.to_string())?;
    serde_json::to_value(enigma::transform(&input.text, &p)?).map_err(|_| {
        OperationError::new(
            ErrorCode::InvalidInput,
            "typed Enigma report serialization failed",
        )
        .operation(enigma::ID)
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    #[test]
    fn enigma_named_ports_call_core_and_preserve_private_policy() {
        let op = "cipher.enigma";
        assert!(super::super::supported(op));
        let p = json!({"profile":"enigma_i","rotors":["I","II","III"],"reflector":"B","rings":[1,1,1],"windows":"AAA","plugboard":[]});
        let r = super::super::execute(op, json!({"text":"AAAAA"}), p.clone()).unwrap();
        assert_eq!(r, json!({"text":"BDZGO","final_windows":"AAF","trace":[]}));
        let c = cube_core::operations::require(op)
            .unwrap()
            .contract()
            .unwrap();
        c.validate_outputs(&r).unwrap();
        assert!(c.policy.private_input && c.policy.private_output);
        assert!(c.policy.sensitive_parameters.contains(&"plugboard".into()));
        let mut bad = p.clone();
        bad["windows"] = json!("bad");
        assert!(super::super::execute(op, json!({"text":"AAAAA"}), bad).is_err());
        assert!(super::super::execute(op, json!({"text":"AAAAA", "extra":true}), p).is_err());
    }
}
