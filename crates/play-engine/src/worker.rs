use crate::{
    Result,
    protocol::{self, Invocation},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::Deserialize;
use serde_json::{Value, json};
use std::io::{self, Read};
mod composition;
mod structured;

fn supported(operation: &str) -> bool {
    Probe::parse(operation).is_ok()
        || composition::supported(operation)
        || structured::supported(operation)
        || operation == "image.dimensions"
}

pub fn validate(invocation: &Invocation) -> Result<()> {
    let probe = Probe::parse(&invocation.operation)?;
    let maximum = if matches!(probe, Probe::Pixel) {
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
    if matches!(probe, Probe::Pixel) {
        let p: PixelParameters = serde_json::from_value(invocation.parameters.clone())
            .map_err(|_| "invalid_parameters")?;
        let image = image(&invocation.input)?;
        image
            .view(p.expand_palette)
            .map_err(|_| "invalid_image")?
            .pixel(p.x, p.y)
            .map_err(|_| "invalid_parameters")?;
    }
    Ok(())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PixelParameters {
    x: u32,
    y: u32,
    expand_palette: bool,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct BrainfuckParameters {
    max_instructions: u64,
    max_output_bytes: usize,
    max_memory_bytes: usize,
}
fn configured_limits(value: &Value) -> Result<cube_core::esolang::brainfuck::Limits> {
    let p: BrainfuckParameters =
        serde_json::from_value(value.clone()).map_err(|_| "invalid_parameters")?;
    let maximum = brainfuck_limits();
    if p.max_instructions == 0
        || p.max_instructions > maximum.max_instructions
        || p.max_output_bytes == 0
        || p.max_output_bytes > maximum.max_output_bytes
        || p.max_memory_bytes == 0
        || p.max_memory_bytes > maximum.max_memory_bytes
    {
        return Err("input_limit");
    }
    Ok(cube_core::esolang::brainfuck::Limits {
        max_instructions: p.max_instructions,
        max_output_bytes: p.max_output_bytes,
        max_memory_bytes: p.max_memory_bytes,
    })
}
pub fn validate_configuration(operation: &str, version: u32, parameters: &Value) -> Result<()> {
    let probe = Probe::parse(operation).ok();
    if !supported(operation) {
        return Err("unsupported_operation");
    }
    let contract = cube_core::operations::require(operation)
        .map_err(|_| "unsupported_operation")?
        .contract()
        .map_err(|_| "contract_mismatch")?;
    if contract.operation_version != version {
        return Err("contract_mismatch");
    }
    if contract.policy.private_input
        || contract.policy.private_output
        || !contract.policy.sensitive_parameters.is_empty()
    {
        return Err("retention_policy_unavailable");
    }
    contract
        .validate_parameters(parameters)
        .map_err(|_| "invalid_parameters")?;
    match probe {
        Some(Probe::Rot13) | None => {}
        Some(Probe::Pixel) => {
            let _: PixelParameters =
                serde_json::from_value(parameters.clone()).map_err(|_| "invalid_parameters")?;
        }
        Some(Probe::Brainfuck) => {
            configured_limits(parameters)?;
        }
    }
    Ok(())
}

enum Probe {
    Rot13,
    Brainfuck,
    Pixel,
}

impl Probe {
    fn parse(operation: &str) -> Result<Self> {
        match operation {
            "cipher.rot13" => Ok(Self::Rot13),
            "image.pixels.read" => Ok(Self::Pixel),
            "esolang.brainfuck.execute" => Ok(Self::Brainfuck),
            _ => Err("unsupported_operation"),
        }
    }

    fn execute(self, input: &str, parameters: &Value) -> Result<Value> {
        self.execute_with_bytes(input, &[], parameters)
    }
    fn execute_with_bytes(self, input: &str, bytes: &[u8], parameters: &Value) -> Result<Value> {
        match self {
            Self::Pixel => {
                let p: PixelParameters =
                    serde_json::from_value(parameters.clone()).map_err(|_| "invalid_parameters")?;
                image(input)?
                    .view(p.expand_palette)
                    .map_err(|_| "operation_failed")?
                    .pixel(p.x, p.y)
                    .map(|bytes| json!({"bytes":bytes}))
                    .map_err(|_| "operation_failed")
            }
            Self::Rot13 => Ok(json!({"text": cube_core::ciphers::rot13(input)})),
            Self::Brainfuck => {
                let limits = configured_limits(parameters)?;
                cube_core::esolang::brainfuck::execute(input, bytes, limits)
                    .map(|bytes| json!({"bytes": bytes}))
                    .map_err(|_| "operation_failed")
            }
        }
    }
}

fn image(input: &str) -> Result<cube_core::image::Image> {
    let data = STANDARD.decode(input).map_err(|_| "invalid_image")?;
    if data.len() > protocol::contract().image_bytes || !data.starts_with(b"\x89PNG\r\n\x1a\n") {
        return Err("invalid_image");
    }
    cube_core::image::Image::decode(&data).map_err(|_| "invalid_image")
}

fn brainfuck_limits() -> cube_core::esolang::brainfuck::Limits {
    cube_core::esolang::brainfuck::Limits {
        max_output_bytes: protocol::contract().output_bytes,
        ..Default::default()
    }
}

pub fn parameters(operation: &str) -> Result<Value> {
    if composition::supported(operation)
        || structured::supported(operation)
        || operation == "image.dimensions"
    {
        return composition::parameters(operation);
    }
    match Probe::parse(operation)? {
        Probe::Rot13 => Ok(json!({})),
        Probe::Pixel => Ok(json!({"x":0,"y":0,"expand_palette":false})),
        Probe::Brainfuck => serde_json::to_value(brainfuck_limits()).map_err(|_| "invalid_state"),
    }
}
pub fn named_inputs(invocation: &Invocation) -> Result<Value> {
    let contract = cube_core::operations::require(&invocation.operation)
        .map_err(|_| "unsupported_operation")?
        .contract()
        .map_err(|_| "contract_mismatch")?;
    let port = contract
        .inputs
        .iter()
        .find(|port| ["text", "image.samples"].contains(&port.semantic_type.as_str()))
        .ok_or("contract_mismatch")?;
    let value = if port.semantic_type == "image.samples" {
        json!(cube_core::operations::contracts::values::Image::from(
            &image(&invocation.input)?
        ))
    } else {
        json!(invocation.input)
    };
    let mut inputs = json!({port.name.clone():value});
    if invocation.operation == "esolang.brainfuck.execute" {
        inputs["input"] = json!([]);
    }
    Ok(inputs)
}
pub fn named_outputs(operation: &str, output: &Value) -> Result<Value> {
    if !supported(operation) {
        return Err("unsupported_operation");
    }
    let contract = cube_core::operations::require(operation)
        .map_err(|_| "unsupported_operation")?
        .contract()
        .map_err(|_| "contract_mismatch")?;
    let port = contract.outputs.first().ok_or("contract_mismatch")?;
    let value = match port.semantic_type.as_str() {
        "text" => {
            let text = output["text"].as_str().ok_or("invalid_response")?;
            if text.len() > protocol::contract().output_bytes {
                return Err("output_limit");
            }
            json!(text)
        }
        "bytes" => {
            let bytes = output["bytes"].as_array().ok_or("invalid_response")?;
            if bytes.len() > protocol::contract().output_bytes {
                return Err("output_limit");
            }
            let data = bytes
                .iter()
                .map(|v| {
                    v.as_u64()
                        .and_then(|v| u8::try_from(v).ok())
                        .ok_or("invalid_response")
                })
                .collect::<Result<Vec<_>>>()?;
            json!(cube_core::operations::contracts::values::Bytes { data })
        }
        "json" => output.get("value").cloned().ok_or("invalid_response")?,
        _ => return Err("contract_mismatch"),
    };
    Ok(json!({port.name.clone():value}))
}

pub fn catalog() -> Result<Value> {
    let entries = cube_core::operations::catalog()
        .into_iter()
        .filter(|descriptor| supported(descriptor.id))
        .map(|descriptor| {
            let contract = descriptor.contract().map_err(|_| "contract_mismatch")?;
            let name = descriptor
                .metadata()
                .map_or(descriptor.command, |metadata| metadata.canonical_name);
            Ok(
                json!({"operation":descriptor.id, "operation_version":contract.operation_version,
                "name":name, "description":descriptor.description, "input_type":contract.inputs.first().ok_or("contract_mismatch")?.semantic_type, "parameters":parameters(descriptor.id)?,"contract":contract}),
            )
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(json!(entries))
}

fn execute(invocation: Invocation) -> Result<Value> {
    validate(&invocation)?;
    Probe::parse(&invocation.operation)?.execute(&invocation.input, &invocation.parameters)
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
fn image_input<'a>(
    invocation: &'a PipelineInvocation,
    node: &crate::graph::Node,
) -> Result<&'a str> {
    let Some(crate::graph::Source::Input { name }) = node.inputs.get("input") else {
        return Err("invalid_connection");
    };
    match invocation.inputs.get(name) {
        Some(InputValue::Image { png }) => Ok(png),
        _ => Err("contract_mismatch"),
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
    for n in &invocation.definition.nodes {
        if n.operation == "image.pixels.read" {
            validate(&Invocation {
                operation: n.operation.clone(),
                operation_version: n.operation_version,
                input: image_input(invocation, n)?.into(),
                parameters: n.parameters.clone(),
            })?;
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
        let raw = execute_node(&invocation, node, &values, &mut emit)?;
        let named = named_outputs(&node.operation, &raw)?;
        contract
            .validate_outputs(&named)
            .map_err(|_| "invalid_response")?;
        let size = match raw.get("text").and_then(Value::as_str) {
            Some(text) => text.len(),
            None => match raw.get("value") {
                Some(value) => value.to_string().len(),
                None => raw["bytes"].as_array().ok_or("invalid_response")?.len(),
            },
        };
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
    invocation: &PipelineInvocation,
    node: &crate::graph::Node,
    values: &Value,
    emit: &mut impl FnMut(Value) -> Result<()>,
) -> Result<Value> {
    let id = node.id;
    if node.operation == "image.dimensions" {
        emit(json!({"type":"node_started","nodeId":id}))?;
        return Ok(
            json!({"value":cube_core::image::dimensions::inspect(&image(image_input(invocation,node)?)?)}),
        );
    }
    if structured::supported(&node.operation) {
        structured::validate_inputs(&node.operation, values, &node.parameters)?;
        emit(json!({"type":"node_started","nodeId":id}))?;
        return structured::execute(&node.operation, values, &node.parameters);
    }
    if composition::supported(&node.operation) {
        composition::validate_inputs(&node.operation, values, &node.parameters)?;
        emit(json!({"type":"node_started","nodeId":id}))?;
        return composition::execute(&node.operation, values, &node.parameters);
    }
    let contract = node.contract()?;
    let primary = contract
        .inputs
        .iter()
        .find(|p| ["text", "image.samples"].contains(&p.semantic_type.as_str()))
        .ok_or("contract_mismatch")?;
    let input = if primary.semantic_type == "image.samples" {
        image_input(invocation, node)?
    } else {
        values[&primary.name].as_str().ok_or("contract_mismatch")?
    };
    let single = Invocation {
        operation: node.operation.clone(),
        operation_version: node.operation_version,
        parameters: node.parameters.clone(),
        input: input.into(),
    };
    validate(&single)?;
    let bytes = if node.operation == "esolang.brainfuck.execute" {
        values["input"]
            .as_array()
            .ok_or("contract_mismatch")?
            .iter()
            .map(|v| {
                v.as_u64()
                    .and_then(|n| u8::try_from(n).ok())
                    .ok_or("contract_mismatch")
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        Vec::new()
    };
    if bytes.len() > protocol::contract().output_bytes {
        return Err("input_limit");
    }
    emit(json!({"type":"node_started","nodeId":id}))?;
    let raw = Probe::parse(&node.operation)?.execute_with_bytes(input, &bytes, &node.parameters)?;
    Ok(raw)
}
