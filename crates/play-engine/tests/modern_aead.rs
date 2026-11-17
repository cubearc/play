//! Independent authenticated profiles through consent-gated named native ports.
use play_engine::{
    graph::Definition,
    worker::{self, InputValue, PipelineInvocation},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use uuid::Uuid;
fn bytes(v: &Value, name: &str) -> Vec<u8> {
    cube_core::encoding::hex_decode(v[name].as_str().unwrap()).unwrap()
}
fn transport(data: &[u8]) -> Value {
    json!({"base64":cube_core::encoding::base64_encode(data)})
}
fn invoke(graph: Value, plain: &[u8]) -> Result<Value, &'static str> {
    let invocation = PipelineInvocation {
        definition: Definition::parse(graph)?,
        inputs: BTreeMap::from([(
            "source".into(),
            InputValue::Bytes {
                base64: cube_core::encoding::base64_encode(plain),
            },
        )]),
    };
    worker::validate_pipeline(&invocation)?;
    worker::execute_pipeline(invocation, |_| Ok(()))
}
#[test]
fn exact_aead_profiles_and_siv_named_tags_chain_without_failed_plaintext() {
    for siv in [false, true] {
        let vectors: Vec<Value> = serde_json::from_str(if siv {
            include_str!(
                "../../../../cube/crates/cube-core/src/ciphers/modern_aead/siv_vectors.json"
            )
        } else {
            include_str!("../../../../cube/crates/cube-core/src/ciphers/modern_aead/vectors.json")
        })
        .unwrap();
        for v in vectors {
            let plain = bytes(&v, "plaintext_hex");
            let ciphertext = bytes(&v, "ciphertext_hex");
            let tag = bytes(&v, "tag_hex");
            let parameters = &v["parameters"];
            let (encrypt_op, decrypt_op, aad_name) = if siv {
                (
                    "cipher.aes.siv.encrypt",
                    "cipher.aes.siv.decrypt",
                    "aad_components",
                )
            } else {
                ("cipher.aead.encrypt", "cipher.aead.decrypt", "aad")
            };
            let aad = if siv {
                json!(
                    v["aad_components_hex"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| cube_core::encoding::hex_decode(v.as_str().unwrap()).unwrap())
                        .collect::<Vec<_>>()
                )
            } else {
                transport(&bytes(&v, "aad_hex"))
            };
            let encrypt = Uuid::new_v4();
            let decrypt = Uuid::new_v4();
            let mut encrypt_inputs = json!({"input":{"kind":"input","name":"source"}});
            encrypt_inputs[aad_name] = json!({"kind":"literal","value":aad});
            let mut decrypt_inputs = json!({"input":{"kind":"output","nodeId":encrypt,"port":"output"},"tag":{"kind":"output","nodeId":encrypt,"port":"tag"}});
            decrypt_inputs[aad_name] = json!({"kind":"literal","value":aad});
            let graph = json!({"formatVersion":4,"inputs":[{"name":"source","semanticType":"bytes"}],"nodes":[
                {"id":encrypt,"operation":encrypt_op,"operationVersion":1,"parameters":parameters,"retainPrivate":true,"position":{"x":0,"y":0},"inputs":encrypt_inputs},
                {"id":decrypt,"operation":decrypt_op,"operationVersion":1,"parameters":parameters,"retainPrivate":true,"position":{"x":1,"y":0},"inputs":decrypt_inputs}
            ],"output":decrypt});
            let output = invoke(graph.clone(), &plain).unwrap();
            assert_eq!(
                worker::named_outputs(decrypt_op, &output).unwrap(),
                json!({"output":transport(&plain)})
            );
            let mut only_encrypt = graph.clone();
            only_encrypt["nodes"].as_array_mut().unwrap().pop();
            only_encrypt["output"] = json!(encrypt);
            let output = invoke(only_encrypt, &plain).unwrap();
            assert_eq!(
                worker::named_outputs(encrypt_op, &output).unwrap(),
                json!({"output":transport(&ciphertext),"tag":transport(&tag)})
            );
            let mut bad = tag;
            bad[0] ^= 1;
            let mut forged = graph.clone();
            forged["nodes"][1]["inputs"]["tag"] = json!({"kind":"literal","value":transport(&bad)});
            assert_eq!(invoke(forged, &plain), Err("invalid_input"));
            let mut denied = graph;
            denied["nodes"][1]["retainPrivate"] = json!(false);
            assert_eq!(
                Definition::parse(denied).map(|_| ()),
                Err("retention_policy_unavailable")
            );
        }
    }
}
