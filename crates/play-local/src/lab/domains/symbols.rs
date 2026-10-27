//! Named-port adapters for existing Core symbol representations and scalar squares.
use super::{bytes, field};
use cube_core::{
    ciphers::{bacon, polybius},
    errors::{ErrorCode, OperationError},
    symbols::{self, clock, keypad},
};
use serde::Serialize;
use serde_json::{Value, json};

#[derive(Clone, Copy)]
enum Adapter {
    MorseEncode,
    MorseDecode,
    BrailleEncode,
    BrailleDecode,
    BaconEncode,
    BaconDecode,
    PolybiusEncode,
    PolybiusDecode,
    KeypadEncode,
    KeypadDecode,
    Clock,
}
fn adapter(op: &str) -> Option<Adapter> {
    Some(match op {
        "symbol.morse.encode" => Adapter::MorseEncode,
        "symbol.morse.decode" => Adapter::MorseDecode,
        "symbol.braille.encode" => Adapter::BrailleEncode,
        "symbol.braille.decode" => Adapter::BrailleDecode,
        "cipher.bacon.encode" => Adapter::BaconEncode,
        "cipher.bacon.decode" => Adapter::BaconDecode,
        "cipher.polybius.encode" => Adapter::PolybiusEncode,
        "cipher.polybius.decode" => Adapter::PolybiusDecode,
        "symbol.keypad.encode" => Adapter::KeypadEncode,
        "symbol.keypad.decode" => Adapter::KeypadDecode,
        "symbol.clock.directions" => Adapter::Clock,
        _ => return None,
    })
}
pub(super) fn supported(op: &str) -> bool {
    adapter(op).is_some()
}
fn invalid() -> OperationError {
    OperationError::new(
        ErrorCode::InvalidParameter,
        "use the exact symbol input, explicit convention and compatible parameter fields",
    )
}
fn string<'a>(v: &'a Value, name: &str) -> Result<&'a str, OperationError> {
    v.get(name).and_then(Value::as_str).ok_or_else(invalid)
}
fn optional_integer(v: &Value, name: &str) -> Result<Option<i64>, OperationError> {
    v.get(name)
        .map(|n| {
            n.as_i64()
                .or_else(|| n.as_str()?.parse().ok())
                .ok_or_else(invalid)
        })
        .transpose()
}
fn square(p: &Value) -> Result<polybius::Square, OperationError> {
    if let Some(alphabet) = p.get("alphabet") {
        let aliases: Vec<String> = p
            .get("aliases")
            .map(|_| field(p, "aliases"))
            .transpose()?
            .unwrap_or_default();
        let aliases = aliases
            .iter()
            .map(|pair| {
                let mut chars = pair.chars();
                match (chars.next(), chars.next(), chars.next(), chars.next()) {
                    (Some(from), Some('='), Some(to), None) => Ok((from, to)),
                    _ => Err(invalid().parameter("aliases")),
                }
            })
            .collect::<Result<Vec<_>, _>>()?;
        polybius::Square::new(
            field(p, "side")?,
            alphabet.as_str().ok_or_else(invalid)?,
            &aliases,
        )
    } else {
        if p.get("side").is_some() || p.get("aliases").is_some() {
            return Err(invalid().parameter("alphabet"));
        }
        polybius::Square::preset(string(p, "preset")?)
    }
}
fn output(value: impl Serialize) -> Result<Value, OperationError> {
    serde_json::to_value(value)
        .map(|value| json!({"output": value}))
        .map_err(|_| invalid())
}
pub(super) fn execute(op: &str, inputs: Value, p: Value) -> Result<Value, OperationError> {
    let identity = cube_core::operations::require(op)?.id;
    let selected = adapter(op).ok_or_else(invalid)?;
    let result = (|| match selected {
        Adapter::MorseEncode => output(symbols::morse_encode(string(&inputs, "input")?)?),
        Adapter::MorseDecode => output(symbols::morse_decode(string(&inputs, "input")?)?),
        Adapter::BrailleEncode => output(symbols::braille_encode(&bytes(&inputs["input"])?)?),
        Adapter::BrailleDecode => output(
            json!({"base64":cube_core::encoding::base64_encode(&symbols::braille_decode(string(&inputs,"input")?)?)}),
        ),
        Adapter::BaconEncode => output(bacon::encode(
            string(&inputs, "input")?,
            string(&p, "variant")?,
            string(&p, "notation")?,
        )?),
        Adapter::BaconDecode => output(bacon::decode(
            string(&inputs, "input")?,
            string(&p, "variant")?,
            string(&p, "notation")?,
        )?),
        Adapter::PolybiusEncode => output(polybius::format_tokens(
            &square(&p)?.encode(string(&inputs, "input")?, field(&p, "index_base")?)?,
        )?),
        Adapter::PolybiusDecode => output(square(&p)?.decode(
            &polybius::parse_tokens(string(&inputs, "input")?)?,
            field(&p, "index_base")?,
        )?),
        Adapter::KeypadEncode | Adapter::KeypadDecode => {
            let mapping = p
                .get("mapping")
                .map(|_| keypad::Mapping::parse(string(&p, "mapping")?))
                .transpose()?
                .unwrap_or_else(keypad::Mapping::standard);
            if matches!(selected, Adapter::KeypadEncode) {
                output(mapping.encode(string(&inputs, "input")?, string(&p, "mode")?)?)
            } else {
                output(mapping.decode(string(&inputs, "input")?, string(&p, "mode")?)?)
            }
        }
        Adapter::Clock => output(clock::inspect(
            p.get("time").map(|_| string(&p, "time")).transpose()?,
            optional_integer(&p, "hour_millidegrees")?,
            optional_integer(&p, "minute_millidegrees")?,
            string(&p, "zero_axis")?,
            field(&p, "clockwise")?,
            field(&p, "decode")?,
        )?),
    })();
    result.map_err(|error: OperationError| error.operation(identity))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn eleven_symbol_adapters_preserve_exact_known_values_and_contract_outputs() {
        let cases = [
            (
                "symbol.morse.encode",
                json!({"input":"SOS 2"}),
                json!({}),
                json!("... --- ... / ..---"),
            ),
            (
                "symbol.morse.decode",
                json!({"input":"... --- ... / ..---"}),
                json!({}),
                json!("SOS 2"),
            ),
            (
                "symbol.braille.encode",
                json!({"input":{"base64":"AAH/"}}),
                json!({}),
                json!("\u{2800}\u{2801}\u{28ff}"),
            ),
            (
                "symbol.braille.decode",
                json!({"input":"\u{2800}\u{2801}\u{28ff}"}),
                json!({}),
                json!({"base64":"AAH/"}),
            ),
            (
                "cipher.bacon.encode",
                json!({"input":"ABC"}),
                json!({"variant":"extended26"}),
                json!("AAAAA AAAAB AAABA"),
            ),
            (
                "cipher.bacon.decode",
                json!({"input":"AAAAA AAAAB AAABA"}),
                json!({"variant":"extended26"}),
                json!("ABC"),
            ),
            (
                "cipher.polybius.encode",
                json!({"input":"AJ I"}),
                json!({}),
                json!("1,1 2,4 / 2,4"),
            ),
            (
                "cipher.polybius.decode",
                json!({"input":"1,1 2,4 / 2,4"}),
                json!({}),
                json!("AI I"),
            ),
            (
                "symbol.keypad.encode",
                json!({"input":"ABC"}),
                json!({"mode":"multi_tap"}),
                json!("2|22|222"),
            ),
            (
                "symbol.keypad.decode",
                json!({"input":"2|22|222"}),
                json!({"mode":"multi_tap"}),
                json!({"text":"ABC","alternatives":[],"reversible":true}),
            ),
            (
                "symbol.clock.directions",
                json!({}),
                json!({"time":"03:00"}),
                json!({"hour":{"normalized_input_millidegrees":90000,"bearing_millidegrees":90000,"directions":["E"],"ambiguous":false},"minute":{"normalized_input_millidegrees":0,"bearing_millidegrees":0,"directions":["N"],"ambiguous":false},"zero_axis":"N","clockwise":true,"tolerance_millidegrees":1,"decoded":null,"interpretation":"caller_selected_clock_convention_hour_LH_minute_RH"}),
            ),
        ];
        for (op, inputs, mut parameters, expected) in cases {
            assert!(supported(op));
            let contract = cube_core::operations::require(op)
                .unwrap()
                .contract()
                .unwrap();
            for (key, value) in contract.parameters.as_value()["properties"]
                .as_object()
                .into_iter()
                .flatten()
            {
                if let Some(value) = value.get("default") {
                    parameters
                        .as_object_mut()
                        .unwrap()
                        .entry(key)
                        .or_insert_with(|| value.clone());
                }
            }
            contract
                .validate_inputs(&inputs)
                .unwrap_or_else(|e| panic!("{op} inputs: {e}"));
            contract
                .validate_parameters(&parameters)
                .unwrap_or_else(|e| panic!("{op} parameters: {e}"));
            let actual = execute(op, inputs, parameters).unwrap();
            contract
                .validate_outputs(&actual)
                .unwrap_or_else(|e| panic!("{op} outputs: {e}"));
            assert_eq!(actual, json!({"output":expected}), "{op}");
        }
        assert!(!supported("unknown.operation"));
    }
    #[test]
    fn ambiguous_keypads_custom_aliases_and_invalid_conventions_stay_explicit() {
        assert_eq!(
            execute(
                "symbol.keypad.decode",
                json!({"input":"2"}),
                json!({"mode":"telephone"})
            )
            .unwrap()["output"],
            json!({"text":null,"alternatives":["ABC"],"reversible":false})
        );
        assert_eq!(
            execute(
                "cipher.polybius.encode",
                json!({"input":"XA"}),
                json!({"alphabet":"ABCD","side":2,"aliases":["X=A"],"index_base":0})
            )
            .unwrap()["output"],
            json!("0,0 0,0")
        );
        for (operation, inputs, parameters) in [
            (
                "symbol.morse.decode",
                json!({"input":"...  ---"}),
                json!({}),
            ),
            ("symbol.braille.decode", json!({"input":"ABC"}), json!({})),
            (
                "cipher.bacon.encode",
                json!({"input":"A"}),
                json!({"variant":"guess","notation":"ab"}),
            ),
            (
                "cipher.polybius.encode",
                json!({"input":"A"}),
                json!({"side":2,"preset":"polybius","index_base":1}),
            ),
            (
                "cipher.polybius.encode",
                json!({"input":"A"}),
                json!({"alphabet":"ABCD","side":2,"aliases":["not an alias"],"index_base":1}),
            ),
            (
                "symbol.clock.directions",
                json!({}),
                json!({"time":"03:00","hour_millidegrees":0,"zero_axis":"N","clockwise":true,"decode":false}),
            ),
            (
                "symbol.clock.directions",
                json!({}),
                json!({"hour_millidegrees":"invalid","zero_axis":"N","clockwise":true,"decode":false}),
            ),
        ] {
            assert!(
                execute(operation, inputs, parameters).is_err(),
                "{operation}"
            );
        }
    }
}
