//! Core-only algorithms through private typed native execution and byte viewing.
use play_engine::{
    graph::Definition,
    worker::{self, InputValue, PipelineInvocation},
};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
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
    let profiles: BTreeSet<_> = vectors
        .iter()
        .map(|v| v["profile"].as_str().unwrap())
        .collect();
    assert_eq!(
        profiles,
        cube_core::ciphers::block_profiles::Profile::NAMES
            .iter()
            .copied()
            .collect()
    );
    let workers = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(4);
    std::thread::scope(|scope| {
        for chunk in vectors.chunks(vectors.len().div_ceil(workers)) {
            scope.spawn(move || {
                for vector in chunk {
                    assert_block_reference(vector);
                }
            });
        }
    });
}
fn assert_block_reference(v: &Value) {
    let mut p = json!({"profile":v["profile"],"key_hex":v["key_hex"]});
    if let Some(bits) = v.get("effective_key_bits") {
        p["effective_key_bits"] = bits.clone();
    }
    if let Some(tweak) = v.get("tweak_hex") {
        p["tweak_hex"] = tweak.clone();
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
        assert_eq!(
            run(operation, input.clone(), p.clone(), false),
            Err("retention_policy_unavailable")
        );
        let result = run(operation, input, p.clone(), true)
            .unwrap_or_else(|e| panic!("{} {operation}: {e}", v["profile"]));
        assert_eq!(
            result,
            json!({"output":{"base64":cube_core::encoding::base64_encode(&hex_bytes(expected.as_str().unwrap()))}})
        );
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

#[test]
fn xxtea_whole_word_answers_cross_private_graph_and_byte_viewer() {
    let rows: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../cube/crates/cube-core/src/ciphers/xxtea/vectors.json"
    ))
    .unwrap();
    for v in rows {
        let p = json!({"key_hex":v["key_hex"]});
        for (op, input, expected) in [
            (
                "cipher.xxtea.encrypt_words",
                &v["plaintext_hex"],
                &v["ciphertext_hex"],
            ),
            (
                "cipher.xxtea.decrypt_words",
                &v["ciphertext_hex"],
                &v["plaintext_hex"],
            ),
        ] {
            let input = json!({"base64":cube_core::encoding::base64_encode(&hex_bytes(input.as_str().unwrap()))});
            assert_eq!(
                run(op, input.clone(), p.clone(), false),
                Err("retention_policy_unavailable")
            );
            let output = run(op, input, p.clone(), true).unwrap();
            assert_eq!(
                output,
                json!({"output":{"base64":cube_core::encoding::base64_encode(&hex_bytes(expected.as_str().unwrap()))}})
            );
        }
    }
}
