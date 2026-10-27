//! Exact numeric transport; arithmetic and parsing algorithms remain in Core.
use cube_core::{
    errors::{ErrorCode, OperationError},
    numbers::{self, combinatorics as counts, constants, modular},
};
use serde_json::{Value, json};
fn invalid() -> OperationError {
    OperationError::new(
        ErrorCode::InvalidInput,
        "use exact numeric ports and parameters",
    )
}
pub(super) fn supported(op: &str) -> bool {
    numbers::OPERATIONS
        .iter()
        .chain(modular::OPERATIONS)
        .chain(counts::OPERATIONS)
        .chain(constants::OPERATIONS)
        .chain(numbers::alphabet::OPERATIONS)
        .chain(numbers::digital::OPERATIONS)
        .any(|d| d.id == op)
}
fn number(value: &Value, name: &str) -> Result<u64, OperationError> {
    let value = value.get(name).ok_or_else(invalid)?;
    value
        .as_u64()
        .or_else(|| value.as_str()?.parse().ok())
        .ok_or_else(invalid)
}
fn text<'a>(value: &'a Value, name: &str) -> Result<&'a str, OperationError> {
    value.get(name).and_then(Value::as_str).ok_or_else(invalid)
}
pub(super) fn execute(op: &str, inputs: Value, p: Value) -> Result<Value, OperationError> {
    let a = |name| number(&inputs, name);
    let n = |name| number(&p, name);
    let t = |name| text(&p, name);
    let limits = || -> Result<counts::Limits, OperationError> {
        Ok(counts::Limits {
            max_work: n("max_work")?,
            max_report_bytes: usize::try_from(n("max_report_bytes")?).map_err(|_| invalid())?,
        })
    };
    let value = match op {
        "number.gcd" => json!(modular::gcd(a("a")?, a("b")?)?),
        "number.extended_gcd" => json!(modular::extended_gcd(a("a")?, a("b")?)?),
        "number.mod_inverse" => json!(modular::mod_inverse(a("a")?, a("modulus")?)?),
        "number.mod_pow" => json!(modular::mod_pow(a("base")?, a("exponent")?, a("modulus")?)?),
        "number.binomial" => json!(counts::binomial(a("n")?, a("k")?, limits()?)?),
        "number.permutations" => json!(counts::permutations(a("n")?, a("k")?, limits()?)?),
        "number.multinomial" => {
            let groups = inputs["groups"]
                .as_array()
                .ok_or_else(invalid)?
                .iter()
                .map(|v| number(&json!({"value":v}), "value"))
                .collect::<Result<Vec<_>, _>>()?;
            json!(counts::multinomial(&groups, limits()?)?)
        }
        "number.pascal" => json!(counts::pascal(
            a("rows")?,
            n("max_work")?,
            usize::try_from(n("max_output_bytes")?).map_err(|_| invalid())?
        )?),
        "number.base.convert" => json!(numbers::convert_base(
            text(&inputs, "input")?,
            n("from_base")?.try_into().map_err(|_| invalid())?,
            n("to_base")?.try_into().map_err(|_| invalid())?
        )?),
        "number.alphabet.convert" => json!(numbers::alphabet::convert(
            text(&inputs, "input")?,
            t("source_alphabet")?,
            t("target_alphabet")?,
            n("from_base")?.try_into().map_err(|_| invalid())?,
            n("to_base")?.try_into().map_err(|_| invalid())?
        )?),
        "number.digital_root" => json!(numbers::digital::from_decimal(
            text(&inputs, "input")?,
            n("radix")?.try_into().map_err(|_| invalid())?
        )?),
        "number.constant_digits" => {
            let indices = p
                .get("indices")
                .map(|_| constants::parse_indices(t("indices")?))
                .transpose()?;
            let optional = |name| {
                p.get(name)
                    .map(|_| usize::try_from(n(name)?).map_err(|_| invalid()))
                    .transpose()
            };
            json!(constants::digits(
                constants::Constant::parse(t("constant")?)?,
                constants::DigitOptions {
                    start: optional("start")?,
                    count: optional("count")?,
                    indices: indices.as_deref(),
                    max_digits: n("max_digits")?.try_into().map_err(|_| invalid())?
                }
            )?)
        }
        _ => return Err(invalid()),
    };
    Ok(json!({"output":value}))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_numeric_adapter_uses_real_core_and_conforms_without_integer_rounding() {
        let cases = [
            (
                "number.gcd",
                json!({"a":"18446744073709551615","b":3}),
                json!({}),
                json!(3),
            ),
            (
                "number.extended_gcd",
                json!({"a":12,"b":8}),
                json!({}),
                json!(modular::extended_gcd(12, 8).unwrap()),
            ),
            (
                "number.mod_inverse",
                json!({"a":3,"modulus":11}),
                json!({}),
                json!(4),
            ),
            (
                "number.mod_pow",
                json!({"base":2,"exponent":10,"modulus":1000}),
                json!({}),
                json!(24),
            ),
            (
                "number.binomial",
                json!({"n":5,"k":2}),
                json!({"max_work":65536,"max_report_bytes":4096}),
                json!(counts::binomial(5, 2, counts::Limits::default()).unwrap()),
            ),
            (
                "number.permutations",
                json!({"n":5,"k":2}),
                json!({"max_work":65536,"max_report_bytes":4096}),
                json!(counts::permutations(5, 2, counts::Limits::default()).unwrap()),
            ),
            (
                "number.multinomial",
                json!({"groups":[2,1]}),
                json!({"max_work":65536,"max_report_bytes":4096}),
                json!(counts::multinomial(&[2, 1], counts::Limits::default()).unwrap()),
            ),
            (
                "number.pascal",
                json!({"rows":3}),
                json!({"max_work":65536,"max_output_bytes":1048576}),
                json!(counts::pascal(3, 65536, 1048576).unwrap()),
            ),
            (
                "number.base.convert",
                json!({"input":"255"}),
                json!({"from_base":10,"to_base":16}),
                json!("FF"),
            ),
            (
                "number.alphabet.convert",
                json!({"input":"10"}),
                json!({"from_base":2,"to_base":2,"source_alphabet":"01","target_alphabet":"ab"}),
                json!("ba"),
            ),
            (
                "number.digital_root",
                json!({"input":"-999"}),
                json!({"radix":10}),
                json!(9),
            ),
            (
                "number.constant_digits",
                json!({}),
                json!({"constant":"pi","max_digits":16,"start":0,"count":3}),
                json!(
                    constants::digits(
                        constants::Constant::Pi,
                        constants::DigitOptions {
                            start: Some(0),
                            count: Some(3),
                            indices: None,
                            max_digits: 16
                        }
                    )
                    .unwrap()
                ),
            ),
        ];
        for (op, inputs, parameters, expected) in cases {
            assert!(supported(op));
            let output = execute(op, inputs.clone(), parameters.clone()).unwrap();
            assert_eq!(output["output"], expected, "{op}");
            cube_core::operations::describe(op)
                .unwrap()
                .contract()
                .unwrap()
                .validate_fixture(&cube_core::operations::contracts::Fixture {
                    inputs,
                    parameters,
                    outputs: output,
                })
                .unwrap();
        }
    }
}
