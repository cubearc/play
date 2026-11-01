//! Composition preserves exact token arrays and ambiguity-bearing outputs.
use play_engine::{
    graph::Definition,
    worker::{self, PipelineInvocation},
};
use serde_json::json;
use std::collections::BTreeMap;
use uuid::Uuid;
#[test]
fn triliteral_token_ports_round_trip_unicode_through_a_native_graph() {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let p = json!({"profile":"ordered_ternary_v1","alphabet":"é🦀\u{0000}","trits":"0.-"});
    let definition=Definition::parse(json!({"formatVersion":4,"inputs":[{"name":"source","semanticType":"text"}],"nodes":[
      {"id":a,"operation":"cipher.triliteral.encode","operationVersion":1,"parameters":p,"position":{"x":0,"y":0},"inputs":{"text":{"kind":"literal","value":"🦀\u{0000}é"}}},
      {"id":b,"operation":"cipher.triliteral.decode","operationVersion":1,"parameters":p,"position":{"x":300,"y":0},"inputs":{"tokens":{"kind":"output","nodeId":a,"port":"tokens"}}}
    ],"output":b})).unwrap();
    let invocation = PipelineInvocation {
        definition,
        inputs: BTreeMap::new(),
    };
    worker::validate_pipeline(&invocation).unwrap();
    let output = worker::execute_pipeline(invocation, |_| Ok(())).unwrap();
    assert_eq!(
        worker::named_outputs("cipher.triliteral.decode", &output).unwrap()["text"],
        "🦀\u{0000}é"
    );
}
#[test]
fn nullable_key_phrase_readings_cannot_silently_feed_a_text_port() {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let p = json!({"phrase":"GIVEMELIBERTYORGIVEMEDEATH"});
    let definition = Definition::parse(
        json!({"formatVersion":4,"inputs":[{"name":"source","semanticType":"text"}],"nodes":[
      {"id":a,"operation":"cipher.key_phrase.decode","operationVersion":1,"parameters":p,"position":{"x":0,"y":0},"inputs":{"text":{"kind":"literal","value":"I"}}},
      {"id":b,"operation":"cipher.key_phrase.encode","operationVersion":1,"parameters":p,"position":{"x":300,"y":0},"inputs":{"text":{"kind":"output","nodeId":a,"port":"text"}}}
    ],"output":b}),
    );
    assert_eq!(definition.err(), Some("contract_mismatch"));
}

#[test]
fn dated_alphabet_profiles_preserve_digraphs_and_every_source_assignment() {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../../cube/crates/cube-core/src/symbols/alphabet/vectors.json"
    ))
    .unwrap();
    for row in rows {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let p = json!({"profile":row["profile"]});
        let definition = Definition::parse(json!({"formatVersion":4,"inputs":[{"name":"source","semanticType":"text"}],"nodes":[
            {"id":a,"operation":"symbol.alphabet.encode","operationVersion":1,"parameters":p,"position":{"x":0,"y":0},"inputs":{"units":{"kind":"literal","value":row["units"]}}},
            {"id":b,"operation":"symbol.alphabet.decode","operationVersion":1,"parameters":p,"position":{"x":300,"y":0},"inputs":{"tokens":{"kind":"output","nodeId":a,"port":"tokens"}}}
        ],"output":b})).unwrap();
        let invocation = PipelineInvocation {
            definition,
            inputs: BTreeMap::new(),
        };
        worker::validate_pipeline(&invocation).unwrap();
        let output = worker::execute_pipeline(invocation, |_| Ok(())).unwrap();
        let actual = worker::named_outputs("symbol.alphabet.decode", &output).unwrap();
        assert_eq!(actual["tokens"], row["tokens"]);
        for (index, unit) in row["units"].as_array().unwrap().iter().enumerate() {
            assert_eq!(
                actual["readings"][index],
                json!({"token_index":index,"alternatives":row.get("alternatives").and_then(|a|a.get(index)).cloned().unwrap_or_else(||json!([unit]))})
            );
        }
        assert_eq!(actual["provenance"]["profile_id"], row["profile"]);
    }
}

#[test]
fn bounded_classical_reports_remain_typed_private_native_graph_outputs() {
    let rows: Vec<serde_json::Value> = serde_json::from_str(include_str!(
        "../../../../cube/crates/cube-core/src/operations/contracts/conformance.json"
    ))
    .unwrap();
    for operation in ["analysis.substitution.solve", "analysis.family.suggest"] {
        let row = rows.iter().find(|r| r["operation"] == operation).unwrap();
        let id = Uuid::new_v4();
        let definition=Definition::parse(json!({"formatVersion":4,"inputs":[{"name":"source","semanticType":"text"}],"nodes":[
            {"id":id,"operation":operation,"operationVersion":1,"parameters":{},"retainPrivate":true,"position":{"x":0,"y":0},"inputs":{"request":{"kind":"literal","value":row["inputs"]["request"]}}}
        ],"output":id})).unwrap();
        let invocation = PipelineInvocation {
            definition,
            inputs: BTreeMap::new(),
        };
        worker::validate_pipeline(&invocation).unwrap();
        let output = worker::execute_pipeline(invocation, |_| Ok(())).unwrap();
        assert_eq!(
            worker::named_outputs(operation, &output).unwrap(),
            row["outputs"]
        );
    }
}
