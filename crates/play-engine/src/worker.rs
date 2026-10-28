use crate::{
    Result,
    protocol::{self, Invocation},
};
use serde_json::{Value, json};
use std::io::{self, Read};

pub fn validate(invocation: &Invocation) -> Result<()> {
    if invocation.input.len() > protocol::contract().input_bytes {
        return Err("input_limit");
    }
    Probe::parse(&invocation.operation)?;
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
}

impl Probe {
    fn parse(operation: &str) -> Result<Self> {
        match operation {
            "cipher.rot13" => Ok(Self::Rot13),
            "esolang.brainfuck.execute" => Ok(Self::Brainfuck),
            _ => Err("unsupported_operation"),
        }
    }

    fn execute(self, input: &str) -> Result<Value> {
        match self {
            Self::Rot13 => Ok(json!({"text": cube_core::ciphers::rot13(input)})),
            Self::Brainfuck => {
                let limits = cube_core::esolang::brainfuck::Limits {
                    max_output_bytes: protocol::contract().output_bytes,
                    ..Default::default()
                };
                cube_core::esolang::brainfuck::execute(input, &[], limits)
                    .map(|bytes| json!({"bytes": bytes}))
                    .map_err(|_| "operation_failed")
            }
        }
    }
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
                "name":name, "description":descriptor.description}),
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
