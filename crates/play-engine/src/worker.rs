//! Native transport for the Core-owned pure-data execution registry.
use crate::{
    Result,
    protocol::{self, Invocation},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use cube_core::operations::execution::{self as core, Budget, Effects, Representation};
use serde::Deserialize;
use serde_json::{Value, json};
use std::io::{self, Read};

fn budget() -> Budget {
    Budget {
        input_bytes: protocol::contract().input_bytes,
        output_bytes: protocol::contract().output_bytes,
        ..Budget::default()
    }
}
fn failure(error: core::Failure) -> &'static str {
    error.0
}
fn executable(operation: &str) -> Result<core::Executable> {
    let e = core::describe(operation, budget()).map_err(failure)?;
    if e.effects != Effects::Pure {
        return Err("unsupported_operation");
    }
    if e.contract.policy.private_input
        || e.contract.policy.private_output
        || !e.contract.policy.sensitive_parameters.is_empty()
    {
        return Err("retention_policy_unavailable");
    }
    // The native host can materialize images as inputs; it does not yet retain
    // generated rasters. Unsupported representations never gain execution.
    if e.outputs
        .values()
        .any(|r| matches!(r, Representation::ImageSamples))
    {
        return Err("unsupported_representation");
    }
    Ok(e)
}
pub fn validate_configuration(operation: &str, version: u32, parameters: &Value) -> Result<()> {
    executable(operation)?;
    core::validate_parameters(operation, version, parameters, budget()).map_err(failure)
}
pub fn parameters(operation: &str) -> Result<Value> {
    Ok(executable(operation)?.parameters)
}
fn image(input: &str) -> Result<cube_core::image::Image> {
    let data = STANDARD.decode(input).map_err(|_| "invalid_image")?;
    if data.len() > protocol::contract().image_bytes || !data.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("invalid_image");
    }
    cube_core::image::Image::decode(&data).map_err(|_| "invalid_image")
}
pub fn empty_value(representation: Representation) -> Result<Value> {
    match representation {
        Representation::Text => Ok(json!("")),
        Representation::Bytes => Ok(json!({"base64":""})),
        Representation::ByteArray => Ok(json!([])),
        Representation::Json => Ok(Value::Null),
        Representation::ImageSamples => Err("invalid_binding"),
    }
}
pub fn named_inputs(invocation: &Invocation) -> Result<Value> {
    let e = executable(&invocation.operation)?;
    let primary = e
        .contract
        .inputs
        .iter()
        .find(|p| ["text", "image.samples"].contains(&p.semantic_type.as_str()))
        .ok_or("contract_mismatch")?;
    let mut inputs = serde_json::Map::new();
    for port in &e.contract.inputs {
        let representation = *e.inputs.get(&port.name).ok_or("contract_mismatch")?;
        let value = if port.name == primary.name {
            match representation {
                Representation::Text => json!(invocation.input),
                Representation::ImageSamples => {
                    json!(cube_core::operations::contracts::values::Image::from(
                        &image(&invocation.input)?
                    ))
                }
                _ => return Err("contract_mismatch"),
            }
        } else {
            empty_value(representation)?
        };
        inputs.insert(port.name.clone(), value);
    }
    Ok(Value::Object(inputs))
}
pub fn validate(invocation: &Invocation) -> Result<()> {
    let e = executable(&invocation.operation)?;
    let maximum = if e
        .inputs
        .values()
        .any(|r| *r == Representation::ImageSamples)
    {
        protocol::contract().image_bytes.div_ceil(3) * 4
    } else {
        protocol::contract().input_bytes
    };
    if invocation.input.len() > maximum {
        return Err("input_limit");
    }
    validate_configuration(
        &invocation.operation,
        invocation.operation_version,
        &invocation.parameters,
    )?;
    core::validate(
        &invocation.operation,
        invocation.operation_version,
        &named_inputs(invocation)?,
        &invocation.parameters,
        budget(),
    )
    .map_err(failure)
}
// Worker transport keeps compact single values and complete named maps for
// multiple outputs. This choice depends only on representation and port count.
fn raw_outputs(operation: &str, named: &Value) -> Result<Value> {
    let e = executable(operation)?;
    if e.outputs.len() != 1 {
        return Ok(json!({"ports":named}));
    }
    let (name, representation) = e.outputs.first_key_value().ok_or("contract_mismatch")?;
    let v = named.get(name).ok_or("invalid_response")?;
    match representation {
        Representation::Text => Ok(json!({"text":v})),
        Representation::Bytes => Ok(
            json!({"bytes":STANDARD.decode(v["base64"].as_str().ok_or("invalid_response")?).map_err(|_|"invalid_response")?}),
        ),
        Representation::ByteArray => Ok(json!({"bytes":v})),
        Representation::Json => Ok(json!({"value":v})),
        Representation::ImageSamples => Err("unsupported_representation"),
    }
}
pub fn named_outputs(operation: &str, output: &Value) -> Result<Value> {
    let e = executable(operation)?;
    let named = if let Some(ports) = output.get("ports") {
        ports.clone()
    } else {
        if e.outputs.len() != 1 {
            return Err("invalid_response");
        }
        let (name, representation) = e.outputs.first_key_value().ok_or("contract_mismatch")?;
        let value = match representation {
            Representation::Text => output.get("text").cloned().ok_or("invalid_response")?,
            Representation::Json => output.get("value").cloned().ok_or("invalid_response")?,
            Representation::Bytes => {
                let data: Vec<u8> = serde_json::from_value(output["bytes"].clone())
                    .map_err(|_| "invalid_response")?;
                json!(cube_core::operations::contracts::values::Bytes { data })
            }
            Representation::ByteArray => output.get("bytes").cloned().ok_or("invalid_response")?,
            Representation::ImageSamples => return Err("unsupported_representation"),
        };
        json!({name.clone():value})
    };
    e.contract
        .validate_outputs(&named)
        .map_err(|_| "invalid_response")?;
    if core::output_size(operation, &named).map_err(failure)? > protocol::contract().output_bytes {
        return Err("output_limit");
    }
    Ok(named)
}
pub fn catalog() -> Result<Value> {
    let mut entries = Vec::new();
    for registration in core::catalog(budget()).map_err(failure)? {
        let e = match executable(registration.operation) {
            Ok(e) => e,
            Err("unsupported_representation" | "retention_policy_unavailable") => continue,
            Err(e) => return Err(e),
        };
        let d = cube_core::operations::require(e.operation).map_err(|_| "contract_mismatch")?;
        entries.push(json!({"operation":e.operation,"operation_version":e.contract.operation_version,
            "name":d.metadata().map_or(d.command,|m|m.canonical_name),"description":d.description,
            "input_type":e.contract.inputs.first().ok_or("contract_mismatch")?.semantic_type,
            "parameters":e.parameters,"execution":{"effects":e.effects,"inputs":e.inputs,"outputs":e.outputs},"contract":e.contract}));
    }
    Ok(json!(entries))
}
fn execute(invocation: Invocation) -> Result<Value> {
    validate(&invocation)?;
    let named = core::execute(
        &invocation.operation,
        invocation.operation_version,
        &named_inputs(&invocation)?,
        &invocation.parameters,
        budget(),
    )
    .map_err(failure)?;
    raw_outputs(&invocation.operation, &named)
}

