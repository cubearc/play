//! Educational token algorithms execute through the private named-port adapter.
use play_engine::{
    graph::Definition,
    worker::{self, InputValue, PipelineInvocation},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use uuid::Uuid;
fn invoke(v: &Value, operation: &str, retain: bool) -> Result<Value, &'static str> {
    let node = Uuid::new_v4();
    let definition = Definition::parse(
        json!({"formatVersion":4,"inputs":[{"name":"source","semanticType":"json"},{"name":"secret","semanticType":"bytes"}],"nodes":[{"id":node,"operation":operation,"operationVersion":1,"parameters":v["parameters"],"retainPrivate":retain,"position":{"x":0,"y":0},"inputs":{"tokens":{"kind":"input","name":"source"},"key":{"kind":"input","name":"secret"}}}],"output":node}),
    )?;
    let invocation = PipelineInvocation {
        definition,
        inputs: BTreeMap::from([
            (
                "source".into(),
                InputValue::Structured {
                    value: v["tokens"].clone(),
                },
            ),
            (
                "secret".into(),
                InputValue::Bytes {
                    base64: cube_core::encoding::base64_encode(
                        &serde_json::to_vec(&v["key"]).unwrap(),
                    ),
                },
            ),
        ]),
    };
    worker::validate_pipeline(&invocation)?;
    worker::named_outputs(
        operation,
        &worker::execute_pipeline(invocation, |_| Ok(()))?,
    )
}
#[test]
fn independent_reference_arrays_and_complete_root_sets_require_private_retention() {
    let vectors: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../cube/crates/cube-core/src/asymmetric/historical_public_key/vectors.json"
    ))
    .unwrap();
    for v in vectors {
        let family = v["scheme"].as_str().unwrap();
        let enc = format!("cipher.{family}.encrypt");
        let dec = format!("cipher.{family}.decrypt");
        let mut case = json!({"parameters":v["parameters"],"tokens":v["ciphertext_tokens"],"key":v["private_key"]});
        assert_eq!(
            invoke(&case, &dec, false),
            Err("retention_policy_unavailable")
        );
        let expected = if family == "rabin" {
            json!({"candidates":v["candidate_tokens"]})
        } else {
            json!({"tokens":v["tokens"]})
        };
        assert_eq!(invoke(&case, &dec, true).unwrap(), expected);
        case["tokens"] = v["tokens"].clone();
        case["key"] = v["public_key"].clone();
        assert_eq!(
            invoke(&case, &enc, false),
            Err("retention_policy_unavailable")
        );
        let cipher = invoke(&case, &enc, true).unwrap();
        case["tokens"] = cipher["tokens"].clone();
        case["key"] = v["private_key"].clone();
        assert_eq!(invoke(&case, &dec, true).unwrap(), expected);
    }
}
