use play_engine::{
    graph::Definition,
    worker::{self, PipelineInvocation},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use uuid::Uuid;

fn run(
    operation: &str,
    text: &str,
    parameters: Value,
    retain_private: bool,
) -> Result<Value, &'static str> {
    let node = Uuid::new_v4();
    let definition = Definition::parse(
        json!({"formatVersion":4,"inputs":[{"name":"source","semanticType":"text"}],"nodes":[{"id":node,"operation":operation,"operationVersion":1,"parameters":parameters,"retainPrivate":retain_private,"inputs":{"text":{"kind":"literal","value":text}},"position":{"x":40,"y":40}}],"output":node}),
    )?;
    let invocation = PipelineInvocation {
        definition,
        inputs: BTreeMap::new(),
    };
    worker::validate_pipeline(&invocation)?;
    let output = worker::execute_pipeline(invocation, |_| Ok(()))?;
    worker::named_outputs(operation, &output)
}

#[test]
fn every_qualified_profile_executes_its_source_answer_through_native_graph_and_named_viewer() {
    let fixtures: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../cube/crates/cube-core/src/operations/contracts/conformance.json"
    ))
    .unwrap();
    let names = [
        "cadenus",
        "swagman",
        "nihilist_transposition",
        "ubchi",
        "cm_bifid",
        "vic",
    ];
    let mut count = 0;
    for f in fixtures.iter().filter(|f| {
        names.contains(
            &f["operation"]
                .as_str()
                .unwrap()
                .split('.')
                .nth(1)
                .unwrap_or(""),
        )
    }) {
        let operation = f["operation"].as_str().unwrap();
        let actual = run(
            operation,
            f["inputs"]["text"].as_str().unwrap(),
            f["parameters"].clone(),
            operation.starts_with("cipher.vic."),
        )
        .unwrap_or_else(|e| panic!("{operation}: {e}"));
        assert_eq!(actual, f["outputs"], "{operation}");
        count += 1;
    }
    assert_eq!(count, 12);
}

#[test]
fn native_qualified_profiles_refuse_invalid_keys_and_noncanonical_vic_text() {
    assert!(
        run(
            "cipher.swagman.encode",
            "",
            json!({"key":[[1,2,3,4],[1,2,3,4],[1,2,3,4],[1,2,3,4]]}),
            false
        )
        .is_err()
    );
    assert!(
        run(
            "cipher.ubchi.encode",
            "A",
            json!({"key_phrase":"AB AB","nulls":"Z"}),
            false
        )
        .is_err()
    );
    assert!(run("cipher.vic.encode","123",json!({"phrase":"ALLTHEPEOPLEAREDEADB","mnemonic":"ASINTOER","date_digits":"391752","personal_number":15,"message_indicator":"60115","split_at":0,"padding":""}), true).is_err());
}

#[test]
fn private_profiles_require_explicit_retention_consent() {
    let parameters = json!({"phrase":"ALLTHEPEOPLEAREDEADB","mnemonic":"ASINTOER","date_digits":"391752","personal_number":15,"message_indicator":"60115","split_at":0,"padding":""});
    assert_eq!(
        run("cipher.vic.encode", "HELLO", parameters, false),
        Err("retention_policy_unavailable")
    );
    let catalog = worker::catalog().unwrap();
    let vic = catalog
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["operation"] == "cipher.vic.encode")
        .unwrap();
    assert_eq!(vic["requires_private_retention"], true);
    assert_eq!(vic["contract"]["policy"]["private_input"], true);
}