pub fn run() -> Result<()> {
    crate::native::restrict()?;
    let mut bytes = Vec::new();
    io::stdin()
        .take(protocol::contract().frame_bytes as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "invalid_frame")?;
    if bytes.len() > protocol::contract().frame_bytes {
        return Err("invalid_frame");
    }
    let value =
        cube_core::pipeline::json::parse(std::str::from_utf8(&bytes).map_err(|_| "invalid_frame")?)
            .map_err(|_| "invalid_frame")?;
    let task: Task = serde_json::from_value(value).map_err(|_| "invalid_command")?;
    let mut stdout = io::stdout().lock();
    let outcome = match task {
        Task::Probe { invocation } => execute(invocation),
        Task::Pipeline { invocation } => {
            execute_pipeline(invocation, |v| protocol::write_frame(&mut stdout, &v))
        }
    };
    let result = match outcome {
        Ok(output) => json!({"status":"completed", "output":output}),
        Err(code) => json!({"status":"failed", "code":code}),
    };
    protocol::write_frame(&mut stdout, &result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invokes_real_core_and_preserves_unicode() {
        let invocation = Invocation {
            operation: "cipher.rot13".into(),
            parameters: parameters("cipher.rot13").unwrap(),
            operation_version: 1,
            input: "Hello, ação!".into(),
        };
        assert_eq!(execute(invocation).unwrap(), json!({"text":"Uryyb, nçãb!"}));
        let invocation = Invocation {
            operation: "esolang.brainfuck.execute".into(),
            parameters: parameters("esolang.brainfuck.execute").unwrap(),
            operation_version: 2,
            input: "++++++++[>++++++++<-]>+.".into(),
        };
        assert_eq!(execute(invocation).unwrap(), json!({"bytes":[65]}));
    }

    #[test]
    fn pixel_adapter_uses_real_core_samples_and_catalog_filters_match_contracts() {
        let input = STANDARD.encode(include_bytes!("../../../tests/fixtures/sample.png"));
        let invocation = Invocation {
            operation: "image.pixels.read".into(),
            parameters: parameters("image.pixels.read").unwrap(),
            operation_version: 1,
            input,
        };
        assert_eq!(
            execute(invocation.clone()).unwrap(),
            json!({"bytes":[20,80,50]})
        );
        let inputs = named_inputs(&invocation).unwrap();
        let contract = cube_core::operations::require("image.pixels.read")
            .unwrap()
            .contract()
            .unwrap();
        contract.validate_inputs(&inputs).unwrap();
        contract
            .validate_parameters(&parameters("image.pixels.read").unwrap())
            .unwrap();
        contract
            .validate_outputs(
                &named_outputs("image.pixels.read", &json!({"bytes":[20,80,50]})).unwrap(),
            )
            .unwrap();
        let entries = catalog().unwrap();
        for (operation, kind) in [
            ("cipher.rot13", "text"),
            ("esolang.brainfuck.execute", "text"),
            ("image.pixels.read", "image.samples"),
        ] {
            assert_eq!(
                entries
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|e| e["operation"] == operation)
                    .unwrap()["input_type"],
                kind
            );
        }
        let mut bad = invocation;
        bad.input = "invalid".into();
        assert_eq!(execute(bad), Err("invalid_image"));
    }

    #[test]
    fn rejects_uninstalled_versions_and_unbounded_inputs() {
        let mut invocation = Invocation {
            operation: "cipher.rot13".into(),
            parameters: parameters("cipher.rot13").unwrap(),
            operation_version: 2,
            input: "hello".into(),
        };
        assert_eq!(validate(&invocation), Err("contract_mismatch"));
        invocation.operation_version = 1;
        invocation.input = "x".repeat(protocol::contract().input_bytes + 1);
        assert_eq!(validate(&invocation), Err("input_limit"));
        invocation.input.clear();
        invocation.operation = "shell".into();
        assert_eq!(validate(&invocation), Err("unsupported_operation"));
    }
}

