//! Private key byte ports preserve native graph limits and retention authority.
use play_engine::{
    graph::Definition,
    worker::{self, InputValue, PipelineInvocation},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use uuid::Uuid;
fn invoke(
    operation: &str,
    input: &[u8],
    key: &[u8],
    parameters: Value,
    retain: bool,
) -> Result<Value, &'static str> {
    let node = Uuid::new_v4();
    let definition = Definition::parse(
        json!({"formatVersion":4,"inputs":[{"name":"source","semanticType":"bytes"},{"name":"key_material","semanticType":"bytes"}],"nodes":[{"id":node,"operation":operation,"operationVersion":1,"parameters":parameters,"retainPrivate":retain,"position":{"x":0,"y":0},"inputs":{"input":{"kind":"input","name":"source"},"key":{"kind":"input","name":"key_material"}}}],"output":node}),
    )?;
    let invocation = PipelineInvocation {
        definition,
        inputs: BTreeMap::from([
            (
                "source".into(),
                InputValue::Bytes {
                    base64: cube_core::encoding::base64_encode(input),
                },
            ),
            (
                "key_material".into(),
                InputValue::Bytes {
                    base64: cube_core::encoding::base64_encode(key),
                },
            ),
        ]),
    };
    worker::validate_pipeline(&invocation)?;
    let output = worker::execute_pipeline(invocation, |_| Ok(()))?;
    worker::named_outputs(operation, &output)
}
fn bytes(v: &Value, name: &str) -> Vec<u8> {
    cube_core::encoding::hex_decode(v[name].as_str().unwrap()).unwrap()
}
fn decode(v: &Value) -> Vec<u8> {
    cube_core::encoding::base64_decode(v["output"]["base64"].as_str().unwrap()).unwrap()
}
#[test]
fn all_rsa_key_widths_and_independent_legacy_results_cross_private_native_graphs() {
    let vectors: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../cube/crates/cube-core/src/asymmetric/rsa/interoperability.json"
    ))
    .unwrap();
    for v in vectors {
        let plain = bytes(&v, "plaintext_hex");
        let ct = bytes(&v, "ciphertext_hex");
        let private = bytes(&v, "private_key_hex");
        let public = bytes(&v, "public_key_hex");
        assert_eq!(
            decode(
                &invoke(
                    "cipher.rsa.oaep.decrypt",
                    &ct,
                    &private,
                    v["private_parameters"].clone(),
                    true
                )
                .unwrap()
            ),
            plain
        );
        let a = invoke(
            "cipher.rsa.oaep.encrypt",
            &plain,
            &public,
            v["public_parameters"].clone(),
            true,
        )
        .unwrap();
        let b = invoke(
            "cipher.rsa.oaep.encrypt",
            &plain,
            &public,
            v["public_parameters"].clone(),
            true,
        )
        .unwrap();
        assert_ne!(a, b);
        assert_eq!(
            decode(
                &invoke(
                    "cipher.rsa.oaep.decrypt",
                    &decode(&a),
                    &private,
                    v["private_parameters"].clone(),
                    true
                )
                .unwrap()
            ),
            plain
        );
        assert_eq!(
            invoke(
                "cipher.rsa.oaep.decrypt",
                &ct,
                &private,
                v["private_parameters"].clone(),
                false
            ),
            Err("retention_policy_unavailable")
        );
    }
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../../cube/crates/cube-core/src/asymmetric/rsa/implicit_rejection_vectors.json"
    ))
    .unwrap();
    for group in vectors["groups"].as_array().unwrap() {
        let key = bytes(group, "key_hex");
        for v in group["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["expect"] != "public_error")
        {
            let output = invoke(
                "cipher.rsa.pkcs1_v1_5.decrypt",
                &bytes(v, "ciphertext_hex"),
                &key,
                group["parameters"].clone(),
                true,
            )
            .unwrap();
            assert_eq!(decode(&output), bytes(v, "output_hex"));
        }
    }
}
