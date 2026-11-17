//! Exact SM2 profiles pass through private native graphs.
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
        json!({"formatVersion":4,"inputs":[{"name":"source","semanticType":"bytes"},{"name":"secret","semanticType":"bytes"}],"nodes":[{"id":node,"operation":operation,"operationVersion":1,"parameters":v["parameters"],"retainPrivate":retain,"position":{"x":0,"y":0},"inputs":{"input":{"kind":"input","name":"source"},"key":{"kind":"input","name":"secret"}}}],"output":node}),
    )?;
    let byte = |name: &str| InputValue::Bytes {
        base64: cube_core::encoding::base64_encode(
            &cube_core::encoding::hex_decode(v[name].as_str().unwrap()).unwrap(),
        ),
    };
    let invocation = PipelineInvocation {
        definition,
        inputs: BTreeMap::from([
            ("source".into(), byte("input_hex")),
            ("secret".into(), byte("key_hex")),
        ]),
    };
    worker::validate_pipeline(&invocation)?;
    worker::named_outputs(
        operation,
        &worker::execute_pipeline(invocation, |_| Ok(()))?,
    )
}
#[test]
fn external_sm2_answers_and_fresh_ciphertexts_require_explicit_retention() {
    let vectors: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../cube/crates/cube-core/src/asymmetric/sm2/interoperability.json"
    ))
    .unwrap();
    for v in vectors {
        for (profile, field) in [
            ("sm2_sm3_c1c3c2", "c1c3c2_hex"),
            ("sm2_sm3_c1c2c3", "c1c2c3_hex"),
            ("sm2_sm3_der", "der_hex"),
        ] {
            let mut case = json!({"parameters":{"profile":profile},"input_hex":v[field],"key_hex":v["private_key_hex"]});
            assert_eq!(
                invoke(&case, "cipher.sm2.decrypt", false),
                Err("retention_policy_unavailable")
            );
            let expected =
                cube_core::encoding::hex_decode(v["plaintext_hex"].as_str().unwrap()).unwrap();
            assert_eq!(
                invoke(&case, "cipher.sm2.decrypt", true).unwrap(),
                json!({"output":{"base64":cube_core::encoding::base64_encode(&expected)}})
            );
            case["input_hex"] = v["plaintext_hex"].clone();
            case["key_hex"] = v["public_key_hex"].clone();
            let encrypted = invoke(&case, "cipher.sm2.encrypt", true).unwrap();
            case["input_hex"] = json!(cube_core::encoding::hex_encode(
                &cube_core::encoding::base64_decode(
                    encrypted["output"]["base64"].as_str().unwrap()
                )
                .unwrap()
            ));
            case["key_hex"] = v["private_key_hex"].clone();
            assert_eq!(
                invoke(&case, "cipher.sm2.decrypt", true).unwrap(),
                json!({"output":{"base64":cube_core::encoding::base64_encode(&expected)}})
            );
        }
    }
}
