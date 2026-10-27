//! Adapt Core's existing closed pipelines, inert recipes and bounded sessions.
use super::field;
use cube_core::{
    errors::{ErrorCode, OperationError},
    investigation::{profile, session, triage},
    pipeline::{self, recipe::Recipe},
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
enum Adapter {
    Pipeline,
    Validate,
    Check,
    Recipe,
    RecipeValidate,
    Plan,
    Profile,
    Replay,
    Report,
    Record,
    Triage,
}
fn adapter(op: &str) -> Option<Adapter> {
    Some(match op {
        "pipeline.execute" => Adapter::Pipeline,
        "pipeline.validate" => Adapter::Validate,
        "recipe.check" => Adapter::Check,
        "recipe.execute" => Adapter::Recipe,
        "recipe.validate" => Adapter::RecipeValidate,
        "investigation.profile.plan" => Adapter::Plan,
        "investigation.profile.run" => Adapter::Profile,
        "investigation.replay" => Adapter::Replay,
        "investigation.report" => Adapter::Report,
        "investigation.session.record" => Adapter::Record,
        "investigation.triage" => Adapter::Triage,
        _ => return None,
    })
}
pub(super) fn supported(op: &str) -> bool {
    adapter(op).is_some()
}
fn invalid() -> OperationError {
    OperationError::new(
        ErrorCode::InvalidInput,
        "use an explicit bounded Core workflow definition and exact typed values",
    )
}
fn document(v: &Value) -> Result<String, OperationError> {
    serde_json::to_string(v).map_err(|_| invalid())
}
fn unsigned(value: &Value) -> Result<u64, OperationError> {
    value
        .as_u64()
        .or_else(|| value.as_str()?.parse().ok())
        .ok_or_else(invalid)
}
// Restore only the Core tagged value's native integer fields. Text and byte
// representations remain explicit; no arbitrary JSON strings are coerced.
fn pipeline_value(value: &Value) -> Result<pipeline::Value, OperationError> {
    if value["type"] == "byte_range" {
        return Ok(pipeline::Value::ByteRange {
            offset: unsigned(&value["offset"])?,
            length: if value["length"].is_null() {
                None
            } else {
                Some(unsigned(&value["length"])?)
            },
        });
    }
    serde_json::from_value(value.clone()).map_err(|_| invalid())
}
fn recipe(value: &Value) -> Result<Recipe, OperationError> {
    let mut native = value.clone();
    if native["input"]["source"] == "inline" {
        native["input"]["value"] = serde_json::to_value(pipeline_value(&native["input"]["value"])?)
            .map_err(|_| invalid())?;
    }
    Recipe::parse(&document(&native)?).map_err(|e| *e.error)
}
fn session(value: &Value) -> Result<session::Session, OperationError> {
    let mut native = value.clone();
    native["input"]["size"] = json!(unsigned(&native["input"]["size"])?);
    serde_json::from_value(native).map_err(|_| invalid())
}
fn plan(v: &Value) -> Result<pipeline::Plan, OperationError> {
    pipeline::Plan::validate(
        field(v, "input_type")?,
        field(v, "steps")?,
        field(v, "limits")?,
    )
    .map_err(|e| *e.error)
}
pub(super) fn execute(op: &str, inputs: Value, p: Value) -> Result<Value, OperationError> {
    let identity = cube_core::operations::require(op)?.id;
    let selected = adapter(op).ok_or_else(invalid)?;
    let result = (|| {
        let request = &inputs["input"];
        Ok(match selected {
            Adapter::Pipeline => {
                json!({"output":plan(&request["definition"] )?.execute(pipeline_value(&request["input"])?,&[],None).map_err(|e|*e.error)?})
            }
            Adapter::Validate => {
                let plan = plan(request)?;
                json!({"output":plan.info()})
            }
            Adapter::RecipeValidate => {
                let recipe = recipe(request)?;
                json!({"output":recipe.info()})
            }
            Adapter::Recipe => {
                let recipe = recipe(&request["recipe"])?;
                let bindings = request["bindings"]
                    .as_object()
                    .ok_or_else(invalid)?
                    .iter()
                    .map(|(k, v)| pipeline_value(v).map(|v| (k.clone(), v)))
                    .collect::<Result<BTreeMap<_, _>, _>>()?;
                json!({"output":recipe.execute(&bindings,&[],None).map_err(|e|*e.error)?})
            }
            Adapter::Check => {
                let recipe = recipe(&request["recipe"])?;
                let input = pipeline_value(&request["input"])?;
                let bindings = if let Some(slot) = recipe.info().input_slot {
                    BTreeMap::from([(slot.into(), input)])
                } else {
                    let declared = pipeline_value(&request["recipe"]["input"]["value"])?;
                    if declared != input {
                        return Err(invalid().parameter("input"));
                    }
                    BTreeMap::new()
                };
                json!({"output":recipe.check(&bindings,&pipeline_value(&request["expected"])?).map_err(|e|*e.error)?})
            }
            Adapter::Plan => {
                let profile: profile::Profile = field(request, "profile")?;
                let size = request["input_size"]
                    .as_u64()
                    .or_else(|| request["input_size"].as_str()?.parse().ok())
                    .ok_or_else(invalid)?;
                json!({"output":profile.dry_run(size).map_err(|e|*e.error)?})
            }
            Adapter::Profile => {
                let profile: profile::Profile = field(&inputs, "profile")?;
                json!({"output":profile.run_bytes(&super::bytes(request)?,None).map_err(|e|*e.error)?.session})
            }
            Adapter::Replay => {
                let session = session(&request["session"])?;
                json!({"output":session.replay(&super::bytes(&request["input"])?,None).map_err(|e|*e.error)?})
            }
            Adapter::Report => {
                let session = session(&inputs["input"])?;
                let report = session
                    .to_json(&session::Export::default())
                    .map_err(|e| *e.error)?;
                json!({"output":serde_json::from_str::<Value>(&report).map_err(|_|invalid())?})
            }
            Adapter::Record => {
                let bytes = super::bytes(&request["input"])?;
                let mut session =
                    session::Session::new(bytes.len() as u64, field(request, "limits")?)
                        .map_err(|e| *e.error)?;
                session
                    .execute(
                        &bytes,
                        field(request, "spec")?,
                        session::Capture {
                            parameters: field(request, "record_parameters")?,
                            result: field(request, "retain_result")?,
                        },
                        None,
                    )
                    .map_err(|e| *e.error)?;
                json!({"output":session})
            }
            Adapter::Triage => {
                json!({"output":triage::bytes(&super::bytes(request)?,field(&p,"max_read_bytes")?,&cube_core::operations::catalog())?})
            }
        })
    })();
    result.map_err(|e: OperationError| e.operation(identity))
}
#[cfg(test)]
mod tests {
    use super::super::images::bytes;
    use super::*;
    fn run(op: &str, inputs: Value, mut p: Value) -> Result<Value, OperationError> {
        let c = cube_core::operations::require(op)?.contract().unwrap();
        for (k, s) in c.parameters.as_value()["properties"].as_object().unwrap() {
            if let Some(d) = s.get("default") {
                p.as_object_mut()
                    .unwrap()
                    .entry(k)
                    .or_insert_with(|| d.clone());
            }
        }
        c.validate_inputs(&inputs)
            .unwrap_or_else(|e| panic!("{op} inputs: {e}"));
        c.validate_parameters(&p).unwrap();
        let result = cube_core::operations::contracts::canonical_value(execute(op, inputs, p)?);
        c.validate_outputs(&result)
            .unwrap_or_else(|e| panic!("{op} outputs: {e}"));
        Ok(result)
    }
    #[test]
    fn inert_ranges_and_session_sizes_preserve_full_u64_wire_values() {
        let value = json!({"type":"byte_range","offset":"18446744073709551615","length":null});
        let definition =
            json!({"input_type":"byte_range","steps":[],"limits":pipeline::Limits::default()});
        let result = run(
            "pipeline.execute",
            json!({"input":{"definition":definition,"input":value}}),
            json!({}),
        )
        .unwrap();
        assert_eq!(result["output"]["output"], value);
        let recipe = json!({"format_version":1,"engine_api":1,"input":{"source":"inline","value":value},"output":{"type":"byte_range"},"steps":[]});
        let result = run(
            "recipe.execute",
            json!({"input":{"recipe":recipe,"bindings":{}}}),
            json!({}),
        )
        .unwrap();
        assert_eq!(result["output"]["output"], value);
        let metadata = session::Session::new(u64::MAX, Default::default()).unwrap();
        let metadata = cube_core::operations::contracts::canonical_value(json!(metadata));
        let report = run("investigation.report", json!({"input":metadata}), json!({})).unwrap();
        assert_eq!(
            report["output"]["session"]["input"]["size"],
            "18446744073709551615"
        );
    }
    #[test]
    fn eleven_existing_workflow_adapters_execute_real_closed_contracts_and_keep_replay_gaps() {
        let definition = json!({"input_type":"bytes","steps":[{"operation":"encoding.hex.encode","version":1}],"limits":pipeline::Limits::default()});
        let recipe = json!({"format_version":1,"engine_api":1,"input":{"source":"slot","name":"payload","type":"bytes"},"output":{"type":"text"},"steps":[{"operation":"encoding.hex.encode","version":1}]});
        let value = json!({"type":"bytes","base64":"AP8="});
        let profile = json!(profile::Profile {
            identify: true,
            ..Default::default()
        });
        let session=run("investigation.session.record",json!({"input":{"input":bytes(&[0,255]),"spec":{"kind":"identity"},"limits":session::SessionLimits::default(),"record_parameters":false,"retain_result":false}}),json!({})).unwrap()["output"].clone();
        assert_eq!(
            session["input"]["digest"]["value"],
            cube_core::hashing::sha256(&[0, 255])
        );
        let cases = [
            (
                "pipeline.validate",
                json!({"input":definition}),
                "/output/output_type",
                json!("text"),
            ),
            (
                "pipeline.execute",
                json!({"input":{"definition":definition,"input":value}}),
                "/output/output/data",
                json!("00ff"),
            ),
            (
                "recipe.validate",
                json!({"input":recipe}),
                "/output/input_slot",
                json!("payload"),
            ),
            (
                "recipe.execute",
                json!({"input":{"recipe":recipe,"bindings":{"payload":value}}}),
                "/output/output/data",
                json!("00ff"),
            ),
            (
                "recipe.check",
                json!({"input":{"recipe":recipe,"input":value,"expected":{"type":"text","data":"00ff"}}}),
                "/output/passed",
                json!(true),
            ),
            (
                "investigation.profile.plan",
                json!({"input":{"profile":profile,"input_size":2}}),
                "/output/input_size",
                json!(2),
            ),
            (
                "investigation.profile.run",
                json!({"profile":profile,"input":bytes(&[0,255])}),
                "/output/input/digest/value",
                json!(cube_core::hashing::sha256(&[0, 255])),
            ),
            (
                "investigation.replay",
                json!({"input":{"input":bytes(&[0,255]),"session":session}}),
                "/output/input/size",
                json!(2),
            ),
            (
                "investigation.report",
                json!({"input":session}),
                "/output/schema_version",
                json!(1),
            ),
            (
                "investigation.triage",
                json!({"input":bytes(b"%PDF-1.7")}),
                "/output/inspected_bytes",
                json!(8),
            ),
        ];
        for (op, i, pointer, expected) in cases {
            let result = run(op, i, json!({})).unwrap_or_else(|e| panic!("{op}: {e}"));
            assert_eq!(result.pointer(pointer), Some(&expected), "{op}");
        }
        assert!(
            run(
                "investigation.replay",
                json!({"input":{"input":bytes(b"different"),"session":session}}),
                json!({})
            )
            .is_err()
        );
        let mut bad = definition;
        bad["steps"][0]["operation"] = json!("shell.execute");
        assert!(run("pipeline.validate", json!({"input":bad}), json!({})).is_err());
    }
}