#[cfg(test)]
mod configuration_tests {
    use super::*;
    #[test]
    fn saved_coordinates_are_executed_by_core_and_invalid_coordinates_are_rejected_before_admission()
     {
        let data = include_bytes!("../../../tests/fixtures/sample.png");
        let mut invocation = Invocation {
            operation: "image.pixels.read".into(),
            operation_version: 1,
            input: STANDARD.encode(data),
            parameters: json!({"x":1,"y":0,"expand_palette":false}),
        };
        let expected = cube_core::image::Image::decode(data)
            .unwrap()
            .pixel(1, 0)
            .unwrap();
        assert_ne!(
            expected,
            cube_core::image::Image::decode(data)
                .unwrap()
                .pixel(0, 0)
                .unwrap()
        );
        assert_eq!(
            execute(invocation.clone()).unwrap(),
            json!({"bytes":expected})
        );
        invocation.parameters["x"] = json!(u32::MAX);
        assert_eq!(validate(&invocation), Err("invalid_parameters"));
    }
    #[test]
    fn saved_brainfuck_limits_are_effective_and_cannot_raise_native_caps() {
        let mut invocation = Invocation {
            operation: "esolang.brainfuck.execute".into(),
            operation_version: 2,
            input: "++.".into(),
            parameters: parameters("esolang.brainfuck.execute").unwrap(),
        };
        invocation.parameters["max_instructions"] = json!(1);
        assert_eq!(execute(invocation.clone()), Err("operation_failed"));
        invocation.parameters["max_instructions"] = json!(1000000);
        invocation.parameters["max_output_bytes"] = json!(16385);
        assert_eq!(validate(&invocation), Err("input_limit"));
    }
}

