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
        "cipher.hagelin.encode",
        "cipher.hagelin.decode",
        "cipher.t52ab.encode",
        "cipher.t52ab.decode",
        "cipher.nema.encode",
        "cipher.nema.decode",
        "cipher.lorenz.encode",
        "cipher.lorenz.decode",
    ] {
        let row = rows.iter().find(|r| r["operation"] == operation).unwrap();
        let node = Uuid::new_v4();
        let input = if row["inputs"].get("codes").is_some() {
            "codes"
        } else {
            "text"
        };
        let definition=Definition::parse(json!({"formatVersion":4,"inputs":[{"name":"source","semanticType":"text"},{"name":"settings","semanticType":"json"}],"nodes":[{"id":node,"operation":operation,"operationVersion":1,"parameters":{},"retainPrivate":true,"position":{"x":0,"y":0},"inputs":{(input):{"kind":"literal","value":row["inputs"][input]},"settings":{"kind":"input","name":"settings"}}}],"output":node})).unwrap_or_else(|e|panic!("{operation}: {e}"));
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

#[test]
fn all_five_hagelin_profiles_cross_the_native_typed_graph_boundary() {
    let cases = [
        ("c36_fixed25_bouch_v1", vec![25, 23, 21, 19, 17], vec![3; 5]),
        (
            "c38_standard27_v1",
            vec![26, 25, 23, 21, 19, 17],
            vec![3; 6],
        ),
        ("m209_standard_v1", vec![26, 25, 23, 21, 19, 17], vec![3; 6]),
        (
            "c52_c2_27_v1",
            vec![26, 25, 46, 42, 38, 34],
            vec![3, 3, 6, 6, 6, 6],
        ),
        ("cx52_m27_late_v1", vec![47; 6], vec![3, 0, 0, 0, 0, 0]),
    ];
    for (profile, lengths, state) in cases {
        let mut settings = json!({"profile":profile,"pins":lengths.iter().map(|n|"0".repeat(*n)).collect::<Vec<_>>(),"initial_state":vec![0;lengths.len()]});
        if !profile.starts_with("c36_") {
            settings["lugs"] = json!(vec![vec![false; 6]; 27]);
        }
        if profile != "m209_standard_v1" {
            settings["slide"] = json!(0);
        }
        let expected = if profile == "m209_standard_v1" {
            "ZYX"
        } else {
            "AZY"
        };
        for (operation, input, expected) in [
            ("cipher.hagelin.encode", "ABC", expected),
            ("cipher.hagelin.decode", expected, "ABC"),
        ] {
            let node = Uuid::new_v4();
            let definition=Definition::parse(json!({"formatVersion":4,"inputs":[{"name":"source","semanticType":"text"},{"name":"settings","semanticType":"json"}],"nodes":[{"id":node,"operation":operation,"operationVersion":1,"parameters":{},"retainPrivate":true,"position":{"x":0,"y":0},"inputs":{"text":{"kind":"literal","value":input},"settings":{"kind":"input","name":"settings"}}}],"output":node})).unwrap();
            let invocation = PipelineInvocation {
                definition,
                inputs: BTreeMap::from([(
                    "settings".into(),
                    InputValue::Structured {
                        value: settings.clone(),
                    },
                )]),
            };
            worker::validate_pipeline(&invocation).unwrap();
            let output = worker::execute_pipeline(invocation, |_| Ok(())).unwrap();
            let actual = worker::named_outputs(operation, &output).unwrap();
            assert_eq!(actual["text"], expected, "{profile} {operation}");
            assert_eq!(actual["final_state"], json!(state), "{profile} {operation}");
        }
    }
}
