//! Existing bounded Core graph, Boolean and exact finite-probability models.
use super::field;
use cube_core::{
    errors::{ErrorCode, OperationError},
    graph,
    logic::boolean,
    probability,
};
use serde::Serialize;
use serde_json::{Value, json};

enum Adapter {
    Components,
    Shortest,
    Cycle,
    Euler,
    Evaluate,
    Analyze,
    Finite,
    DiceSum,
    DiceCompare,
}
fn adapter(op: &str) -> Option<Adapter> {
    Some(match op {
        "graph.components" => Adapter::Components,
        "graph.shortest_path" => Adapter::Shortest,
        "graph.cycle" => Adapter::Cycle,
        "graph.euler_trail" => Adapter::Euler,
        "logic.boolean.evaluate" => Adapter::Evaluate,
        "logic.boolean.analyze" => Adapter::Analyze,
        "probability.finite.analyze" => Adapter::Finite,
        "probability.dice.sum" => Adapter::DiceSum,
        "probability.dice.compare" => Adapter::DiceCompare,
        _ => return None,
    })
}
pub(super) fn supported(op: &str) -> bool {
    adapter(op).is_some()
}
fn invalid() -> OperationError {
    OperationError::new(
        ErrorCode::InvalidInput,
        "use the exact structured Core model and parameter fields",
    )
}
fn document(value: &Value) -> Result<String, OperationError> {
    serde_json::to_string(value).map_err(|_| invalid())
}
fn output(value: impl Serialize) -> Result<Value, OperationError> {
    serde_json::to_value(value)
        .map(|value| json!({"output":value}))
        .map_err(|_| invalid())
}
pub(super) fn execute(op: &str, inputs: Value, p: Value) -> Result<Value, OperationError> {
    let identity = cube_core::operations::require(op)?.id;
    let selected = adapter(op).ok_or_else(invalid)?;
    let result = (|| match selected {
        Adapter::Components | Adapter::Shortest | Adapter::Cycle | Adapter::Euler => {
            let limits = graph::Limits {
                max_work: field(&p, "max_work")?,
                max_memory_bytes: field(&p, "max_memory_bytes")?,
                max_report_bytes: field(&p, "max_report_bytes")?,
            };
            let graph = graph::Graph::new(graph::parse(&document(&inputs["input"])?)?, limits)?;
            match selected {
                Adapter::Components => output(graph.components(limits)?),
                Adapter::Shortest => output(graph.shortest_path(
                    &field::<String>(&p, "source")?,
                    &field::<String>(&p, "target")?,
                    limits,
                )?),
                Adapter::Cycle => output(graph.cycle(limits)?),
                _ => output(
                    graph.euler_trail(
                        p.get("start")
                            .map(|_| field::<String>(&p, "start"))
                            .transpose()?
                            .as_deref(),
                        limits,
                    )?,
                ),
            }
        }
        Adapter::Evaluate | Adapter::Analyze => {
            let limits = boolean::Limits {
                max_work: field(&p, "max_work")?,
                max_rows: field(&p, "max_rows")?,
                max_memory_bytes: field(&p, "max_memory_bytes")?,
                max_report_bytes: field(&p, "max_report_bytes")?,
            };
            if matches!(selected, Adapter::Evaluate) {
                let model = boolean::parse(&document(&inputs["model"])?)?;
                let assignment = boolean::parse_assignment(&document(&inputs["assignment"])?)?;
                output(boolean::evaluate(&model, &assignment, limits)?)
            } else {
                output(boolean::analyze(
                    &boolean::parse(&document(&inputs["input"])?)?,
                    limits,
                )?)
            }
        }
        Adapter::Finite | Adapter::DiceSum | Adapter::DiceCompare => {
            let limits = probability::Limits {
                max_work: field(&p, "max_work")?,
                max_memory_bytes: field(&p, "max_memory_bytes")?,
                max_report_bytes: field(&p, "max_report_bytes")?,
            };
            let document = document(&inputs["input"])?;
            match selected {
                Adapter::Finite => output(probability::finite_analyze(
                    &probability::parse_finite(&document)?,
                    limits,
                )?),
                Adapter::DiceSum => output(probability::dice_sum(
                    &probability::parse_dice(&document)?,
                    limits,
                )?),
                _ => output(probability::dice_compare(
                    &probability::parse_pair(&document)?,
                    limits,
                )?),
            }
        }
    })();
    result.map_err(|error: OperationError| error.operation(identity))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn parameters(op: &str, mut p: Value) -> Value {
        let c = cube_core::operations::require(op)
            .unwrap()
            .contract()
            .unwrap();
        for (k, v) in c.parameters.as_value()["properties"].as_object().unwrap() {
            if let Some(default) = v.get("default") {
                p.as_object_mut()
                    .unwrap()
                    .entry(k)
                    .or_insert_with(|| default.clone());
            }
        }
        p
    }
    #[test]
    fn nine_core_model_adapters_preserve_known_witnesses_and_exact_probability_types() {
        let graph = json!({"vertices":["A","B","C"],"directed":false,"edges":[{"id":"ab","from":"A","to":"B"},{"id":"bc","from":"B","to":"C"}]});
        let variable = json!({"variables":["A"],"formula":{"op":"var","name":"A"}});
        let tautology = json!({"variables":["A"],"formula":{"op":"or","left":{"op":"var","name":"A"},"right":{"op":"not","operand":{"op":"var","name":"A"}}}});
        let die = json!({"dice":[[{"value":1,"weight":"1"},{"value":2,"weight":"1"}]]});
        let cases = [
            (
                "graph.components",
                json!({"input":graph}),
                json!({}),
                "/output/result",
                json!([["A", "B", "C"]]),
            ),
            (
                "graph.shortest_path",
                json!({"input":graph}),
                json!({"source":"A","target":"C"}),
                "/output/result/vertices",
                json!(["A", "B", "C"]),
            ),
            (
                "graph.cycle",
                json!({"input":graph}),
                json!({}),
                "/output/result",
                Value::Null,
            ),
            (
                "graph.euler_trail",
                json!({"input":graph}),
                json!({"start":"A"}),
                "/output/result/path/vertices",
                json!(["A", "B", "C"]),
            ),
            (
                "logic.boolean.analyze",
                json!({"input":tautology}),
                json!({"max_rows":2}),
                "/output/tautology",
                json!(true),
            ),
            (
                "logic.boolean.evaluate",
                json!({"model":variable,"assignment":[{"name":"A","value":true}]}),
                json!({}),
                "/output/value",
                json!(true),
            ),
            (
                "probability.finite.analyze",
                json!({"input":{"outcomes":[{"id":"red","weight":"1","value":1},{"id":"blue","weight":"1","value":2}],"event_a":["red"]}}),
                json!({}),
                "/output/probability_a",
                json!({"numerator":"1","denominator":"2"}),
            ),
            (
                "probability.dice.sum",
                json!({"input":die}),
                json!({}),
                "/output/expected_value",
                json!({"negative":false,"numerator":"3","denominator":"2"}),
            ),
            (
                "probability.dice.compare",
                json!({"input":{"left":{"dice":[[{"value":2,"weight":"1"}]]},"right":{"dice":[[{"value":1,"weight":"1"}]]}}}),
                json!({}),
                "/output/win",
                json!({"numerator":"1","denominator":"1"}),
            ),
        ];
        for (op, inputs, p, pointer, expected) in cases {
            assert!(supported(op));
            let contract = cube_core::operations::require(op)
                .unwrap()
                .contract()
                .unwrap();
            let p = parameters(op, p);
            contract
                .validate_inputs(&inputs)
                .unwrap_or_else(|e| panic!("{op} inputs: {e}"));
            contract.validate_parameters(&p).unwrap();
            let actual = execute(op, inputs.clone(), p.clone())
                .unwrap_or_else(|e| panic!("{op} execution: {e}"));
            contract
                .validate_outputs(&actual)
                .unwrap_or_else(|e| panic!("{op} outputs: {e}"));
            assert_eq!(actual.pointer(pointer), Some(&expected), "{op}");
            let mut exhausted = p;
            exhausted["max_work"] = json!(0);
            assert!(
                execute(op, inputs, exhausted).is_err(),
                "{op} resource budget"
            );
        }
        assert!(!supported("unknown.operation"));
    }
}
