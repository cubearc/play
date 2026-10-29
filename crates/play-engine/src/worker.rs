use crate::{
    Result,
    protocol::{self, Invocation},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use std::io::{self, Read};

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
    let descriptor = cube_core::operations::require(&invocation.operation)
        .map_err(|_| "unsupported_operation")?;
    let contract = descriptor.contract().map_err(|_| "contract_mismatch")?;
    if contract.operation_version != invocation.operation_version {
        return Err("contract_mismatch");
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

    fn execute(self, input: &str) -> Result<Value> {
        match self {
            Self::Pixel => image(input)?
                .pixel(0, 0)
                .map(|bytes| json!({"bytes":bytes}))
                .map_err(|_| "operation_failed"),
            Self::Rot13 => Ok(json!({"text": cube_core::ciphers::rot13(input)})),
            Self::Brainfuck => {
                let limits = brainfuck_limits();
                cube_core::esolang::brainfuck::execute(input, &[], limits)
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
    let value = match Probe::parse(operation)? {
        Probe::Rot13 => {
            let text = output["text"].as_str().ok_or("invalid_response")?;
            if text.len() > protocol::contract().output_bytes {
                return Err("output_limit");
            }
            json!(text)
        }
        Probe::Brainfuck | Probe::Pixel => {
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
    };
    let contract = cube_core::operations::require(operation)
        .map_err(|_| "unsupported_operation")?
        .contract()
        .map_err(|_| "contract_mismatch")?;
    let port = contract.outputs.first().ok_or("contract_mismatch")?;
    Ok(json!({port.name.clone():value}))
}

pub fn catalog() -> Result<Value> {
    let entries = cube_core::operations::catalog()
        .into_iter()
        .filter(|descriptor| Probe::parse(descriptor.id).is_ok())
        .map(|descriptor| {
            let contract = descriptor.contract().map_err(|_| "contract_mismatch")?;
            let name = descriptor
                .metadata()
                .map_or(descriptor.command, |metadata| metadata.canonical_name);
            Ok(
                json!({"operation":descriptor.id, "operation_version":contract.operation_version,
                "name":name, "description":descriptor.description, "input_type":contract.inputs.iter().find(|port| ["text", "image.samples"].contains(&port.semantic_type.as_str())).ok_or("contract_mismatch")?.semantic_type, "parameters":parameters(descriptor.id)?}),
            )
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(json!(entries))
}

fn execute(invocation: Invocation) -> Result<Value> {
    validate(&invocation)?;
    Probe::parse(&invocation.operation)?.execute(&invocation.input)
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
    let invocation = serde_json::from_value(value).map_err(|_| "invalid_command")?;
    let result = match execute(invocation) {
        Ok(output) => json!({"status":"completed", "output":output}),
        Err(code) => json!({"status":"failed", "code":code}),
    };
    protocol::write_frame(&mut io::stdout().lock(), &result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invokes_real_core_and_preserves_unicode() {
        let invocation = Invocation {
            operation: "cipher.rot13".into(),
            operation_version: 1,
            input: "Hello, ação!".into(),
        };
        assert_eq!(execute(invocation).unwrap(), json!({"text":"Uryyb, nçãb!"}));
        let invocation = Invocation {
            operation: "esolang.brainfuck.execute".into(),
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
