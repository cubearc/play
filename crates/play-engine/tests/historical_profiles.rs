//! The actual installed Core contracts pass through native graph admission,
//! pure worker execution and named-output/viewer transport without an ID map.
use play_engine::{
    graph::Definition,
    worker::{self, PipelineInvocation},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use uuid::Uuid;
#[test]
fn all_milestone23_fixtures_execute_through_the_native_worker() {
    let fixtures: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../cube/crates/cube-core/src/operations/contracts/conformance.json"
    ))
    .unwrap();
    let ids = [
        "cipher.rot8000",
        "cipher.malespin",
        "cipher.vatsyayana",
        "cipher.nomenclator.encode",
        "cipher.nomenclator.decode",
        "cipher.syllabary.encode",
        "cipher.syllabary.decode",
        "cipher.great_cipher.encode",
        "cipher.great_cipher.decode",
    ];
    let catalog = worker::catalog().unwrap();
    for id in ids {
        assert!(
            catalog
                .as_array()
                .unwrap()
                .iter()
                .any(|entry| entry["operation"] == id),
            "{id} missing native availability"
        );
        let fixture = fixtures.iter().find(|f| f["operation"] == id).unwrap();
        let node = Uuid::new_v4();
        let inputs = fixture["inputs"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(name, value)| (name.clone(), json!({"kind":"literal","value":value})))
            .collect::<serde_json::Map<_, _>>();
        let definition=Definition::parse(json!({"formatVersion":4,"inputs":[{"name":"source","semanticType":"text"}],"nodes":[{"id":node,"operation":id,"operationVersion":1,"parameters":fixture["parameters"],"inputs":inputs,"position":{"x":40,"y":40}}],"output":node})).unwrap();
        let invocation = PipelineInvocation {
            definition,
            inputs: BTreeMap::new(),
        };
        worker::validate_pipeline(&invocation).unwrap();
        let mut events = Vec::new();
        let raw = worker::execute_pipeline(invocation, |event| {
            events.push(event);
            Ok(())
        })
        .unwrap();
        assert_eq!(
            worker::named_outputs(id, &raw).unwrap(),
            fixture["outputs"],
            "{id}"
        );
        assert!(
            events
                .iter()
                .any(|e| e["type"] == "node_finished" && e["nodeId"] == node.to_string()),
            "{id}"
        );
    }
}
#[test]
fn unknown_great_cipher_group_is_an_execution_error_without_a_result() {
    let node = Uuid::new_v4();
    let definition=Definition::parse(json!({"formatVersion":4,"inputs":[{"name":"source","semanticType":"text"}],"nodes":[{"id":node,"operation":"cipher.great_cipher.decode","operationVersion":1,"parameters":{},"inputs":{"tokens":{"kind":"literal","value":["6"]}},"position":{"x":40,"y":40}}],"output":node})).unwrap();
    let mut events = Vec::new();
    assert_eq!(
        worker::execute_pipeline(
            PipelineInvocation {
                definition,
                inputs: BTreeMap::new()
            },
            |event| {
                events.push(event);
                Ok(())
            }
        ),
        Err("invalid_input")
    );
    assert!(
        !events
            .iter()
            .any(|e| e["type"] == "node_finished" && e["status"] == "completed")
    );
}
