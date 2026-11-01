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
