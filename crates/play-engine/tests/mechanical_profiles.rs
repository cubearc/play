//! Exact Core mechanical results through native typed graph admission and worker.
use play_engine::{
    graph::Definition,
    worker::{self, InputValue, PipelineInvocation},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use uuid::Uuid;
#[test]
fn mechanical_profiles_preserve_exact_private_named_outputs() {
    let rows: Vec<Value> = serde_json::from_str(include_str!(
        "../../../../cube/crates/cube-core/src/operations/contracts/conformance.json"
    ))
    .unwrap();
    for operation in [
        "cipher.m94.encode",
        "cipher.m94.decode",
        "cipher.jefferson.encode",
        "cipher.jefferson.decode",
        "cipher.m138a.encode",
        "cipher.m138a.decode",
        "cipher.wheatstone.encode",
        "cipher.wheatstone.decode",
        "cipher.kryha.encode",
        "cipher.kryha.decode",
        "cipher.alberti.encode",
        "cipher.alberti.decode",
    ] {
        let row = rows.iter().find(|r| r["operation"] == operation).unwrap();
        let node = Uuid::new_v4();
        let definition=Definition::parse(json!({"formatVersion":4,"inputs":[{"name":"source","semanticType":"text"},{"name":"settings","semanticType":"json"}],"nodes":[{"id":node,"operation":operation,"operationVersion":1,"parameters":{},"retainPrivate":true,"position":{"x":0,"y":0},"inputs":{"text":{"kind":"literal","value":row["inputs"]["text"]},"settings":{"kind":"input","name":"settings"}}}],"output":node})).unwrap_or_else(|e|panic!("{operation}: {e}"));
        let invocation = PipelineInvocation {
            definition,
            inputs: BTreeMap::from([(
                "settings".into(),
                InputValue::Structured {
                    value: row["inputs"]["settings"].clone(),
                },
            )]),
        };
        worker::validate_pipeline(&invocation).unwrap();
        let output = worker::execute_pipeline(invocation, |_| Ok(())).unwrap();
        assert_eq!(
            worker::named_outputs(operation, &output).unwrap(),
            row["outputs"],
            "{operation}"
        );
    }
}
