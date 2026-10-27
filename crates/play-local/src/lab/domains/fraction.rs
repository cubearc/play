//! Explicit typed Core bridge. No CLI, algorithm copies or inferred padding.
use cube_core::{
    ciphers::{fractionation, poly, transposition},
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
struct TokenInput {
    tokens: Value,
}
pub(super) fn supported(operation: &str) -> bool {
    transposition::OPERATIONS
        .iter()
        .chain(fractionation::OPERATIONS)
        .any(|d| d.id == operation)
}
pub(super) fn execute(
    operation: &str,
    inputs: Value,
    parameters: Value,
) -> Result<Value, OperationError> {
    let op = cube_core::operations::require(operation)?.id;
    let result = (|| {
        let document = serde_json::to_string(&parameters).map_err(|_| {
            OperationError::new(ErrorCode::InvalidParameter, "invalid parameter transport")
        })?;
        match op {
            "cipher.scytale.encode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: transposition::Scytale = poly::parse_parameters(&document)?;
                Ok(json!({"text":transposition::scytale(&input.text,&p,false)?}))
            }
            "cipher.scytale.decode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: transposition::Scytale = poly::parse_parameters(&document)?;
                Ok(json!({"text":transposition::scytale(&input.text,&p,true)?}))
            }
            "cipher.rail_fence.encode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: transposition::Rails = poly::parse_parameters(&document)?;
                Ok(json!({"text":transposition::rail_fence(&input.text,&p,false)?}))
            }
            "cipher.rail_fence.decode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: transposition::Rails = poly::parse_parameters(&document)?;
                Ok(json!({"text":transposition::rail_fence(&input.text,&p,true)?}))
            }
            "cipher.redefence.encode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: transposition::Redefence = poly::parse_parameters(&document)?;
                Ok(json!({"text":transposition::redefence(&input.text,&p,false)?}))
            }
            "cipher.redefence.decode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: transposition::Redefence = poly::parse_parameters(&document)?;
                Ok(json!({"text":transposition::redefence(&input.text,&p,true)?}))
            }
            "cipher.sequence_transposition.encode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: transposition::Sequence = poly::parse_parameters(&document)?;
                Ok(json!({"text":transposition::sequence(&input.text,&p,false)?}))
            }
            "cipher.sequence_transposition.decode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: transposition::Sequence = poly::parse_parameters(&document)?;
                Ok(json!({"text":transposition::sequence(&input.text,&p,true)?}))
            }
            "cipher.skip.encode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: transposition::Skip = poly::parse_parameters(&document)?;
                Ok(json!({"text":transposition::skip(&input.text,&p,false)?}))
            }
            "cipher.skip.decode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: transposition::Skip = poly::parse_parameters(&document)?;
                Ok(json!({"text":transposition::skip(&input.text,&p,true)?}))
            }
            "cipher.columnar.encode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: transposition::Columnar = poly::parse_parameters(&document)?;
                Ok(json!({"text":transposition::columnar(&input.text,&p,false)?}))
            }
            "cipher.columnar.decode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: transposition::Columnar = poly::parse_parameters(&document)?;
                Ok(json!({"text":transposition::columnar(&input.text,&p,true)?}))
            }
            "cipher.complete_columnar.encode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: transposition::CompleteColumnar = poly::parse_parameters(&document)?;
                serde_json::to_value(transposition::complete_columnar(&input.text, &p, false)?)
                    .map_err(|_| {
                        OperationError::new(
                            ErrorCode::InvalidInput,
                            "typed output serialization failed",
                        )
                    })
            }
            "cipher.complete_columnar.decode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: transposition::CompleteColumnar = poly::parse_parameters(&document)?;
                serde_json::to_value(transposition::complete_columnar(&input.text, &p, true)?)
                    .map_err(|_| {
                        OperationError::new(
                            ErrorCode::InvalidInput,
                            "typed output serialization failed",
                        )
                    })
            }
            "cipher.double_columnar.encode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: transposition::DoubleColumnar = poly::parse_parameters(&document)?;
                Ok(json!({"text":transposition::double_columnar(&input.text,&p,false)?}))
            }
            "cipher.double_columnar.decode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: transposition::DoubleColumnar = poly::parse_parameters(&document)?;
                Ok(json!({"text":transposition::double_columnar(&input.text,&p,true)?}))
            }
            "cipher.myszkowski.encode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: transposition::Myszkowski = poly::parse_parameters(&document)?;
                Ok(json!({"text":transposition::myszkowski(&input.text,&p,false)?}))
            }
            "cipher.myszkowski.decode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: transposition::Myszkowski = poly::parse_parameters(&document)?;
                Ok(json!({"text":transposition::myszkowski(&input.text,&p,true)?}))
            }
            "cipher.amsco.encode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: transposition::Amsco = poly::parse_parameters(&document)?;
                Ok(json!({"text":transposition::amsco(&input.text,&p,false)?}))
            }
            "cipher.amsco.decode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: transposition::Amsco = poly::parse_parameters(&document)?;
                Ok(json!({"text":transposition::amsco(&input.text,&p,true)?}))
            }
            "cipher.route.encode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: transposition::routes::Routes = poly::parse_parameters(&document)?;
                Ok(json!({"text":transposition::routes::route(&input.text,&p,false)?}))
            }
            "cipher.route.decode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: transposition::routes::Routes = poly::parse_parameters(&document)?;
                Ok(json!({"text":transposition::routes::route(&input.text,&p,true)?}))
            }
            "cipher.spiral.encode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: transposition::routes::Routes = poly::parse_parameters(&document)?;
                Ok(json!({"text":transposition::routes::spiral(&input.text,&p,false)?}))
            }
            "cipher.spiral.decode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: transposition::routes::Routes = poly::parse_parameters(&document)?;
                Ok(json!({"text":transposition::routes::spiral(&input.text,&p,true)?}))
            }
            "cipher.caesar_box.encode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: transposition::routes::CaesarBox = poly::parse_parameters(&document)?;
                Ok(json!({"text":transposition::routes::caesar_box(&input.text,&p,false)?}))
            }
            "cipher.caesar_box.decode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: transposition::routes::CaesarBox = poly::parse_parameters(&document)?;
                Ok(json!({"text":transposition::routes::caesar_box(&input.text,&p,true)?}))
            }
            "cipher.turning_grille.encode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: transposition::grille::TurningGrille = poly::parse_parameters(&document)?;
                Ok(json!({"text":transposition::grille::turning_grille(&input.text,&p,false)?}))
            }
            "cipher.turning_grille.decode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: transposition::grille::TurningGrille = poly::parse_parameters(&document)?;
                Ok(json!({"text":transposition::grille::turning_grille(&input.text,&p,true)?}))
            }
            "cipher.bifid.encode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: fractionation::Bifid = poly::parse_parameters(&document)?;
                Ok(json!({"text":fractionation::bifid(&input.text,&p,false)?}))
            }
            "cipher.bifid.decode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: fractionation::Bifid = poly::parse_parameters(&document)?;
                Ok(json!({"text":fractionation::bifid(&input.text,&p,true)?}))
            }
            "cipher.trifid.encode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: fractionation::Trifid = poly::parse_parameters(&document)?;
                Ok(json!({"text":fractionation::trifid(&input.text,&p,false)?}))
            }
            "cipher.trifid.decode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: fractionation::Trifid = poly::parse_parameters(&document)?;
                Ok(json!({"text":fractionation::trifid(&input.text,&p,true)?}))
            }
            "cipher.adfgx.encode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: fractionation::Adfgx = poly::parse_parameters(&document)?;
                Ok(json!({"text":fractionation::adfgx(&input.text,&p,false)?}))
            }
            "cipher.adfgx.decode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: fractionation::Adfgx = poly::parse_parameters(&document)?;
                Ok(json!({"text":fractionation::adfgx(&input.text,&p,true)?}))
            }
            "cipher.adfgvx.encode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: fractionation::Adfgvx = poly::parse_parameters(&document)?;
                Ok(json!({"text":fractionation::adfgvx(&input.text,&p,false)?}))
            }
            "cipher.adfgvx.decode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: fractionation::Adfgvx = poly::parse_parameters(&document)?;
                Ok(json!({"text":fractionation::adfgvx(&input.text,&p,true)?}))
            }
            "cipher.nihilist_substitution.encode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: fractionation::Nihilist = poly::parse_parameters(&document)?;
                Ok(json!({"tokens":fractionation::nihilist_encode(&input.text,&p)?}))
            }
            "cipher.nihilist_substitution.decode" => {
                let input: TokenInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: fractionation::Nihilist = poly::parse_parameters(&document)?;
                Ok(
                    json!({"text":fractionation::nihilist_decode(&fractionation::parse_nihilist_tokens(&input.tokens.to_string())?,&p)?}),
                )
            }
            "cipher.bazeries.encode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: fractionation::Bazeries = poly::parse_parameters(&document)?;
                Ok(json!({"text":fractionation::bazeries(&input.text,&p,false)?}))
            }
            "cipher.bazeries.decode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: fractionation::Bazeries = poly::parse_parameters(&document)?;
                Ok(json!({"text":fractionation::bazeries(&input.text,&p,true)?}))
            }
            "cipher.fractionated_morse.encode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: fractionation::morse::FractionatedMorse = poly::parse_parameters(&document)?;
                serde_json::to_value(fractionation::morse::fractionated_morse(
                    &input.text,
                    &p,
                    false,
                )?)
                .map_err(|_| {
                    OperationError::new(
                        ErrorCode::InvalidInput,
                        "typed output serialization failed",
                    )
                })
            }
            "cipher.fractionated_morse.decode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: fractionation::morse::FractionatedMorse = poly::parse_parameters(&document)?;
                serde_json::to_value(fractionation::morse::fractionated_morse(
                    &input.text,
                    &p,
                    true,
                )?)
                .map_err(|_| {
                    OperationError::new(
                        ErrorCode::InvalidInput,
                        "typed output serialization failed",
                    )
                })
            }
            "cipher.morbit.encode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: fractionation::morse::Morbit = poly::parse_parameters(&document)?;
                serde_json::to_value(fractionation::morse::morbit(&input.text, &p, false)?).map_err(
                    |_| {
                        OperationError::new(
                            ErrorCode::InvalidInput,
                            "typed output serialization failed",
                        )
                    },
                )
            }
            "cipher.morbit.decode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: fractionation::morse::Morbit = poly::parse_parameters(&document)?;
                serde_json::to_value(fractionation::morse::morbit(&input.text, &p, true)?).map_err(
                    |_| {
                        OperationError::new(
                            ErrorCode::InvalidInput,
                            "typed output serialization failed",
                        )
                    },
                )
            }
            "cipher.pollux.encode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: fractionation::morse::Pollux = poly::parse_parameters(&document)?;
                serde_json::to_value(fractionation::morse::pollux(&input.text, &p, false)?).map_err(
                    |_| {
                        OperationError::new(
                            ErrorCode::InvalidInput,
                            "typed output serialization failed",
                        )
                    },
                )
            }
            "cipher.pollux.decode" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use the exact named typed input")
                })?;
                let p: fractionation::morse::Pollux = poly::parse_parameters(&document)?;
                serde_json::to_value(fractionation::morse::pollux(&input.text, &p, true)?).map_err(
                    |_| {
                        OperationError::new(
                            ErrorCode::InvalidInput,
                            "typed output serialization failed",
                        )
                    },
                )
            }
            "cipher.cardan.extract" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use named text input")
                })?;
                let p: transposition::grille::Cardan = poly::parse_parameters(&document)?;
                Ok(json!({"text":transposition::grille::cardan_extract(&input.text,&p)?}))
            }
            "cipher.cardan.overlay" => {
                let input: TextInput = serde_json::from_value(inputs).map_err(|_| {
                    OperationError::new(ErrorCode::InvalidInput, "use named text input")
                })?;
                let p: transposition::grille::Overlay = poly::parse_parameters(&document)?;
                serde_json::to_value(transposition::grille::cardan_overlay(&input.text, &p)?)
                    .map_err(|_| {
                        OperationError::new(
                            ErrorCode::InvalidInput,
                            "typed output serialization failed",
                        )
                    })
            }
            _ => Err(OperationError::new(
                ErrorCode::UnsupportedFormat,
                "unsupported transposition/fractionation operation",
            )),
        }
    })();
    result.map_err(|e: OperationError| e.operation(op))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_48_real_core_adapters_match_independent_vectors_and_contracts() {
        let cases: Vec<Value> =
            serde_json::from_str(include_str!("fraction_vectors.json")).unwrap();
        assert_eq!(cases.len(), 48);
        for case in cases {
            let id = case["operation"].as_str().unwrap();
            assert!(supported(id));
            let contract = cube_core::operations::require(id)
                .unwrap()
                .contract()
                .unwrap();
            contract.validate_inputs(&case["inputs"]).unwrap();
            contract.validate_parameters(&case["parameters"]).unwrap();
            let result = execute(id, case["inputs"].clone(), case["parameters"].clone()).unwrap();
            assert_eq!(result, case["outputs"], "{id}");
            contract.validate_outputs(&result).unwrap();
            let mut bad = case["parameters"].clone();
            bad.as_object_mut()
                .unwrap()
                .insert("extra".into(), json!(true));
            assert!(execute(id, case["inputs"].clone(), bad).is_err());
        }
    }
    #[test]
    fn ambiguity_and_token_inputs_remain_visible_and_strict() {
        assert_eq!(
            execute(
                "cipher.skip.encode",
                json!({"text":"ABCDE"}),
                json!({"start":"0","step":"-9223372036854775808"})
            )
            .unwrap()["text"],
            "ACEBD"
        );

        let r = execute(
            "cipher.morbit.decode",
            json!({"text":"1"}),
            json!({"digits":"123456789"}),
        )
        .unwrap();
        assert!(r["text"].is_null());
        assert_eq!(r["padding_ambiguous"], true);
        assert_eq!(r["morse"], "..");
        assert!(
            execute(
                "cipher.nihilist_substitution.decode",
                json!({"text":"110"}),
                json!({"square":"ABCDEFGHIKLMNOPQRSTUVWXYZ","key":"Z"})
            )
            .is_err()
        );
        let r = execute(
            "cipher.cardan.overlay",
            json!({"text":"A😀β!"}),
            json!({"rows":2,"columns":2,"holes":[0,3],"replacements":"XY"}),
        )
        .unwrap();
        assert_eq!(r["text"], "X😀βY");
        assert_eq!(r["original_selected"], "A!");
    }
}
