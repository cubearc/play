use play_engine::{
    graph::Definition,
    worker::{self, PipelineInvocation},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use uuid::Uuid;

fn run(text: &str, parameters: Value) -> Result<Value, &'static str> {
    let node = Uuid::new_v4();
    let definition = Definition::parse(
        json!({"formatVersion":4,"inputs":[{"name":"source","semanticType":"text"}],"nodes":[{"id":node,"operation":"cipher.bellaso","operationVersion":1,"parameters":parameters,"inputs":{"text":{"kind":"literal","value":text}},"position":{"x":40,"y":40}}],"output":node}),
    )?;
    let invocation = PipelineInvocation {
        definition,
        inputs: BTreeMap::new(),
    };
    worker::validate_pipeline(&invocation)?;
    let raw = worker::execute_pipeline(invocation, |_| Ok(()))?;
    worker::named_outputs("cipher.bellaso", &raw)
}

#[test]
fn printed_bellaso_example_and_inverse_pass_through_native_graph_and_named_viewer() {
    let fixtures: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../cube/crates/cube-core/src/operations/contracts/conformance.json"
    ))
    .unwrap();
    let fixture = fixtures
        .iter()
        .find(|f| f["operation"] == "cipher.bellaso")
        .unwrap();
    let plain = fixture["inputs"]["text"].as_str().unwrap();
    let output = run(plain, fixture["parameters"].clone()).unwrap();
    assert_eq!(output, fixture["outputs"]);
    assert_eq!(
        run(
            output["text"].as_str().unwrap(),
            fixture["parameters"].clone()
        )
        .unwrap(),
        fixture["inputs"]
    );
    assert_eq!(run("U", json!({"key":"A"})), Err("invalid_input"));
}
