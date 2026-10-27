//! Explicit typed cipher bridge. Tables, transformations and validation stay in Core.
use cube_core::{
    ciphers::mono::{
        self,
        checkerboard::{self, Profile},
        grandpre, homophonic, params, rotations,
    },
    errors::{ErrorCode, OperationError},
};
use serde::Deserialize;
use serde_json::{Value, json};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TextInput {
    text: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SymbolInput {
    tokens: Vec<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CoordinateInput {
    tokens: Vec<grandpre::Token>,
}
struct Budget {
    remaining: usize,
}
impl std::io::Write for Budget {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.remaining {
            return Err(std::io::Error::other("typed cipher wire budget exceeded"));
        }
        self.remaining -= bytes.len();
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
fn bounded(value: &Value, maximum: usize, field: &'static str) -> Result<(), OperationError> {
    serde_json::to_writer(&mut Budget { remaining: maximum }, value).map_err(|_| {
        OperationError::new(
            ErrorCode::ResourceLimit,
            "typed cipher wire value exceeds its byte budget",
        )
        .parameter(field)
    })
}
fn typed<T: serde::de::DeserializeOwned>(
    value: Value,
    field: &'static str,
) -> Result<T, OperationError> {
    serde_json::from_value(value).map_err(|_| {
        OperationError::new(
            if field == "inputs" {
                ErrorCode::InvalidInput
            } else {
                ErrorCode::InvalidParameter
            },
            "use the exact typed cipher input/parameter fields",
        )
        .parameter(field)
    })
}
/// The common worker validates schemas and authorizes capabilities before this
/// explicit bridge. No path, process, CLI, network or implicit conversion exists.
pub(super) fn execute(
    operation: &str,
    inputs: Value,
    parameters: Value,
) -> Result<Value, OperationError> {
    let op = cube_core::operations::describe(operation)
        .map(|d| d.id)
        .ok_or_else(|| {
            OperationError::new(
                ErrorCode::UnsupportedFormat,
                "no typed monoalphabetic adapter for this operation",
            )
        })?;
    let result = (|| {
        // Raw Core caps remain authoritative. These separate JSON-wire ceilings
        // include worst-case escaping and bound deserialization before allocation.
        bounded(&inputs, 8 * mono::MAX_INPUT, "inputs")?;
        bounded(&parameters, 8 * mono::MAX_DEFINITION, "parameters")?;
        match op {
            "cipher.substitution.encode" | "cipher.substitution.decode" => {
                let input: TextInput = typed(inputs, "inputs")?;
                let parameters: params::Substitution = typed(parameters, "parameters")?;
                let map = parameters.mapping()?;
                let text = if op == "cipher.substitution.encode" {
                    map.encode(&input.text, parameters.unmapped())?
                } else {
                    map.decode(&input.text, parameters.unmapped())?
                };
                Ok(json!({"text":text}))
            }
            "cipher.rot5" | "cipher.rot18" | "cipher.rot47" => {
                let input: TextInput = typed(inputs, "inputs")?;
                let _: params::Empty = typed(parameters, "parameters")?;
                let text = match op {
                    "cipher.rot5" => rotations::rot5(&input.text)?,
                    "cipher.rot18" => rotations::rot18(&input.text)?,
                    _ => rotations::rot47(&input.text)?,
                };
                Ok(json!({"text":text}))
            }
            "cipher.homophonic.encode" => {
                let input: TextInput = typed(inputs, "inputs")?;
                let parameters: params::HomophonicEncode = typed(parameters, "parameters")?;
                let table = homophonic::Mapping::new(parameters.definition)?;
                Ok(json!({"tokens":table.encode(&input.text,&parameters.counters)?}))
            }
            "cipher.homophonic.decode" => {
                let input: SymbolInput = typed(inputs, "inputs")?;
                let parameters: params::Definition<homophonic::Definition> =
                    typed(parameters, "parameters")?;
                let table = homophonic::Mapping::new(parameters.definition)?;
                Ok(json!({"text":table.decode(&input.tokens)?}))
            }
            "cipher.straddling.encode"
            | "cipher.straddling.decode"
            | "cipher.monome_dinome.encode"
            | "cipher.monome_dinome.decode"
            | "cipher.tridigital.encode"
            | "cipher.tridigital.decode" => {
                let input: TextInput = typed(inputs, "inputs")?;
                let parameters: params::Definition<checkerboard::Definition> =
                    typed(parameters, "parameters")?;
                let profile = match op {
                    "cipher.straddling.encode" | "cipher.straddling.decode" => Profile::Straddling,
                    "cipher.monome_dinome.encode" | "cipher.monome_dinome.decode" => {
                        Profile::MonomeDinome
                    }
                    _ => Profile::Tridigital,
                };
                let table = checkerboard::Table::new(profile, parameters.definition)?;
                if op == "cipher.tridigital.decode" {
                    Ok(json!({"alternatives":table.alternatives(&input.text)?}))
                } else if op.ends_with(".decode") {
                    Ok(json!({"text":table.decode(&input.text)?}))
                } else {
                    Ok(json!({"text":table.encode(&input.text)?}))
                }
            }
            "cipher.grandpre.encode" => {
                let input: TextInput = typed(inputs, "inputs")?;
                let parameters: params::Definition<grandpre::Definition> =
                    typed(parameters, "parameters")?;
                let square = grandpre::Square::new(parameters.definition)?;
                Ok(json!({"tokens":square.encode(&input.text)?}))
            }
            "cipher.grandpre.decode" => {
                let input: CoordinateInput = typed(inputs, "inputs")?;
                let parameters: params::Definition<grandpre::Definition> =
                    typed(parameters, "parameters")?;
                let square = grandpre::Square::new(parameters.definition)?;
                Ok(json!({"text":square.decode(&input.tokens)?}))
            }
            _ => Err(OperationError::new(
                ErrorCode::UnsupportedFormat,
                "no typed monoalphabetic adapter for this operation",
            )),
        }
    })();
    result.map_err(|e| e.operation(op))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn hom() -> Value {
        json!({"ascii_uppercase":true,"entries":[{"symbol":"A","tokens":["1","2"]},{"symbol":"B","tokens":["3"]}]})
    }
    fn straddling() -> Value {
        json!({"digits":"0123456789","prefixes":"37","alphabet":"ABC"})
    }
    fn monome() -> Value {
        json!({"digits":"6318927054","prefixes":"63","alphabet":"NOTARIESBCDFGHKLMPQUVWXY","aliases":[{"from":"J","to":"I"},{"from":"Z","to":"Y"}]})
    }
    fn tridigital() -> Value {
        json!({"digits":"6703528149","prefixes":"","alphabet":"DRAGONFLYBCEHIJKMPQSTUVWXZ"})
    }
    fn grand() -> Value {
        json!({"rows":vec!["AAAAAAAA";8],"row_labels":"12345678","column_labels":"12345678"})
    }
    #[test]
    fn all_fifteen_adapters_match_real_known_answers_and_their_core_contracts() {
        let cases = [
            (
                "cipher.substitution.encode",
                json!({"text":"ABC"}),
                json!({"source":"ABC","target":"CAB"}),
                json!({"text":"CAB"}),
            ),
            (
                "cipher.substitution.decode",
                json!({"text":"CAB"}),
                json!({"source":"ABC","target":"CAB"}),
                json!({"text":"ABC"}),
            ),
            (
                "cipher.rot5",
                json!({"text":"A1"}),
                json!({}),
                json!({"text":"A6"}),
            ),
            (
                "cipher.rot18",
                json!({"text":"A1"}),
                json!({}),
                json!({"text":"N6"}),
            ),
            (
                "cipher.rot47",
                json!({"text":"!"}),
                json!({}),
                json!({"text":"P"}),
            ),
            (
                "cipher.homophonic.encode",
                json!({"text":"AABA"}),
                json!({"definition":hom()}),
                json!({"tokens":["1","2","3","1"]}),
            ),
            (
                "cipher.homophonic.decode",
                json!({"tokens":["1","2"]}),
                json!({"definition":hom()}),
                json!({"text":"AA"}),
            ),
            (
                "cipher.straddling.encode",
                json!({"text":"AB"}),
                json!({"definition":straddling()}),
                json!({"text":"01"}),
            ),
            (
                "cipher.straddling.decode",
                json!({"text":"01"}),
                json!({"definition":straddling()}),
                json!({"text":"AB"}),
            ),
            (
                "cipher.monome_dinome.encode",
                json!({"text":"HIGHF"}),
                json!({"definition":monome()}),
                json!({"text":"600676062"}),
            ),
            (
                "cipher.monome_dinome.decode",
                json!({"text":"600676062"}),
                json!({"definition":monome()}),
                json!({"text":"HIGHF"}),
            ),
            (
                "cipher.tridigital.encode",
                json!({"text":"THE"}),
                json!({"definition":tridigital()}),
                json!({"text":"030"}),
            ),
            (
                "cipher.tridigital.decode",
                json!({"text":"0"}),
                json!({"definition":tridigital()}),
                json!({"alternatives":[["A","E","T"]]}),
            ),
            (
                "cipher.grandpre.encode",
                json!({"text":"A A"}),
                json!({"definition":grand()}),
                json!({"tokens":[[1,1],null,[1,2]]}),
            ),
            (
                "cipher.grandpre.decode",
                json!({"tokens":[[1,1],null]}),
                json!({"definition":grand()}),
                json!({"text":"A "}),
            ),
        ];
        let mut covered = std::collections::BTreeSet::new();
        for (op, inputs, parameters, expected) in cases {
            covered.insert(op);
            let actual = execute(op, inputs.clone(), parameters.clone()).unwrap();
            assert_eq!(actual, expected, "{op}");
            let worker=serde_json::from_value(serde_json::json!({"operation":op,"operation_version":1,"inputs":inputs,"parameters":parameters})).unwrap();
            let output = super::super::super::typed::run(worker).unwrap();
            assert_eq!(
                serde_json::from_slice::<Value>(
                    &output
                        .data
                        .bytes(super::super::super::types::MAX_OUTPUT)
                        .unwrap()
                )
                .unwrap(),
                expected,
                "named worker {op}"
            );
            let contract = cube_core::operations::contracts::describe(
                cube_core::operations::describe(op).unwrap(),
            )
            .unwrap();
            contract
                .validate_fixture(&cube_core::operations::contracts::Fixture {
                    inputs,
                    parameters,
                    outputs: actual,
                })
                .unwrap();
        }
        assert_eq!(covered, mono::OPERATIONS.iter().map(|d| d.id).collect());
    }
    #[test]
    fn unsupported_shapes_bad_tables_and_exhausted_limits_fail_without_coercion() {
        for (op, inputs, parameters) in [
            ("cipher.rot5", json!({"text":"A","extra":true}), json!({})),
            ("cipher.rot5", json!({"text":"A"}), json!({"extra":true})),
            (
                "cipher.substitution.encode",
                json!({"text":"A"}),
                json!({"source":"AB","target":"AA"}),
            ),
            (
                "cipher.homophonic.encode",
                json!({"text":"A"}),
                json!({"definition":hom(),"counters":[{"symbol":"A","index":true}]}),
            ),
            (
                "cipher.homophonic.decode",
                json!({"tokens":["unknown"]}),
                json!({"definition":hom()}),
            ),
            (
                "cipher.grandpre.decode",
                json!({"tokens":[[true,1]]}),
                json!({"definition":grand()}),
            ),
            (
                "cipher.grandpre.decode",
                json!({"tokens":[[0,0]]}),
                json!({"definition":grand()}),
            ),
            (
                "cipher.monome_dinome.decode",
                json!({"text":"6"}),
                json!({"definition":monome()}),
            ),
        ] {
            let e = execute(op, inputs, parameters).unwrap_err();
            assert_eq!(e.operation, Some(op));
        }
        assert_eq!(
            execute(
                "cipher.rot5",
                json!({"text":"A".repeat(mono::MAX_INPUT+1)}),
                json!({})
            )
            .unwrap_err()
            .code,
            ErrorCode::ResourceLimit
        );
        assert!(execute("cipher.missing", json!({"text":"A"}), json!({})).is_err());
        assert!(
            execute(
                "cipher.caesar.encode",
                json!({"text":"A"}),
                json!({"shift":1})
            )
            .is_err()
        );
    }
}
