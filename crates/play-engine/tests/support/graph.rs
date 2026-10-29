use serde_json::{Value, json};
use uuid::Uuid;
// A complete graph fixture, deliberately unchecked so rejection tests reach the engine boundary.
pub fn configuration(
    operation: impl serde::Serialize,
    version: impl serde::Serialize,
    parameters: Value,
) -> Value {
    let operation = serde_json::to_value(operation).unwrap();
    let id = Uuid::from_u128(1);
    let mut inputs = if operation == "esolang.brainfuck.execute" {
        json!({"source":{"kind":"input","name":"source"},"input":{"kind":"literal","value":[]}})
    } else {
        json!({"input":{"kind":"input","name":"source"}})
    };
    if operation == "image.pixels.read" {
        inputs = json!({"input":{"kind":"input","name":"source"}});
    }
    json!({"formatVersion":3,"inputs":[{"name":"source","semanticType":if operation=="image.pixels.read" {"image.samples"}else{"text"}}],"nodes":[{"id":id,"operation":operation,"operationVersion":version,"parameters":parameters,"inputs":inputs,"position":{"x":80,"y":120}}],"output":id})
}
