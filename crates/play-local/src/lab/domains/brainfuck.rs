//! An explicit bounded Core interpreter, with separate source and program bytes.
use super::{field, images::bytes};
use cube_core::{
    errors::{ErrorCode, OperationError},
    esolang::brainfuck as bf,
};
use serde_json::{Value, json};
enum Adapter {
    Execute,
    Report,
    Compare,
}
fn adapter(op: &str) -> Option<Adapter> {
    Some(match op {
        "esolang.brainfuck.execute" => Adapter::Execute,
        "esolang.brainfuck.report" => Adapter::Report,
        "esolang.brainfuck.compare" => Adapter::Compare,
        _ => return None,
    })
}
pub(super) fn supported(op: &str) -> bool {
    adapter(op).is_some()
}
fn invalid() -> OperationError {
    OperationError::new(
        ErrorCode::InvalidInput,
        "use explicit Brainfuck source, byte input and execution budgets",
    )
}
pub(super) fn execute(op: &str, inputs: Value, p: Value) -> Result<Value, OperationError> {
    let identity = cube_core::operations::require(op)?.id;
    let selected = adapter(op).ok_or_else(invalid)?;
    let result = (|| {
        let source: String = field(&inputs, "source")?;
        let input: Vec<u8> = field(&inputs, "input")?;
        let eof = bf::EofPolicy::parse(p["eof"].as_str().ok_or_else(invalid)?)?;
        let limits = bf::Limits {
            max_instructions: field(&p, "max_instructions")?,
            max_output_bytes: field(&p, "max_output_bytes")?,
            max_memory_bytes: field(&p, "max_memory_bytes")?,
        };
        if matches!(selected, Adapter::Compare) {
            return Ok(
                json!({"output":bf::compare(&source,&input,&field::<Vec<u8>>(&p,"widths")?,eof,limits,bf::ComparisonLimits {max_instructions:field(&p,"total_instructions")?,max_output_bytes:field(&p,"total_output_bytes")?,max_memory_bytes:field(&p,"total_memory_bytes")?})?}),
            );
        }
        let program = bf::Program::parse(&source)?;
        let options = bf::Options {
            cell_bits: field(&p, "cell_bits")?,
            eof,
        };
        Ok(match selected {
            Adapter::Execute => json!({"output":bytes(&program.execute(&input,options,limits)?)}),
            Adapter::Report => {
                json!({"output":program.report(&input,options,limits,bf::Diagnostics {tape_start:field(&p,"tape_start")?,tape_len:field(&p,"tape_len")?,max_trace_records:field(&p,"max_trace_records")?,max_trace_bytes:field(&p,"max_trace_bytes")?,max_report_bytes:field(&p,"max_report_bytes")?})?})
            }
            _ => return Err(invalid()),
        })
    })();
    result.map_err(|e: OperationError| e.operation(identity))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn run(op: &str, inputs: Value, mut p: Value) -> Result<Value, OperationError> {
        let c = cube_core::operations::require(op)?.contract().unwrap();
        assert_eq!(c.operation_version, 2);
        for (k, s) in c.parameters.as_value()["properties"].as_object().unwrap() {
            if let Some(d) = s.get("default") {
                p.as_object_mut()
                    .unwrap()
                    .entry(k)
                    .or_insert_with(|| d.clone());
            }
        }
        c.validate_inputs(&inputs).unwrap();
        c.validate_parameters(&p).unwrap();
        let result = execute(op, inputs, p)?;
        c.validate_outputs(&result).unwrap();
        Ok(result)
    }
    #[test]
    fn real_interpreter_preserves_binary_outputs_and_explicit_incomplete_reports() {
        let input = json!({"source":",.","input":[255]});
        assert_eq!(
            run("esolang.brainfuck.execute", input.clone(), json!({})).unwrap()["output"],
            bytes(&[255])
        );
        let report = run(
            "esolang.brainfuck.report",
            input.clone(),
            json!({"tape_len":1}),
        )
        .unwrap();
        assert_eq!(report["output"]["output"], json!([255]));
        assert_eq!(report["output"]["completed"], true);
        let compared = run(
            "esolang.brainfuck.compare",
            input.clone(),
            json!({"widths":[8,16,32]}),
        )
        .unwrap();
        assert_eq!(compared["output"]["equal_completed_outputs"], true);
        assert!(
            run(
                "esolang.brainfuck.execute",
                input,
                json!({"max_instructions":0})
            )
            .is_err()
        );
        let stopped = run(
            "esolang.brainfuck.report",
            json!({"source":"+[]","input":[]}),
            json!({"max_instructions":8}),
        )
        .unwrap();
        assert_eq!(stopped["output"]["completed"], false);
        assert_eq!(stopped["output"]["stop_reason"], "instruction_limit");
        assert!(
            run(
                "esolang.brainfuck.execute",
                json!({"source":"[","input":[]}),
                json!({})
            )
            .is_err()
        );
    }
}
