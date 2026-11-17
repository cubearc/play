//! Exact private numeral strings through the real native graph and JSON viewer.
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
        json!({"formatVersion":4,"inputs":[{"name":"source","semanticType":"json"}],"nodes":[{"id":node,"operation":operation,"operationVersion":1,"parameters":parameters,"retainPrivate":retain,"position":{"x":0,"y":0},"inputs":{"input":{"kind":"input","name":"source"}}}],"output":node}),
    )?;
    let invocation = PipelineInvocation {
        definition,
        inputs: BTreeMap::from([("source".into(), InputValue::Structured { value: input })]),
    };
    worker::validate_pipeline(&invocation)?;
    let output = worker::execute_pipeline(invocation, |_| Ok(()))?;
    worker::named_outputs(operation, &output)
}
#[test]
fn all_fpe_known_answers_cross_private_graphs_and_json_viewing() {
    let cases: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../cube/crates/cube-core/src/ciphers/fpe/vectors.json"
    ))
    .unwrap();
    for v in cases {
        for (op, input, expected) in [
            ("cipher.fpe.encrypt", &v["plaintext"], &v["ciphertext"]),
            ("cipher.fpe.decrypt", &v["ciphertext"], &v["plaintext"]),
        ] {
            assert_eq!(
                run(op, input.clone(), v["parameters"].clone(), true).unwrap(),
                json!({"output":expected})
            );
            assert!(run(op, input.clone(), v["parameters"].clone(), false).is_err());
        }
    }
}
