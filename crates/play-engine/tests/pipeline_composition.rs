use play_engine::{
    graph::Definition,
    worker::{self, InputValue, PipelineInvocation},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use uuid::Uuid;

fn node(id: Uuid, op: &str, inputs: Value, parameters: Value) -> Value {
    json!({"id":id,"operation":op,"operationVersion":1,"inputs":inputs,"parameters":parameters,"position":{"x":40,"y":40}})
}
fn invocation(nodes: Vec<Value>, output: Uuid, text: Option<String>) -> PipelineInvocation {
    PipelineInvocation {
        definition: Definition::parse(json!({"formatVersion":4,"inputs":[{"name":"source","semanticType":"text"}],"nodes":nodes,"output":output})).unwrap(),
        inputs: text.map(|text| BTreeMap::from([("source".into(),InputValue::Text { text })])).unwrap_or_default(),
    }
}
#[test]
fn native_catalog_uses_installed_names_contracts_and_valid_defaults_for_every_composition_adapter()
{
    let entries = worker::catalog().unwrap();
    for (id, name, kind) in [
        ("text.concat", "Concatenate text", "text"),
        ("bytes.concat", "Concatenate bytes", "bytes"),
        ("text.utf8.encode", "Encode UTF-8", "text"),
        ("text.utf8.decode", "Decode UTF-8", "bytes"),
    ] {
        let e = entries
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["operation"] == id)
            .unwrap();
        assert_eq!(e["name"], name);
        assert_eq!(e["input_type"], kind);
        worker::validate_configuration(id, 1, &e["parameters"]).unwrap();
        assert_eq!(
            e["contract"],
            serde_json::to_value(
                cube_core::operations::require(id)
                    .unwrap()
                    .contract()
                    .unwrap()
            )
            .unwrap()
        );
    }
}
#[test]
fn byte_join_keeps_non_utf8_values_and_explicit_port_order() {
    let id = Uuid::new_v4();
    let p = invocation(
        vec![node(
            id,
            "bytes.concat",
            json!({"left":{"kind":"literal","value":{"base64":"AP8="}},"right":{"kind":"literal","value":{"base64":"gAo="}}}),
            json!({}),
        )],
        id,
        None,
    );
    let result = worker::execute_pipeline(p, |_| Ok(())).unwrap();
    assert_eq!(result, json!({"bytes":[0,255,128,10]}));
    assert_eq!(
        worker::named_outputs("bytes.concat", &result).unwrap(),
        json!({"output":{"base64":"AP+ACg=="}})
    );
}
#[test]
fn invalid_utf8_preserves_the_finished_producer_and_never_starts_its_consumer() {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();
    let p = invocation(
        vec![
            node(
                a,
                "bytes.concat",
                json!({"left":{"kind":"literal","value":{"base64":"/w=="}},"right":{"kind":"literal","value":{"base64":""}}}),
                json!({}),
            ),
            node(
                b,
                "text.utf8.decode",
                json!({"input":{"kind":"output","nodeId":a,"port":"output"}}),
                json!({}),
            ),
            node(
                c,
                "text.concat",
                json!({"left":{"kind":"output","nodeId":b,"port":"output"},"right":{"kind":"literal","value":"hidden"}}),
                json!({}),
            ),
        ],
        c,
        None,
    );
    let mut events = Vec::new();
    assert_eq!(
        worker::execute_pipeline(p, |e| {
            events.push(e);
            Ok(())
        }),
        Err("invalid_encoding")
    );
    assert!(
        events
            .iter()
            .any(|e| e["nodeId"] == json!(a) && e["type"] == "node_finished")
    );
    assert!(!events.iter().any(|e| e["nodeId"] == json!(c)));
}
#[test]
fn aggregate_operand_and_output_ceilings_apply_to_composition_and_utf8_round_trips() {
    let id = Uuid::new_v4();
    let join = |separator| {
        invocation(
            vec![node(
                id,
                "text.concat",
                json!({"left":{"kind":"input","name":"source"},"right":{"kind":"input","name":"source"}}),
                json!({"separator":separator}),
            )],
            id,
            Some("é".repeat(2048)),
        )
    };
    assert_eq!(
        worker::execute_pipeline(join(""), |_| Ok(())).unwrap()["text"]
            .as_str()
            .unwrap()
            .len(),
        8192
    );
    let mut events = Vec::new();
    assert_eq!(
        worker::execute_pipeline(join("x"), |e| {
            events.push(e);
            Ok(())
        }),
        Err("input_limit")
    );
    assert!(events.is_empty());
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let c = Uuid::new_v4();
    let mut nodes = vec![
        node(
            a,
            "text.utf8.encode",
            json!({"input":{"kind":"input","name":"source"}}),
            json!({}),
        ),
        node(
            b,
            "text.utf8.decode",
            json!({"input":{"kind":"output","nodeId":a,"port":"output"}}),
            json!({}),
        ),
    ];
    let input = "x".repeat(8192);
    assert_eq!(
        worker::execute_pipeline(
            invocation(nodes.clone(), b, Some(input.clone())),
            |_| Ok(())
        )
        .unwrap()["text"],
        input
    );
    nodes.push(node(
        c,
        "cipher.rot13",
        json!({"input":{"kind":"output","nodeId":b,"port":"output"}}),
        json!({}),
    ));
    assert_eq!(
        worker::execute_pipeline(invocation(nodes, c, Some(input)), |_| Ok(())),
        Err("output_limit")
    );
}