#[derive(Clone, Debug, serde::Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PipelineInvocation {
    pub definition: crate::graph::Definition,
    pub inputs: std::collections::BTreeMap<String, InputValue>,
}
#[derive(serde::Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Task {
    Probe { invocation: Invocation },
    Pipeline { invocation: PipelineInvocation },
}

#[derive(Clone, Debug, serde::Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum InputValue {
    Text { text: String },
    Image { png: String },
}
impl InputValue {
    fn canonical(&self) -> Result<Value> {
        match self {
            Self::Text { text } => Ok(json!(text)),
            Self::Image { png } => Ok(json!(
                cube_core::operations::contracts::values::Image::from(&image(png)?)
            )),
        }
    }
}
pub fn validate_pipeline(invocation: &PipelineInvocation) -> Result<()> {
    invocation.definition.validate()?;
    let required = invocation.definition.required_inputs();
    if invocation
        .inputs
        .keys()
        .map(String::as_str)
        .collect::<std::collections::BTreeSet<_>>()
        != required
    {
        return Err("invalid_binding");
    }
    let mut text_bytes = 0usize;
    let mut image_bytes = 0usize;
    for (name, value) in &invocation.inputs {
        let kind = &invocation.definition.input(name)?.semantic_type;
        match value {
            InputValue::Text { text } if kind == "text" => {
                text_bytes = text_bytes.checked_add(text.len()).ok_or("input_limit")?;
            }
            InputValue::Image { png } if kind == "image.samples" => {
                if png.len() > protocol::contract().image_bytes.div_ceil(3) * 4 {
                    return Err("input_limit");
                }
                let data = STANDARD.decode(png).map_err(|_| "invalid_image")?;
                image_bytes = image_bytes.checked_add(data.len()).ok_or("input_limit")?;
                image(png)?;
            }
            _ => return Err("contract_mismatch"),
        }
    }
    if text_bytes > protocol::contract().input_bytes
        || image_bytes > protocol::contract().image_bytes
    {
        return Err("input_limit");
    }
    for node in &invocation.definition.nodes {
        // Preflight materialized inputs before admission. Dependencies are
        // checked at their execution boundary once their outputs exist.
        let mut values = serde_json::Map::new();
        for (port, source) in &node.inputs {
            let value = match source {
                crate::graph::Source::Input { name } => invocation
                    .inputs
                    .get(name)
                    .ok_or("invalid_binding")?
                    .canonical()?,
                crate::graph::Source::Literal { value } => value.clone(),
                crate::graph::Source::Output { .. } => continue,
            };
            values.insert(port.clone(), value);
        }
        if values.len() == node.inputs.len() {
            core::validate(
                &node.operation,
                node.operation_version,
                &Value::Object(values),
                &node.parameters,
                budget(),
            )
            .map_err(failure)?;
        }
    }
    Ok(())
}
pub fn execute_pipeline(
    invocation: PipelineInvocation,
    mut emit: impl FnMut(Value) -> Result<()>,
) -> Result<Value> {
    use crate::graph::Source;
    use std::collections::BTreeMap;
    validate_pipeline(&invocation)?;
    let mut outputs: BTreeMap<uuid::Uuid, Value> = BTreeMap::new();
    let mut used = 0usize;
    let mut final_output = Value::Null;
    for id in invocation.definition.schedule()? {
        let node = invocation.definition.node(id)?;
        let contract = node.contract()?;
        let mut values = serde_json::Map::new();
        for (port, binding) in &node.inputs {
            let value = match binding {
                Source::Input { name } => invocation
                    .inputs
                    .get(name)
                    .ok_or("invalid_binding")?
                    .canonical()?,
                Source::Literal { value } => value.clone(),
                Source::Output { node_id, port } => outputs
                    .get(node_id)
                    .and_then(|v| v.get(port))
                    .ok_or("invalid_connection")?
                    .clone(),
            };
            values.insert(port.clone(), value);
        }
        let values = Value::Object(values);
        contract
            .validate_inputs(&values)
            .map_err(|_| "contract_mismatch")?;
        let raw = execute_node(node, &values, &mut emit)?;
        let named = named_outputs(&node.operation, &raw)?;
        contract
            .validate_outputs(&named)
            .map_err(|_| "invalid_response")?;
        let size = core::output_size(&node.operation, &named).map_err(failure)?;
        used = used.checked_add(size).ok_or("output_limit")?;
        if used > protocol::contract().output_bytes {
            return Err("output_limit");
        }
        emit(json!({"type":"node_finished","nodeId":id,"status":"completed","output":raw}))?;
        outputs.insert(id, named);
        final_output = raw;
    }
    Ok(final_output)
}

