//! Bounded text report adapters. Unicode, selection and counting stay in Core.
use cube_core::{
    errors::{ErrorCode, OperationError},
    text::{self, classes, grid, statistics as stats, unicode, values},
};
use serde::Serialize;
use serde_json::{Value, json};

enum Adapter {
    Reverse,
    Normalize,
    Extract,
    Statistics,
    Factors,
    Lengths,
    Codepoints,
    Values,
    Inspect,
    Compare,
    Classes,
}
fn adapter(op: &str) -> Option<Adapter> {
    Some(match op {
        "text.reverse" => Adapter::Reverse,
        "text.normalize" => Adapter::Normalize,
        "text.extract" => Adapter::Extract,
        "text.statistics" => Adapter::Statistics,
        "text.grid.factors" => Adapter::Factors,
        "text.word.lengths" => Adapter::Lengths,
        "text.codepoints" => Adapter::Codepoints,
        "text.word.values" => Adapter::Values,
        "text.unicode.inspect" => Adapter::Inspect,
        "text.unicode.compare" => Adapter::Compare,
        "text.class.map" => Adapter::Classes,
        _ => return None,
    })
}
pub(super) fn supported(op: &str) -> bool {
    adapter(op).is_some()
}
fn invalid() -> OperationError {
    OperationError::new(
        ErrorCode::InvalidParameter,
        "use the exact typed text input and parameter fields",
    )
}
fn string<'a>(value: &'a Value, name: &str) -> Result<&'a str, OperationError> {
    value.get(name).and_then(Value::as_str).ok_or_else(invalid)
}
fn unsigned(value: &Value) -> Result<u64, OperationError> {
    value
        .as_u64()
        .or_else(|| value.as_str()?.parse().ok())
        .ok_or_else(invalid)
}
fn number(value: &Value, name: &str) -> Result<u64, OperationError> {
    unsigned(value.get(name).ok_or_else(invalid)?)
}
fn size(value: &Value, name: &str) -> Result<usize, OperationError> {
    number(value, name)?.try_into().map_err(|_| invalid())
}
fn optional_size(value: &Value, name: &str) -> Result<Option<usize>, OperationError> {
    value.get(name).map(|_| size(value, name)).transpose()
}
fn flag(value: &Value, name: &str) -> Result<bool, OperationError> {
    value.get(name).and_then(Value::as_bool).ok_or_else(invalid)
}
fn output(value: impl Serialize) -> Result<Value, OperationError> {
    serde_json::to_value(value)
        .map(|v| json!({"output":v}))
        .map_err(|_| invalid())
}
pub(super) fn execute(op: &str, inputs: Value, p: Value) -> Result<Value, OperationError> {
    let source = string(&inputs, "input")?;
    let selected = adapter(op).ok_or_else(invalid)?;
    match selected {
        Adapter::Reverse => output(text::reverse(source)?),
        Adapter::Normalize => output(text::normalize(source, string(&p, "form")?)?),
        Adapter::Extract => {
            let indices = p
                .get("indices")
                .map(|value| {
                    value
                        .as_array()
                        .ok_or_else(invalid)?
                        .iter()
                        .map(|v| unsigned(v)?.try_into().map_err(|_| invalid()))
                        .collect::<Result<Vec<usize>, OperationError>>()
                })
                .transpose()?;
            output(text::extract(
                source,
                indices.as_deref(),
                optional_size(&p, "start")?,
                optional_size(&p, "stride")?,
                optional_size(&p, "stop")?,
            )?)
        }
        Adapter::Statistics => output(stats::analyze(
            source,
            stats::Options {
                ngram_size: size(&p, "ngram_size")?,
                normalization: stats::Normalization::parse(string(&p, "normalization")?)?,
                case: stats::CaseMode::parse(string(&p, "case")?)?,
                filter: stats::TextFilter::parse(string(&p, "filter")?)?,
                max_results: size(&p, "max_results")?,
            },
        )?),
        Adapter::Factors => output(grid::factors(
            source,
            flag(&p, "drop_whitespace")?,
            size(&p, "max_candidates")?,
        )?),
        Adapter::Lengths => output(values::word_lengths(source)?),
        Adapter::Codepoints => output(values::codepoints(source, string(&p, "format")?)?),
        Adapter::Values => {
            let table = p
                .get("table")
                .map(|_| values::parse_table(string(&p, "table")?))
                .transpose()?;
            output(values::word_values(
                source,
                table.as_deref(),
                string(&p, "unmapped")?,
                flag(&p, "sums")?,
                number(&p, "max_sum")?,
            )?)
        }
        Adapter::Inspect | Adapter::Compare => {
            let options = unicode::Options {
                max_results: size(&p, "max_results")?,
                max_work: number(&p, "max_work")?,
                max_output: size(&p, "max_output")?,
            };
            if matches!(selected, Adapter::Inspect) {
                output(unicode::inspect(source, &options)?)
            } else {
                output(unicode::compare(
                    source,
                    string(&inputs, "other")?,
                    string(&p, "normalization")?,
                    &options,
                )?)
            }
        }
        Adapter::Classes => {
            let definition = p
                .get("definition")
                .map(|_| string(&p, "definition"))
                .transpose()?;
            let y = p.get("y").map(|_| string(&p, "y")).transpose()?;
            if let Some(definition) = definition {
                if y.is_some() {
                    return Err(invalid());
                }
                output(
                    classes::Definition::parse(definition)?.map(source, string(&p, "unmapped")?)?,
                )
            } else {
                output(classes::map(source, y, string(&p, "unmapped")?)?)
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use cube_core::operations;
    #[test]
    fn every_text_adapter_uses_actual_core_results_and_preserves_native_types() {
        let cases = [
            (
                "text.reverse",
                json!({"input":"Aé"}),
                json!({}),
                json!("éA"),
            ),
            (
                "text.normalize",
                json!({"input":"e\u{301}"}),
                json!({"form":"NFC"}),
                json!("é"),
            ),
            (
                "text.extract",
                json!({"input":"ABC"}),
                json!({"indices":[2,0]}),
                json!("CA"),
            ),
            (
                "text.statistics",
                json!({"input":"AAA"}),
                json!({}),
                json!(stats::analyze("AAA", stats::Options::default()).unwrap()),
            ),
            (
                "text.grid.factors",
                json!({"input":"ABCD"}),
                json!({}),
                json!(grid::factors("ABCD", false, 128).unwrap()),
            ),
            (
                "text.word.lengths",
                json!({"input":"Hi 世界"}),
                json!({}),
                json!(values::word_lengths("Hi 世界").unwrap()),
            ),
            (
                "text.codepoints",
                json!({"input":"é"}),
                json!({}),
                json!(values::codepoints("é", "decimal").unwrap()),
            ),
            (
                "text.word.values",
                json!({"input":"ABC"}),
                json!({"sums":true}),
                json!(values::word_values("ABC", None, "error", true, u64::MAX).unwrap()),
            ),
            (
                "text.unicode.inspect",
                json!({"input":"é"}),
                json!({}),
                json!(unicode::inspect("é", &unicode::Options::default()).unwrap()),
            ),
            (
                "text.unicode.compare",
                json!({"input":"é","other":"e\u{301}"}),
                json!({"normalization":"NFC"}),
                json!(
                    unicode::compare("é", "e\u{301}", "NFC", &unicode::Options::default()).unwrap()
                ),
            ),
            (
                "text.class.map",
                json!({"input":"AEIOU"}),
                json!({}),
                json!(classes::map("AEIOU", None, "error").unwrap()),
            ),
        ];
        for (op, inputs, parameters, expected) in cases {
            assert!(supported(op));
            let worker=serde_json::from_value(json!({"operation":op,"operation_version":1,"inputs":inputs,"parameters":parameters})).unwrap();
            let result = super::super::super::typed::run(worker).unwrap();
            let output: Value = serde_json::from_slice(
                &result
                    .data
                    .bytes(super::super::super::types::MAX_OUTPUT)
                    .unwrap(),
            )
            .unwrap();
            assert_eq!(output["output"], expected, "{op}");
            operations::describe(op)
                .unwrap()
                .contract()
                .unwrap()
                .validate_outputs(&output)
                .unwrap();
        }
        assert!(!supported("text.missing"));
    }
    #[test]
    fn invalid_parameters_keep_selected_operation_context_and_never_coerce_input_types() {
        for (operation, inputs, parameters, section) in [
            (
                "text.normalize",
                json!({"input":"é"}),
                json!({"form":"invalid"}),
                "parameters",
            ),
            (
                "text.unicode.compare",
                json!({"input":"é","other":42}),
                json!({}),
                "inputs",
            ),
        ] {
            let worker=serde_json::from_value(json!({"operation":operation,"operation_version":1,"inputs":inputs,"parameters":parameters})).unwrap();
            let error = super::super::super::typed::run(worker).unwrap_err();
            assert_eq!(error.operation.as_deref(), Some(operation));
            assert_eq!(error.parameter.as_deref(), Some(section));
            assert_eq!(
                error.code,
                if section == "parameters" {
                    "invalid_parameter"
                } else {
                    "invalid_input"
                }
            );
        }
        let inputs = json!({"input":"ABC"});
        assert!(execute("text.extract", inputs, json!({"stride":0})).is_err());
    }
}
