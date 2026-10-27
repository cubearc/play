//! Adapt existing byte/text Core adapters to named ports without a second engine.
use cube_core::{
    errors::{ErrorCode, OperationError},
    operations::{self, ParameterType},
    pipeline,
};
use serde_json::{Value, json};

pub(super) fn supported(operation: &str) -> bool {
    operation != "text.transform.masked"
        && pipeline::adapter_version(operation).is_some()
        && operations::describe(operation).is_some_and(|d| {
            matches!(d.input_type, "bytes" | "text") && matches!(d.output_type, "bytes" | "text")
        })
}
fn invalid() -> OperationError {
    OperationError::new(
        ErrorCode::InvalidInput,
        "use exact typed pipeline ports and parameters",
    )
}
pub(super) fn execute(
    operation: &str,
    inputs: Value,
    parameters: Value,
) -> Result<Value, OperationError> {
    // The direct Core contract accepts full-width u64 counts; pipeline integers
    // have a smaller signed range. Preserve the declared direct contract here.
    if matches!(operation, "binary.rotate_left" | "binary.rotate_right") {
        let data = super::bytes(&inputs["input"])?;
        let count = parameters["count"]
            .as_u64()
            .or_else(|| parameters["count"].as_str()?.parse().ok())
            .ok_or_else(invalid)?;
        let unit = parameters["unit_bytes"].as_u64().ok_or_else(invalid)?;
        let unit = usize::try_from(unit).map_err(|_| invalid())?;
        let output = if operation == "binary.rotate_left" {
            cube_core::binary::transform::rotate_left(&data, count, unit)?
        } else {
            cube_core::binary::transform::rotate_right(&data, count, unit)?
        };
        return Ok(json!({"output":{"base64":cube_core::encoding::base64_encode(&output)}}));
    }
    let descriptor = operations::describe(operation).ok_or_else(invalid)?;
    let input = match descriptor.input_type {
        "bytes" => pipeline::Value::Bytes {
            data: super::bytes(&inputs["input"])?,
        },
        "text" => pipeline::Value::Text {
            data: inputs["input"].as_str().ok_or_else(invalid)?.into(),
        },
        _ => return Err(invalid()),
    };
    let mut bindings = std::collections::BTreeMap::new();
    for (name, value) in parameters.as_object().ok_or_else(invalid)? {
        let parameter = descriptor
            .parameters
            .iter()
            .find(|p| p.name == name)
            .ok_or_else(invalid)?;
        let binding = match parameter.constraints {
            ParameterType::Bytes { .. } => pipeline::Parameter::Bytes {
                data: serde_json::from_value(value.clone()).map_err(|_| invalid())?,
            },
            ParameterType::UnsignedList { .. } => pipeline::Parameter::UnsignedList {
                values: serde_json::from_value(value.clone()).map_err(|_| invalid())?,
            },
            ParameterType::TextList { .. } => pipeline::Parameter::TextList {
                values: serde_json::from_value(value.clone()).map_err(|_| invalid())?,
            },
            _ => serde_json::from_value(value.clone()).map_err(|_| invalid())?,
        };
        bindings.insert(name.clone(), binding);
    }
    let step = pipeline::Step {
        operation: operation.into(),
        version: pipeline::adapter_version(operation).ok_or_else(invalid)?,
        parameters: bindings,
    };
    let limits = pipeline::Limits {
        max_steps: 1,
        max_value_bytes: pipeline::MAX_VALUE_BYTES,
        max_work_bytes: pipeline::MAX_WORK_BYTES,
        max_capture_bytes: 0,
    };
    let output = pipeline::Plan::validate(input.kind(), vec![step], limits)
        .and_then(|plan| plan.execute(input, &[], None))
        .map_err(|e| *e.error)?
        .output;
    match output {
        pipeline::Value::Bytes { data } => {
            Ok(json!({"output":{"base64":cube_core::encoding::base64_encode(&data)}}))
        }
        pipeline::Value::Text { data } => Ok(json!({"output":data})),
        _ => Err(invalid()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn existing_real_adapters_preserve_binary_and_parameter_types() {
        for (op, input, params, expected) in [
            (
                "encoding.base64.encode",
                json!({"input":{"base64":"AP8="}}),
                json!({}),
                json!({"output":"AP8="}),
            ),
            (
                "encoding.base64.decode",
                json!({"input":"AP8="}),
                json!({}),
                json!({"output":{"base64":"AP8="}}),
            ),
            (
                "cipher.caesar.encode",
                json!({"input":"ABC"}),
                json!({"shift":3}),
                json!({"output":"DEF"}),
            ),
            (
                "binary.xor",
                json!({"input":{"base64":"AP8="}}),
                json!({"key":[255]}),
                json!({"output":{"base64":"/wA="}}),
            ),
        ] {
            assert!(supported(op));
            let output = execute(op, input.clone(), params.clone()).unwrap();
            assert_eq!(output, expected);
            operations::describe(op)
                .unwrap()
                .contract()
                .unwrap()
                .validate_fixture(&operations::contracts::Fixture {
                    inputs: input,
                    parameters: params,
                    outputs: output,
                })
                .unwrap();
        }
        assert!(!supported("file.strings.extract"));
        assert!(!supported("text.transform.masked"));
    }
}
