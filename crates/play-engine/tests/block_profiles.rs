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

#[test]
fn stream_reference_answers_cross_private_graph_and_byte_viewer() {
    let vectors: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../cube/crates/cube-core/src/ciphers/stream/reference/vectors.json"
    ))
    .unwrap();
    let profiles: BTreeSet<_> = vectors
        .iter()
        .map(|v| v["parameters"]["profile"].as_str().unwrap())
        .collect();
    assert_eq!(
        profiles,
        cube_core::ciphers::stream::reference::Profile::NAMES
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
                    assert_stream_reference(vector);
                }
            });
        }
    });
}

fn assert_stream_reference(v: &Value) {
    let parameters = &v["parameters"];
    let input_limit = play_engine::protocol::contract().input_bytes;
    let limit = input_limit - serde_json::to_vec(parameters).unwrap().len();
    for (op, input, expected) in [
        (
            "cipher.stream.encrypt",
            &v["plaintext_hex"],
            &v["ciphertext_hex"],
        ),
        (
            "cipher.stream.decrypt",
            &v["ciphertext_hex"],
            &v["plaintext_hex"],
        ),
    ] {
        let input = hex_bytes(input.as_str().unwrap());
        let expected = hex_bytes(expected.as_str().unwrap());
        let transport = |bytes: &[u8]| json!({"base64":cube_core::encoding::base64_encode(bytes)});
        assert_eq!(
            run(op, transport(&input), parameters.clone(), false),
            Err("retention_policy_unavailable")
        );
        if input.len() > input_limit {
            assert_eq!(
                run(op, transport(&input), parameters.clone(), true),
                Err("input_limit")
            );
        }
        let n = input.len().min(limit);
        let actual = run(op, transport(&input[..n]), parameters.clone(), true)
            .unwrap_or_else(|e| panic!("{} {op} ({n} bytes): {e}", parameters["profile"]));
        assert_eq!(
            actual,
            json!({"output":{"base64":cube_core::encoding::base64_encode(&expected[..n])}})
        );
    }
}

#[test]
fn authenticated_stream_detached_ports_chain_and_reject_forged_tags() {
    let vectors: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../cube/crates/cube-core/src/ciphers/stream/reference/authentication/vectors.json"
    ))
    .unwrap();
    for v in vectors {
        let parameters = &v["parameters"];
        let transport = |name: &str| json!({"base64":cube_core::encoding::base64_encode(&hex_bytes(v[name].as_str().unwrap()))});
        let plain = transport("plaintext_hex");
        let ciphertext = transport("ciphertext_hex");
        let tag = transport("tag_hex");
        assert_eq!(
            run(
                "cipher.stream.auth.encrypt",
                plain.clone(),
                parameters.clone(),
                false
            ),
            Err("retention_policy_unavailable")
        );
        assert_eq!(
            run(
                "cipher.stream.auth.encrypt",
                plain.clone(),
                parameters.clone(),
                true
            )
            .unwrap(),
            json!({"output":ciphertext,"tag":tag})
        );
        let encrypt = Uuid::new_v4();
        let decrypt = Uuid::new_v4();
        let graph = json!({"formatVersion":4,"inputs":[{"name":"source","semanticType":"bytes"}],"nodes":[
            {"id":encrypt,"operation":"cipher.stream.auth.encrypt","operationVersion":1,"parameters":parameters,"retainPrivate":true,"position":{"x":0,"y":0},"inputs":{"input":{"kind":"input","name":"source"}}},
            {"id":decrypt,"operation":"cipher.stream.auth.decrypt","operationVersion":1,"parameters":parameters,"retainPrivate":true,"position":{"x":1,"y":0},"inputs":{"input":{"kind":"output","nodeId":encrypt,"port":"output"},"tag":{"kind":"output","nodeId":encrypt,"port":"tag"}}}
        ],"output":decrypt});
        let invoke = |graph: Value| PipelineInvocation {
            definition: Definition::parse(graph).unwrap(),
            inputs: BTreeMap::from([(
                "source".into(),
                InputValue::Bytes {
                    base64: plain["base64"].as_str().unwrap().into(),
                },
            )]),
        };
        let invocation = invoke(graph.clone());
        worker::validate_pipeline(&invocation).unwrap();
        let result = worker::execute_pipeline(invocation, |_| Ok(())).unwrap();
        assert_eq!(
            worker::named_outputs("cipher.stream.auth.decrypt", &result).unwrap(),
            json!({"output":plain})
        );
        let mut bad_tag = hex_bytes(v["tag_hex"].as_str().unwrap());
        bad_tag[0] ^= 128;
        let mut forged = graph.clone();
        forged["nodes"][1]["inputs"]["tag"] = json!({"kind":"literal","value":{"base64":cube_core::encoding::base64_encode(&bad_tag)}});
        let invocation = invoke(forged);
        worker::validate_pipeline(&invocation).unwrap();
        assert_eq!(
            worker::execute_pipeline(invocation, |_| Ok(())),
            Err("invalid_input")
        );
        let mut denied = graph;
        denied["nodes"][1]["retainPrivate"] = json!(false);
        assert_eq!(
            Definition::parse(denied).map(|_| ()),
            Err("retention_policy_unavailable")
        );
    }
}

