//! Native named-port graphs exercise the v0.33 cryptography profiles.
use play_engine::{
    graph::Definition,
    worker::{self, InputValue, PipelineInvocation},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use uuid::Uuid;

fn invoke(
    operation: &str,
    parameters: &Value,
    named: &Value,
    retain: bool,
) -> Result<Value, &'static str> {
    let mut port_sources = serde_json::Map::new();
    let mut input_slots = Vec::new();
    let mut inputs = BTreeMap::new();
    for (index, (port, value)) in named.as_object().unwrap().iter().enumerate() {
        let slot = if index == 0 {
            "source".to_owned()
        } else {
            format!("operand{index}")
        };
        input_slots.push(json!({"name":slot,"semanticType":"bytes"}));
        port_sources.insert(port.clone(), json!({"kind":"input","name":slot}));
        inputs.insert(
            slot,
            InputValue::Bytes {
                base64: value["base64"].as_str().unwrap().to_owned(),
            },
        );
    }
    if input_slots.is_empty() {
        input_slots.push(json!({"name":"source","semanticType":"bytes"}));
    }
    let node = Uuid::new_v4();
    let definition = Definition::parse(json!({
        "formatVersion":4,
        "inputs":input_slots,
        "nodes":[{"id":node,"operation":operation,"operationVersion":1,"parameters":parameters,"retainPrivate":retain,"position":{"x":0,"y":0},"inputs":port_sources}],
        "output":node,
    }))?;
    let invocation = PipelineInvocation { definition, inputs };
    worker::validate_pipeline(&invocation)?;
    let frame = |value| {
        let mut frame = Vec::new();
        play_engine::protocol::write_frame(&mut frame, &value)?;
        let text = std::str::from_utf8(&frame).map_err(|_| "invalid_frame")?;
        cube_core::pipeline::json::parse(text).map_err(|_| "invalid_frame")?;
        Ok(())
    };
    let output = worker::execute_pipeline(invocation, frame)?;
    frame(json!({"status":"completed","output":output}))?;
    worker::named_outputs(operation, &output)
}

#[test]
fn consented_native_graphs_execute_key_generation_and_both_kem_peers() {
    let invoke = |verb, inputs, retain| {
        invoke(
            &format!("crypto.ml_kem.{verb}"),
            &json!({"profile":"ml_kem_768_fips203"}),
            &inputs,
            retain,
        )
    };
    assert_eq!(
        invoke("keygen", json!({}), false),
        Err("retention_policy_unavailable")
    );
    let keys = invoke("keygen", json!({}), true).unwrap();
    let envelope = invoke(
        "encapsulate",
        json!({"public_key":keys["public_key"]}),
        true,
    )
    .unwrap();
    let recovered = invoke(
        "decapsulate",
        json!({"ciphertext":envelope["ciphertext"],"private_key":keys["private_key"]}),
        true,
    )
    .unwrap();
    assert!(recovered == json!({"shared_secret":envelope["shared_secret"]}));
}

#[test]
fn deterministic_advanced_ciphertexts_and_shared_secrets_match_the_cube_contracts() {
    let fixtures: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../cube/crates/cube-core/src/operations/contracts/conformance.json"
    ))
    .unwrap();
    for operation in [
        "crypto.kyber.decapsulate",
        "crypto.frodo_kem.decapsulate",
        "crypto.hqc.decapsulate",
        "crypto.classic_mceliece.decapsulate",
        "crypto.bike.decapsulate",
        "crypto.ntru_kem.decapsulate",
        "cipher.bfv.add",
        "cipher.bgv.add",
        "cipher.ckks.add",
        "cipher.tfhe.not",
        "cipher.paillier.add",
        "cipher.damgard_jurik.add",
        "cipher.ntru_encrypt.decrypt",
        "cipher.regev.decrypt",
    ] {
        let fixture = fixtures
            .iter()
            .find(|value| value["operation"] == operation)
            .unwrap();
        let actual = invoke(operation, &fixture["parameters"], &fixture["inputs"], true);
        assert!(actual.is_ok(), "{operation}: {:?}", actual.err());
        assert!(actual.unwrap() == fixture["outputs"], "{operation}");
    }

    for family in ["ntru_encrypt", "regev"] {
        let encrypt = format!("cipher.{family}.encrypt");
        let decrypt = format!("cipher.{family}.decrypt");
        let fixture = fixtures
            .iter()
            .find(|row| row["operation"] == encrypt)
            .unwrap();
        let actual = invoke(&encrypt, &fixture["parameters"], &fixture["inputs"], true).unwrap();
        let verifier = &fixture["verification"];
        let inputs =
            json!({"input":actual["output"],"private_key":verifier["inputs"]["private_key"]});
        let recovered = invoke(&decrypt, &verifier["parameters"], &inputs, true).unwrap();
        assert!(recovered == verifier["expected_outputs"], "{family}");
    }
}
