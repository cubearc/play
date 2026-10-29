//! Typed API transport; Core owns selection, conversions and numeric algorithms.
use super::field;
use cube_core::{composition::structured as s, errors::OperationError};
use serde_json::{Value, json};
pub(super) fn supported(id: &str) -> bool {
    matches!(
        id,
        "json.parse"
            | "json.select"
            | "json.text"
            | "text.integer"
            | "number.maximum"
            | "number.add"
            | "image.dimensions"
    )
}
pub(super) fn execute(id: &str, inputs: Value, parameters: Value) -> Result<Value, OperationError> {
    let input = &inputs["input"];
    Ok(json!({"output":match id {
        "json.parse"=>s::parse_json(&field::<String>(&inputs,"input")?)?,
        "json.select"=>s::select_field(input,&field::<String>(&parameters,"pointer")?)?,
        "json.text"=>json!(s::json_text(input)?),
        "text.integer"=>json!(s::parse_integer(&field::<String>(&inputs,"input")?)?),
        "number.maximum"=>json!(s::maximum(&field::<Vec<i64>>(&inputs,"input")?)?),
        "number.add"=>json!(s::add(field::<i64>(&inputs,"left")?,field::<i64>(&inputs,"right")?)?),
        "image.dimensions"=>json!(cube_core::image::dimensions::inspect(&super::images::raster(&inputs["input"])?)),
        _=>unreachable!("only supported structured operations dispatch here"),
    }}))
}
