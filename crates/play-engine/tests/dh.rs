//! Named finite-field agreement through real typed private graphs.
use play_engine::{
    graph::Definition,
    worker::{self, InputValue, PipelineInvocation},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use uuid::Uuid;
fn invoke(v: &Value, retain: bool) -> Result<Value, &'static str> {
    let node = Uuid::new_v4();
    let definition = Definition::parse(
        json!({"formatVersion":4,"inputs":[{"name":"source","semanticType":"bytes"},{"name":"secret","semanticType":"bytes"}],"nodes":[{"id":node,"operation":"crypto.dh.agree","operationVersion":1,"parameters":v["parameters"],"retainPrivate":retain,"position":{"x":0,"y":0},"inputs":{"input":{"kind":"input","name":"source"},"key":{"kind":"input","name":"secret"}}}],"output":node}),
    )?;
    let byte = |name: &str| InputValue::Bytes {
        base64: cube_core::encoding::base64_encode(
            &cube_core::encoding::hex_decode(v[name].as_str().unwrap()).unwrap(),
        ),
    };
    let invocation = PipelineInvocation {
        definition,
        inputs: BTreeMap::from([
            ("source".into(), byte("peer_hex")),
            ("secret".into(), byte("private_key_hex")),
        ]),
    };
    worker::validate_pipeline(&invocation)?;
    worker::named_outputs(
        "crypto.dh.agree",
        &worker::execute_pipeline(invocation, |_| Ok(()))?,
    )
}
#[test]
fn all_named_dh_groups_require_retention_consent_and_produce_exact_bytes() {
    let vectors: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../cube/crates/cube-core/src/asymmetric/dh/vectors.json"
    ))
    .unwrap();
    for v in vectors {
        assert_eq!(invoke(&v, false), Err("retention_policy_unavailable"));
        let expected = cube_core::encoding::hex_decode(v["shared_hex"].as_str().unwrap()).unwrap();
        assert_eq!(
            invoke(&v, true).unwrap(),
            json!({"output":{"base64":cube_core::encoding::base64_encode(&expected)}})
        );
    }
}
