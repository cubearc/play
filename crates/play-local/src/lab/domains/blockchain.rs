//! Offline Core validation/derivation. Private material stays behind catalog policy.
use super::{field, images::bytes};
use cube_core::{
    blockchain::{self as key, bitcoin, candidates, ethereum, private, wif},
    errors::{ErrorCode, OperationError},
};
use serde_json::{Value, json};
enum Adapter {
    Bitcoin,
    Candidates,
    Ethereum,
    Export,
    Scalar,
    Wif,
}
fn adapter(op: &str) -> Option<Adapter> {
    Some(match op {
        "blockchain.bitcoin.derive" => Adapter::Bitcoin,
        "blockchain.candidates.validate" => Adapter::Candidates,
        "blockchain.ethereum.derive" => Adapter::Ethereum,
        "blockchain.private.export" => Adapter::Export,
        "blockchain.scalar.validate" => Adapter::Scalar,
        "blockchain.wif.validate" => Adapter::Wif,
        _ => return None,
    })
}
pub(super) fn supported(op: &str) -> bool {
    adapter(op).is_some()
}
fn invalid() -> OperationError {
    OperationError::new(
        ErrorCode::InvalidInput,
        "use exact declared private material and explicit offline key options",
    )
}
fn text<'a>(p: &'a Value, k: &str) -> Result<&'a str, OperationError> {
    p[k].as_str().ok_or_else(invalid)
}
fn network(p: &Value, k: &str) -> Result<Option<key::Network>, OperationError> {
    p.get(k)
        .map(|_| key::Network::parse(text(p, k)?))
        .transpose()
}
pub(super) fn execute(op: &str, inputs: Value, p: Value) -> Result<Value, OperationError> {
    let identity = cube_core::operations::require(op)?.id;
    let selected = adapter(op).ok_or_else(invalid)?;
    let result = (|| {
        if matches!(selected, Adapter::Wif) {
            return Ok(
                json!({"output":wif::validate(text(&inputs,"input")?,network(&p,"expected_network")?)?}),
            );
        }
        if matches!(selected, Adapter::Candidates) {
            let document = serde_json::to_vec(&inputs["input"]).map_err(|_| invalid())?;
            let entries = candidates::parse_json(&document)?;
            let derive = candidates::derivation(
                Some(text(&p, "derive")?),
                network(&p, "network")?,
                p.get("compressed")
                    .map(|_| field(&p, "compressed"))
                    .transpose()?,
                p.get("checksum")
                    .map(|_| text(&p, "checksum"))
                    .transpose()?,
            )?;
            let results: Vec<_> = candidates::CandidateBatch::new(entries, derive)?.collect();
            return Ok(json!({"output":results}));
        }
        let material = super::bytes(&inputs["input"])?;
        let input = key::PrivateInput::parse(text(&p, "format")?, &material)?;
        Ok(match selected {
            Adapter::Bitcoin => {
                json!({"output":bitcoin::derive(input,network(&p,"network")?.ok_or_else(invalid)?,field(&p,"compressed")?)?})
            }
            Adapter::Ethereum => {
                json!({"output":ethereum::derive(input,ethereum::Checksum::parse(text(&p,"checksum")?)?)?})
            }
            Adapter::Scalar => json!({"output":key::validate_scalar(input)?}),
            Adapter::Export => {
                json!({"output":bytes(&private::export(input,private::OutputFormat::parse(text(&p,"output")?)?,network(&p,"network")?,p.get("compressed").map(|_|field(&p,"compressed")).transpose()?)?.into_private_bytes())})
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
        assert!(c.policy.private_input);
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
        c.validate_outputs(&result)
            .unwrap_or_else(|e| panic!("{op}: {e}"));
        Ok(result)
    }
    #[test]
    fn six_offline_adapters_preserve_known_public_values_and_explicit_private_exports() {
        let mut scalar = [0u8; 32];
        scalar[31] = 1;
        let material = json!({"input":bytes(&scalar)});
        let wif = "KwDiBf89QgGbjEhKnhXJuH7LrciVrZi3qYjgd9M7rFU73sVHnoWn";
        let cases = [
            (
                "blockchain.bitcoin.derive",
                material.clone(),
                json!({"network":"mainnet","compressed":true}),
                "/output/address",
                json!("1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH"),
            ),
            (
                "blockchain.ethereum.derive",
                material.clone(),
                json!({}),
                "/output/address",
                json!("0x7E5F4552091A69125d5DfCb7b8C2659029395Bdf"),
            ),
            (
                "blockchain.scalar.validate",
                material.clone(),
                json!({}),
                "/output/valid",
                json!(true),
            ),
            (
                "blockchain.wif.validate",
                json!({"input":wif}),
                json!({"expected_network":"mainnet"}),
                "/output/compressed",
                json!(true),
            ),
            (
                "blockchain.private.export",
                material.clone(),
                json!({"output":"raw"}),
                "/output",
                bytes(&scalar),
            ),
            (
                "blockchain.candidates.validate",
                json!({"input":[{"format":"wif","value":wif},{"format":"hex","value":"00"}]}),
                json!({}),
                "/output/1/valid",
                json!(false),
            ),
        ];
        for (op, i, p, pointer, expected) in cases {
            let result = run(op, i, p).unwrap_or_else(|e| panic!("{op}: {e}"));
            assert_eq!(result.pointer(pointer), Some(&expected), "{op}");
            if op != "blockchain.private.export" {
                assert!(!result.to_string().contains(wif));
                assert!(
                    !result
                        .to_string()
                        .contains(&cube_core::encoding::base64_encode(&scalar))
                );
            }
        }
        assert!(
            run(
                "blockchain.scalar.validate",
                json!({"input":bytes(&[0;32])}),
                json!({})
            )
            .is_err()
        );
        assert!(
            run(
                "blockchain.private.export",
                material,
                json!({"output":"wif"})
            )
            .is_err()
        );
    }
}