fn execute_node(
    node: &crate::graph::Node,
    values: &Value,
    emit: &mut impl FnMut(Value) -> Result<()>,
) -> Result<Value> {
    executable(&node.operation)?;
    core::validate(
        &node.operation,
        node.operation_version,
        values,
        &node.parameters,
        budget(),
    )
    .map_err(failure)?;
    emit(json!({"type":"node_started","nodeId":node.id}))?;
    let output = core::execute(
        &node.operation,
        node.operation_version,
        values,
        &node.parameters,
        budget(),
    )
    .map_err(failure)?;
    raw_outputs(&node.operation, &output)
}

#[cfg(test)]
mod registry_tests {
    use super::*;
    #[test]
    fn native_catalog_discovers_all_core_pure_executors_without_an_operation_allowlist() {
        let native = catalog().unwrap();
        let installed = cube_core::operations::execution::catalog(Default::default()).unwrap();
        let ids = native
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["operation"].as_str().unwrap())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(ids, installed.iter().map(|e| e.operation).collect());
        assert!(ids.contains("text.reverse"));
        assert!(ids.contains("image.pixels.export"));
        assert!(!ids.contains("file.info.inspect"));
    }
    #[test]
    fn discovers_text_reverse_and_preserves_both_spiral_output_ports() {
        let invocation = Invocation {
            operation: "text.reverse".into(),
            operation_version: 1,
            input: "abc".into(),
            parameters: json!({}),
        };
        assert_eq!(execute(invocation).unwrap(), json!({"text":"cba"}));
        let im = cube_core::image::Image::from_bytes(
            3,
            3,
            cube_core::image::ColorModel::Gray,
            b"Gur!! lrx",
            &[],
            &[],
        )
        .unwrap();
        let inputs = json!({"input":cube_core::operations::contracts::values::Image::from(&im)});
        let output = cube_core::operations::execution::execute(
            "image.pixels.export",
            1,
            &inputs,
            &json!({"traversal":"spiral_cw"}),
            Default::default(),
        )
        .unwrap();
        let raw = json!({"ports":output});
        let named = named_outputs("image.pixels.export", &raw).unwrap();
        assert_eq!(named["data"]["base64"], STANDARD.encode(b"Gur xrl!!"));
        assert_eq!(named["output"]["traversal"], "spiral_cw");
    }
}