#[test]
fn aes_byte_modes_cross_private_native_graph_and_byte_viewer() {
    let vectors: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../cube/crates/cube-core/src/ciphers/aes_modes/vectors.json"
    ))
    .unwrap();
    for v in vectors {
        for (operation, input, expected) in [
            (
                "cipher.aes.mode.encrypt",
                &v["plaintext_hex"],
                &v["ciphertext_hex"],
            ),
            (
                "cipher.aes.mode.decrypt",
                &v["ciphertext_hex"],
                &v["plaintext_hex"],
            ),
        ] {
            let bytes = cube_core::encoding::hex_decode(input.as_str().unwrap()).unwrap();
            let input = json!({"base64":cube_core::encoding::base64_encode(&bytes)});
            assert!(run(operation, input.clone(), v["parameters"].clone(), false).is_err());
            let result = run(operation, input, v["parameters"].clone(), true).unwrap();
            let expected = cube_core::encoding::hex_decode(expected.as_str().unwrap()).unwrap();
            assert_eq!(
                result,
                json!({"output":{"base64":cube_core::encoding::base64_encode(&expected)}})
            );
        }
    }
}

#[test]
fn aes_key_wrap_and_adversarial_integrity_cases_use_private_native_bytes() {
    for invalid in [false, true] {
        let vectors: Vec<Value> = serde_json::from_str(if invalid {
            include_str!(
                "../../../../cube/crates/cube-core/src/ciphers/key_wrap/invalid_vectors.json"
            )
        } else {
            include_str!("../../../../cube/crates/cube-core/src/ciphers/key_wrap/vectors.json")
        })
        .unwrap();
        for v in vectors {
            let p = &v["parameters"];
            let wrapped =
                cube_core::encoding::hex_decode(v["wrapped_hex"].as_str().unwrap()).unwrap();
            let wrapped = json!({"base64":cube_core::encoding::base64_encode(&wrapped)});
            if invalid {
                assert_eq!(
                    run("cipher.aes.key_wrap.unwrap", wrapped, p.clone(), true),
                    Err("invalid_input")
                );
                continue;
            }
            let plain =
                cube_core::encoding::hex_decode(v["plaintext_hex"].as_str().unwrap()).unwrap();
            let plain = json!({"base64":cube_core::encoding::base64_encode(&plain)});
            assert_eq!(
                run("cipher.aes.key_wrap.wrap", plain.clone(), p.clone(), false),
                Err("retention_policy_unavailable")
            );
            assert_eq!(
                run("cipher.aes.key_wrap.wrap", plain.clone(), p.clone(), true).unwrap(),
                json!({"output":wrapped})
            );
            assert_eq!(
                run("cipher.aes.key_wrap.unwrap", wrapped, p.clone(), true).unwrap(),
                json!({"output":plain})
            );
        }
    }
}
#[test]
fn explicit_openssl_files_use_private_native_bytes_without_authentication_claims() {
    let cases: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../cube/crates/cube-core/src/ciphers/openssl/vectors.json"
    ))
    .unwrap();
    for v in cases {
        let data = cube_core::encoding::hex_decode(v["ciphertext_hex"].as_str().unwrap()).unwrap();
        let plain = cube_core::encoding::hex_decode(v["plaintext_hex"].as_str().unwrap()).unwrap();
        let input = json!({"base64":cube_core::encoding::base64_encode(&data)});
        assert_eq!(
            run(
                "cipher.openssl.decrypt",
                input.clone(),
                v["parameters"].clone(),
                false
            ),
            Err("retention_policy_unavailable")
        );
        assert_eq!(
            run(
                "cipher.openssl.decrypt",
                input,
                v["parameters"].clone(),
                true
            )
            .unwrap(),
            json!({"output":{"base64":cube_core::encoding::base64_encode(&plain)}})
        );
        let truncated = json!({"base64":cube_core::encoding::base64_encode(&data[..data.len()-1])});
        assert_eq!(
            run(
                "cipher.openssl.decrypt",
                truncated,
                v["parameters"].clone(),
                true
            ),
            Err("invalid_input")
        );
    }
}

#[test]
fn age_authenticates_all_chunks_before_private_native_output() {
    let cases: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../cube/crates/cube-core/src/ciphers/age/vectors.json"
    ))
    .unwrap();
    for v in cases {
        let input = json!({"base64":v["ciphertext_base64"]});
        assert_eq!(
            run(
                "cipher.age.decrypt",
                input.clone(),
                v["parameters"].clone(),
                false
            ),
            Err("retention_policy_unavailable")
        );
        let data =
            cube_core::encoding::base64_decode(v["ciphertext_base64"].as_str().unwrap()).unwrap();
        if data.len() > play_engine::protocol::contract().input_bytes {
            assert_eq!(
                run("cipher.age.decrypt", input, v["parameters"].clone(), true),
                Err("input_limit")
            );
            continue;
        }
        let result = run("cipher.age.decrypt", input, v["parameters"].clone(), true);
        if v["expect"] == "success" {
            assert_eq!(
                result.unwrap(),
                json!({"output":{"base64":v["plaintext_base64"]}})
            );
        } else {
            assert!(result.is_err(), "{}", v["case"]);
        }
    }
}
