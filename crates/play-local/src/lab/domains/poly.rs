//! Typed direct Core bridge; no CLI, external process, network or algorithm copies.
use cube_core::{
    ciphers::poly::{self, geometry, hill, schedules, stateful, tableaux},
    errors::{ErrorCode, OperationError},
};
use serde::Deserialize;
use serde_json::{Value, json};
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Input {
    text: String,
}
pub(super) fn execute(
    operation: &str,
    inputs: Value,
    parameters: Value,
) -> Result<Value, OperationError> {
    let op = cube_core::operations::require(operation)?.id;
    let result = (|| {
        let input: Input = serde_json::from_value(inputs).map_err(|_| {
            OperationError::new(ErrorCode::InvalidInput, "use the exact named text input")
        })?;
        let document = serde_json::to_string(&parameters).map_err(|_| {
            OperationError::new(ErrorCode::InvalidParameter, "parameters require valid JSON")
        })?;
        match op {
            "cipher.gronsfeld.encode" => {
                let p: schedules::Gronsfeld = poly::parse_parameters(&document)?;
                Ok(json!({"text":schedules::gronsfeld(&input.text,&p,false)?}))
            }
            "cipher.gronsfeld.decode" => {
                let p: schedules::Gronsfeld = poly::parse_parameters(&document)?;
                Ok(json!({"text":schedules::gronsfeld(&input.text,&p,true)?}))
            }
            "cipher.trithemius.encode" => {
                let p: schedules::Trithemius = poly::parse_parameters(&document)?;
                Ok(json!({"text":schedules::trithemius(&input.text,&p,false)?}))
            }
            "cipher.trithemius.decode" => {
                let p: schedules::Trithemius = poly::parse_parameters(&document)?;
                Ok(json!({"text":schedules::trithemius(&input.text,&p,true)?}))
            }
            "cipher.autokey.encode" => {
                let p: schedules::Autokey = poly::parse_parameters(&document)?;
                Ok(json!({"text":schedules::autokey(&input.text,&p,false)?}))
            }
            "cipher.autokey.decode" => {
                let p: schedules::Autokey = poly::parse_parameters(&document)?;
                Ok(json!({"text":schedules::autokey(&input.text,&p,true)?}))
            }
            "cipher.running_key.encode" => {
                let p: schedules::RunningKey = poly::parse_parameters(&document)?;
                Ok(json!({"text":schedules::running_key(&input.text,&p,false)?}))
            }
            "cipher.running_key.decode" => {
                let p: schedules::RunningKey = poly::parse_parameters(&document)?;
                Ok(json!({"text":schedules::running_key(&input.text,&p,true)?}))
            }
            "cipher.porta" => {
                let p: schedules::Key = poly::parse_parameters(&document)?;
                Ok(json!({"text":schedules::porta(&input.text,&p)?}))
            }
            "cipher.gromark.encode" => {
                let p: stateful::Gromark = poly::parse_parameters(&document)?;
                Ok(json!({"text":stateful::gromark(&input.text,&p,false)?}))
            }
            "cipher.gromark.decode" => {
                let p: stateful::Gromark = poly::parse_parameters(&document)?;
                Ok(json!({"text":stateful::gromark(&input.text,&p,true)?}))
            }
            "cipher.periodic_gromark.encode" => {
                let p: stateful::PeriodicGromark = poly::parse_parameters(&document)?;
                Ok(json!({"text":stateful::periodic_gromark(&input.text,&p,false)?}))
            }
            "cipher.periodic_gromark.decode" => {
                let p: stateful::PeriodicGromark = poly::parse_parameters(&document)?;
                Ok(json!({"text":stateful::periodic_gromark(&input.text,&p,true)?}))
            }
            "cipher.solitaire.encode" => {
                let p: stateful::Solitaire = poly::parse_parameters(&document)?;
                Ok(json!({"text":stateful::solitaire(&input.text,&p,false)?}))
            }
            "cipher.solitaire.decode" => {
                let p: stateful::Solitaire = poly::parse_parameters(&document)?;
                Ok(json!({"text":stateful::solitaire(&input.text,&p,true)?}))
            }
            "cipher.slidefair.encode" => {
                let p: schedules::Key = poly::parse_parameters(&document)?;
                Ok(json!({"text":tableaux::slidefair(&input.text,&p,false)?}))
            }
            "cipher.slidefair.decode" => {
                let p: schedules::Key = poly::parse_parameters(&document)?;
                Ok(json!({"text":tableaux::slidefair(&input.text,&p,true)?}))
            }
            "cipher.portax" => {
                let p: schedules::Key = poly::parse_parameters(&document)?;
                Ok(json!({"text":tableaux::portax(&input.text,&p)?}))
            }
            "cipher.ragbaby.encode" => {
                let p: stateful::Ragbaby = poly::parse_parameters(&document)?;
                Ok(json!({"text":stateful::ragbaby(&input.text,&p,false)?}))
            }
            "cipher.ragbaby.decode" => {
                let p: stateful::Ragbaby = poly::parse_parameters(&document)?;
                Ok(json!({"text":stateful::ragbaby(&input.text,&p,true)?}))
            }
            "cipher.condi.encode" => {
                let p: stateful::Condi = poly::parse_parameters(&document)?;
                Ok(json!({"text":stateful::condi(&input.text,&p,false)?}))
            }
            "cipher.condi.decode" => {
                let p: stateful::Condi = poly::parse_parameters(&document)?;
                Ok(json!({"text":stateful::condi(&input.text,&p,true)?}))
            }
            "cipher.nicodemus.encode" => {
                let p: tableaux::Nicodemus = poly::parse_parameters(&document)?;
                Ok(json!({"text":tableaux::nicodemus(&input.text,&p,false)?}))
            }
            "cipher.nicodemus.decode" => {
                let p: tableaux::Nicodemus = poly::parse_parameters(&document)?;
                Ok(json!({"text":tableaux::nicodemus(&input.text,&p,true)?}))
            }
            "cipher.quagmire.encode" => {
                let p: tableaux::Quagmire = poly::parse_parameters(&document)?;
                Ok(json!({"text":tableaux::quagmire(&input.text,&p,false)?}))
            }
            "cipher.quagmire.decode" => {
                let p: tableaux::Quagmire = poly::parse_parameters(&document)?;
                Ok(json!({"text":tableaux::quagmire(&input.text,&p,true)?}))
            }
            "cipher.phillips.encode" => {
                let p: tableaux::Phillips = poly::parse_parameters(&document)?;
                Ok(json!({"text":tableaux::phillips(&input.text,&p,false)?}))
            }
            "cipher.phillips.decode" => {
                let p: tableaux::Phillips = poly::parse_parameters(&document)?;
                Ok(json!({"text":tableaux::phillips(&input.text,&p,true)?}))
            }
            "cipher.chaocipher.encode" => {
                let p: stateful::Chaocipher = poly::parse_parameters(&document)?;
                Ok(json!({"text":stateful::chaocipher(&input.text,&p,false)?}))
            }
            "cipher.chaocipher.decode" => {
                let p: stateful::Chaocipher = poly::parse_parameters(&document)?;
                Ok(json!({"text":stateful::chaocipher(&input.text,&p,true)?}))
            }
            "cipher.playfair.encode" => {
                let p: geometry::Playfair = poly::parse_parameters(&document)?;
                serde_json::to_value(geometry::playfair(&input.text, &p, false)?).map_err(|_| {
                    OperationError::new(
                        ErrorCode::InvalidInput,
                        "typed output serialization failed",
                    )
                })
            }
            "cipher.playfair.decode" => {
                let p: geometry::Playfair = poly::parse_parameters(&document)?;
                serde_json::to_value(geometry::playfair(&input.text, &p, true)?).map_err(|_| {
                    OperationError::new(
                        ErrorCode::InvalidInput,
                        "typed output serialization failed",
                    )
                })
            }
            "cipher.seriated_playfair.encode" => {
                let p: geometry::SeriatedPlayfair = poly::parse_parameters(&document)?;
                serde_json::to_value(geometry::seriated_playfair(&input.text, &p, false)?).map_err(
                    |_| {
                        OperationError::new(
                            ErrorCode::InvalidInput,
                            "typed output serialization failed",
                        )
                    },
                )
            }
            "cipher.seriated_playfair.decode" => {
                let p: geometry::SeriatedPlayfair = poly::parse_parameters(&document)?;
                serde_json::to_value(geometry::seriated_playfair(&input.text, &p, true)?).map_err(
                    |_| {
                        OperationError::new(
                            ErrorCode::InvalidInput,
                            "typed output serialization failed",
                        )
                    },
                )
            }
            "cipher.two_square.encode" => {
                let p: geometry::Squares = poly::parse_parameters(&document)?;
                Ok(json!({"text":geometry::two_square(&input.text,&p,false)?}))
            }
            "cipher.two_square.decode" => {
                let p: geometry::Squares = poly::parse_parameters(&document)?;
                Ok(json!({"text":geometry::two_square(&input.text,&p,true)?}))
            }
            "cipher.four_square.encode" => {
                let p: geometry::Squares = poly::parse_parameters(&document)?;
                Ok(json!({"text":geometry::four_square(&input.text,&p,false)?}))
            }
            "cipher.four_square.decode" => {
                let p: geometry::Squares = poly::parse_parameters(&document)?;
                Ok(json!({"text":geometry::four_square(&input.text,&p,true)?}))
            }
            "cipher.hill.encode" => {
                let p: hill::Hill = poly::parse_parameters(&document)?;
                serde_json::to_value(hill::hill(&input.text, &p, false)?).map_err(|_| {
                    OperationError::new(
                        ErrorCode::InvalidInput,
                        "typed output serialization failed",
                    )
                })
            }
            "cipher.hill.decode" => {
                let p: hill::Hill = poly::parse_parameters(&document)?;
                serde_json::to_value(hill::hill(&input.text, &p, true)?).map_err(|_| {
                    OperationError::new(
                        ErrorCode::InvalidInput,
                        "typed output serialization failed",
                    )
                })
            }
            "cipher.digrafid.encode" => {
                let p: geometry::Digrafid = poly::parse_parameters(&document)?;
                serde_json::to_value(geometry::digrafid(&input.text, &p, false)?).map_err(|_| {
                    OperationError::new(
                        ErrorCode::InvalidInput,
                        "typed output serialization failed",
                    )
                })
            }
            "cipher.digrafid.decode" => {
                let p: geometry::Digrafid = poly::parse_parameters(&document)?;
                serde_json::to_value(geometry::digrafid(&input.text, &p, true)?).map_err(|_| {
                    OperationError::new(
                        ErrorCode::InvalidInput,
                        "typed output serialization failed",
                    )
                })
            }
            "cipher.tri_square.encode" => {
                let p: geometry::TriSquare = poly::parse_parameters(&document)?;
                Ok(json!({"text":geometry::tri_square(&input.text,&p,false)?}))
            }
            "cipher.tri_square.decode" => {
                let p: geometry::TriSquare = poly::parse_parameters(&document)?;
                Ok(json!({"text":geometry::tri_square(&input.text,&p,true)?}))
            }
            _ => Err(OperationError::new(
                ErrorCode::UnsupportedFormat,
                "unsupported polyalphabetic/polygraphic operation",
            )),
        }
    })();
    result.map_err(|e: OperationError| e.operation(op))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_44_typed_bridges_match_independent_core_cli_native_vectors() {
        let cases: Vec<Value> = serde_json::from_str(include_str!("poly_vectors.json")).unwrap();
        assert_eq!(cases.len(), 44);
        for case in cases {
            let id = case["operation"].as_str().unwrap();
            let descriptor = cube_core::operations::require(id).unwrap();
            let contract = descriptor.contract().unwrap();
            let inputs = case["inputs"].clone();
            let parameters = case["parameters"].clone();
            contract.validate_inputs(&inputs).unwrap();
            contract.validate_parameters(&parameters).unwrap();
            let actual = execute(id, inputs.clone(), parameters.clone()).unwrap();
            assert_eq!(actual, case["outputs"], "{id}");
            contract.validate_outputs(&actual).unwrap();
            let mut wrong = parameters;
            wrong
                .as_object_mut()
                .unwrap()
                .insert("unknown".into(), json!(true));
            assert!(execute(id, inputs, wrong).is_err(), "{id}");
        }
    }
    #[test]
    fn explicit_port_outputs_keep_filler_evidence_and_private_policy() {
        let output = execute(
            "cipher.playfair.encode",
            json!({"text":"XX"}),
            json!({"square":"ABCDEFGHIKLMNOPQRSTUVWXYZ"}),
        )
        .unwrap();
        assert_eq!(output["prepared"], "XQXQ");
        assert_eq!(output["fillers"], json!([1, 3]));
        let descriptor = cube_core::operations::require("cipher.solitaire.encode").unwrap();
        let contract = descriptor.contract().unwrap();
        assert!(contract.policy.private_input && contract.policy.private_output);
        assert!(
            !contract.policy.automatic_capture
                && !contract.policy.grants_network_access
                && !contract.policy.grants_filesystem_access
        );
        assert!(descriptor.discovery().pipeline.is_none());
        let error = execute(
            "cipher.solitaire.encode",
            json!({"text":"PRIVATE_MESSAGE"}),
            json!({"deck":[]}),
        )
        .unwrap_err();
        assert!(!error.to_string().contains("PRIVATE_MESSAGE"));
        assert_eq!(error.operation, Some("cipher.solitaire.encode"));
    }
}
