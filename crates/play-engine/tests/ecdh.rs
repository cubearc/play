//! Exact shared secrets pass through private native graphs and byte output handling.
use play_engine::{
    graph::Definition,
    worker::{self, InputValue, PipelineInvocation},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use uuid::Uuid;
fn invoke(input: &[u8], parameters: Value, retain: bool) -> Result<Value, &'static str> {
    let node = Uuid::new_v4();
    let definition = Definition::parse(
        json!({"formatVersion":4,"inputs":[{"name":"source","semanticType":"bytes"}],"nodes":[{"id":node,"operation":"crypto.ecdh.agree","operationVersion":1,"parameters":parameters,"retainPrivate":retain,"position":{"x":0,"y":0},"inputs":{"input":{"kind":"input","name":"source"}}}],"output":node}),
    )?;
    let invocation = PipelineInvocation {
        definition,
        inputs: BTreeMap::from([(
            "source".into(),
            InputValue::Bytes {
                base64: cube_core::encoding::base64_encode(input),
            },
        )]),
    };
    worker::validate_pipeline(&invocation)?;
    let result = worker::execute_pipeline(invocation, |_| Ok(()))?;
    worker::named_outputs("crypto.ecdh.agree", &result)
}
#[test]
fn rfc_shared_secrets_are_exact_and_explicit_private_retention_is_required() {
    let vectors: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../cube/crates/cube-core/src/asymmetric/ecdh/vectors.json"
    ))
    .unwrap();
    for v in vectors {
        let peer = cube_core::encoding::hex_decode(v["peer_hex"].as_str().unwrap()).unwrap();
        let expected = cube_core::encoding::hex_decode(v["shared_hex"].as_str().unwrap()).unwrap();
        assert_eq!(
            invoke(&peer, v["parameters"].clone(), true).unwrap(),
            json!({"output":{"base64":cube_core::encoding::base64_encode(&expected)}})
        );
        assert_eq!(
            invoke(&peer, v["parameters"].clone(), false),
            Err("retention_policy_unavailable")
        );
        assert_eq!(
            invoke(&[0; 32], v["parameters"].clone(), true),
            Err("invalid_input")
        );
    }
}
