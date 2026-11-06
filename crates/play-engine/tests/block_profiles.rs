//! Core-only algorithms through private typed native execution and byte viewing.
use play_engine::{
    graph::Definition,
    worker::{self, InputValue, PipelineInvocation},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use uuid::Uuid;
fn run(
    operation: &str,
    input: Value,
    parameters: Value,
    retain: bool,
) -> Result<Value, &'static str> {
    let node = Uuid::new_v4();
    let definition = Definition::parse(
        json!({"formatVersion":4,"inputs":[{"name":"source","semanticType":"bytes"}],"nodes":[{"id":node,"operation":operation,"operationVersion":1,"parameters":parameters,"retainPrivate":retain,"position":{"x":0,"y":0},"inputs":{"input":{"kind":"input","name":"source"}}}],"output":node}),
    )?;
    let invocation = PipelineInvocation {
        definition,
        inputs: BTreeMap::from([(
            "source".into(),
            InputValue::Bytes {
                base64: input["base64"].as_str().unwrap().into(),
            },
        )]),
    };
    worker::validate_pipeline(&invocation)?;
    let output = worker::execute_pipeline(invocation, |_| Ok(()))?;
    worker::named_outputs(operation, &output)
}
#[test]
fn all_block_reference_answers_cross_the_native_graph_and_byte_viewer() {
    let vectors: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../cube/crates/cube-core/src/ciphers/block_profiles/vectors.json"
    ))
    .unwrap();
    for v in vectors {
        let mut p = json!({"profile":v["profile"],"key_hex":v["key_hex"]});
        if let Some(bits) = v.get("effective_key_bits") {
            p["effective_key_bits"] = bits.clone();
        }
        for (operation, input, expected) in [
            (
                "cipher.block.encrypt_blocks",
                &v["plaintext_hex"],
                &v["ciphertext_hex"],
            ),
            (
                "cipher.block.decrypt_blocks",
                &v["ciphertext_hex"],
                &v["plaintext_hex"],
            ),
        ] {
            let input = json!({"base64":cube_core::encoding::base64_encode(&hex_bytes(input.as_str().unwrap()))});
            assert!(run(operation, input.clone(), p.clone(), false).is_err());
            let result = run(operation, input, p.clone(), true)
                .unwrap_or_else(|e| panic!("{} {operation}: {e}", v["profile"]));
            assert_eq!(
                result,
                json!({"output":{"base64":cube_core::encoding::base64_encode(&hex_bytes(expected.as_str().unwrap()))}})
            );
        }
    }
}
fn hex_bytes(value: &str) -> Vec<u8> {
    value
        .as_bytes()
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}
