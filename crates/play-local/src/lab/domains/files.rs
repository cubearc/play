//! Bounded, inert byte readers and offline document inspection. No paths are opened.
use super::field;
use cube_core::{
    errors::{ErrorCode, OperationError},
    forensics::{self, ByteRange, boundaries, exif, ranges, signatures, zip},
    web,
};
use serde::Serialize;
use serde_json::{Value, json};
use std::io::Cursor;

enum Adapter {
    Appended,
    Compare,
    Exif,
    Hex,
    Extract,
    Signature,
    Scan,
    Strings,
    Zip,
    Carve,
    Info,
    Html,
    Har,
}
fn adapter(op: &str) -> Option<Adapter> {
    Some(match op {
        "file.appended.detect" => Adapter::Appended,
        "file.binary.compare" => Adapter::Compare,
        "file.exif.inspect" => Adapter::Exif,
        "file.hex.inspect" => Adapter::Hex,
        "file.range.extract" => Adapter::Extract,
        "file.signature.detect" => Adapter::Signature,
        "file.signature.scan" => Adapter::Scan,
        "file.strings.extract" => Adapter::Strings,
        "file.zip.validate" => Adapter::Zip,
        "file.carve" => Adapter::Carve,
        "file.info.inspect" => Adapter::Info,
        "web.html.inspect" => Adapter::Html,
        "web.har.inspect" => Adapter::Har,
        _ => return None,
    })
}
pub(super) fn supported(op: &str) -> bool {
    adapter(op).is_some()
}
fn invalid() -> OperationError {
    OperationError::new(
        ErrorCode::InvalidInput,
        "use exact bytes and bounded file inspection parameters",
    )
}
fn number(p: &Value, key: &str) -> Result<u64, OperationError> {
    let value = &p[key];
    value
        .as_u64()
        .or_else(|| value.as_str()?.parse().ok())
        .ok_or_else(invalid)
}
fn selection(p: &Value) -> Result<ByteRange, OperationError> {
    Ok(ByteRange {
        offset: number(p, "offset")?,
        length: p.get("length").map(|_| number(p, "length")).transpose()?,
    })
}
fn output(value: impl Serialize) -> Result<Value, OperationError> {
    serde_json::to_value(value)
        .map(|v| json!({"output":v}))
        .map_err(|_| invalid())
}
pub(super) fn execute(op: &str, inputs: Value, p: Value) -> Result<Value, OperationError> {
    let identity = cube_core::operations::require(op)?.id;
    let selected = adapter(op).ok_or_else(invalid)?;
    let result = (|| {
        if matches!(selected, Adapter::Info) {
            return Err(OperationError::new(
                ErrorCode::InvalidInput,
                "File inspection requires an authorized workspace file binding",
            ));
        }
        if matches!(selected, Adapter::Carve) {
            let selection = if p.get("length").is_some() {
                forensics::carving::Selection::Range {
                    offset: number(&p, "offset")?,
                    length: number(&p, "length")?,
                }
            } else {
                forensics::carving::Selection::Candidate {
                    offset: number(&p, "offset")?,
                }
            };
            let (report, data) = forensics::carving::carve_reader(
                &mut Cursor::new(super::bytes(&inputs["source"])?),
                selection,
                number(&p, "max_bytes")?,
            )?;
            return Ok(
                json!({"output":report,"data":{"base64":cube_core::encoding::base64_encode(&data)}}),
            );
        }
        if matches!(selected, Adapter::Html | Adapter::Har) {
            let text: String = field(&inputs, "input")?;
            return match selected {
                Adapter::Html => output(web::html::inspect(&text)?),
                _ => output(web::har::inspect(&text)?),
            };
        }
        if matches!(selected, Adapter::Compare) {
            return output(ranges::compare(
                &mut Cursor::new(super::bytes(&inputs["left"])?),
                &mut Cursor::new(super::bytes(&inputs["right"])?),
                selection(&p)?,
                field(&p, "max_results")?,
            )?);
        }
        let bytes = super::bytes(&inputs["input"])?;
        let mut reader = Cursor::new(&bytes);
        match selected {
            Adapter::Appended => output(boundaries::inspect(&mut reader)?),
            Adapter::Exif => output(exif::inspect(&bytes)?),
            Adapter::Signature => output(signatures::detect(&bytes)),
            Adapter::Hex => {
                let mut range = selection(&p)?;
                // Match Core's documented CLI default; an explicit length is exact.
                if range.length.is_none() {
                    range.length = Some((bytes.len() as u64).saturating_sub(range.offset).min(256));
                }
                output(ranges::hex_rows(&mut reader, range, field(&p, "width")?)?)
            }
            Adapter::Extract => {
                let data = forensics::read_range(&mut reader, selection(&p)?)?;
                output(json!({"base64":cube_core::encoding::base64_encode(&data)}))
            }
            Adapter::Scan => {
                let format = p
                    .get("format")
                    .map(|_| field::<String>(&p, "format"))
                    .transpose()?;
                output(signatures::scan(
                    &mut reader,
                    selection(&p)?,
                    format.as_deref(),
                    field(&p, "max_results")?,
                )?)
            }
            Adapter::Strings => output(ranges::strings(
                &mut reader,
                selection(&p)?,
                field(&p, "min_length")?,
                field(&p, "max_results")?,
            )?),
            Adapter::Zip => output(zip::validate(&mut reader, number(&p, "offset")?)?),
            _ => Err(invalid()),
        }
    })();
    result.map_err(|e: OperationError| e.operation(identity))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn wire(bytes: &[u8]) -> Value {
        json!({"base64":cube_core::encoding::base64_encode(bytes)})
    }
    fn run(op: &str, inputs: Value, mut p: Value) -> Result<Value, OperationError> {
        let contract = cube_core::operations::require(op)?.contract().unwrap();
        for (key, schema) in contract.parameters.as_value()["properties"]
            .as_object()
            .unwrap()
        {
            if let Some(default) = schema.get("default") {
                p.as_object_mut()
                    .unwrap()
                    .entry(key)
                    .or_insert_with(|| default.clone());
            }
        }
        contract.validate_inputs(&inputs).unwrap();
        contract.validate_parameters(&p).unwrap();
        let result = execute(op, inputs, p)?;
        contract
            .validate_outputs(&result)
            .unwrap_or_else(|e| panic!("{op}: {e}"));
        Ok(result)
    }
    #[test]
    fn eleven_inert_readers_preserve_offsets_exact_bytes_and_offline_metadata() {
        let mut png = include_bytes!("../../../../../tests/fixtures/sample.png").to_vec();
        png.extend_from_slice(b"tail");
        let zip = cube_core::encoding::hex_decode("504b0506000000000000000000000000000000000000")
            .unwrap();
        let cases = [
            (
                "file.appended.detect",
                json!({"input":wire(&png)}),
                json!({}),
                "/output/trailing_bytes",
                json!(4),
            ),
            (
                "file.binary.compare",
                json!({"left":wire(b"abc"),"right":wire(b"axc!")}),
                json!({}),
                "/output/differing_bytes",
                json!(2),
            ),
            (
                "file.exif.inspect",
                json!({"input":wire(b"II\x2a\0\x08\0\0\0\0\0\0\0\0\0")}),
                json!({}),
                "/output/present",
                json!(false),
            ),
            (
                "file.hex.inspect",
                json!({"input":wire(b"\0ABC\xff")}),
                json!({"offset":"1","length":3,"width":2}),
                "/output/0/hex",
                json!("4142"),
            ),
            (
                "file.range.extract",
                json!({"input":wire(b"\0\xff\x80END")}),
                json!({"offset":1,"length":2}),
                "/output/base64",
                json!("/4A="),
            ),
            (
                "file.signature.detect",
                json!({"input":wire(&png)}),
                json!({}),
                "/output/0/format",
                json!("png"),
            ),
            (
                "file.signature.scan",
                json!({"input":wire(b"abc%PDF-1.7")}),
                json!({"format":"pdf"}),
                "/output/candidates/0/offset",
                json!(3),
            ),
            (
                "file.strings.extract",
                json!({"input":wire(b"\0alpha\0beta\xff")}),
                json!({}),
                "/output/strings/1/text",
                json!("beta"),
            ),
            (
                "file.zip.validate",
                json!({"input":wire(&zip)}),
                json!({}),
                "/output/entries",
                json!([]),
            ),
            (
                "web.html.inspect",
                json!({"input":"<p>Hello</p><img src='https://example.invalid/x'><script>throw 1</script>"}),
                json!({}),
                "/output/references/0/value",
                json!("https://example.invalid/x"),
            ),
            (
                "web.har.inspect",
                json!({"input":"{\"log\":{\"version\":\"1.2\",\"entries\":[{\"request\":{\"method\":\"GET\",\"url\":\"https://example.invalid\"},\"response\":{\"status\":200}}]}}"}),
                json!({}),
                "/output/entries/0/status",
                json!(200),
            ),
        ];
        for (op, input, p, pointer, expected) in cases {
            let actual = run(op, input, p).unwrap_or_else(|e| panic!("{op}: {e}"));
            assert_eq!(actual.pointer(pointer), Some(&expected), "{op}");
        }
        assert!(supported("file.info.inspect")); // Execution needs the capability resolver.
        assert!(supported("file.carve"));
    }
    #[test]
    fn carving_preserves_selected_bytes_and_revalidates_candidate_framing() {
        let png = include_bytes!("../../../../../tests/fixtures/sample.png");
        let mut source = b"host".to_vec();
        source.extend_from_slice(png);
        source.extend_from_slice(b"tail");
        let result = run(
            "file.carve",
            json!({"source":wire(&source)}),
            json!({"offset":4}),
        )
        .unwrap();
        assert_eq!(super::super::bytes(&result["data"]).unwrap(), png);
        assert_eq!(result["output"]["sha256"], cube_core::hashing::sha256(png));
        assert_eq!(result["output"]["format"], "png");
        let raw = run(
            "file.carve",
            json!({"source":wire(b"a\0\xffz")}),
            json!({"offset":1,"length":2}),
        )
        .unwrap();
        assert_eq!(super::super::bytes(&raw["data"]).unwrap(), [0, 255]);
        assert_eq!(raw["output"]["format"], Value::Null);
        assert!(
            run(
                "file.carve",
                json!({"source":wire(&source)}),
                json!({"offset":4,"max_bytes":1})
            )
            .is_err()
        );
        assert!(
            execute(
                "file.info.inspect",
                json!({"input":{"reference":"/etc/passwd"}}),
                json!({})
            )
            .is_err()
        );
    }
    #[test]
    fn invalid_readers_fail_without_path_resolution_or_silent_truncation() {
        for (op, p) in [
            (
                "file.range.extract",
                json!({"offset":"18446744073709551615","length":1}),
            ),
            ("file.hex.inspect", json!({"offset":5})),
            ("file.zip.validate", json!({})),
            ("file.appended.detect", json!({})),
        ] {
            assert!(run(op, json!({"input":wire(b"abc")}), p).is_err(), "{op}");
        }
        assert!(
            run(
                "web.har.inspect",
                json!({"input":"{\"log\":{},\"log\":{}}"}),
                json!({})
            )
            .is_err()
        );
        assert!(
            execute(
                "file.hex.inspect",
                json!({"input":{"reference":"/etc/passwd"}}),
                json!({"offset":0,"width":16})
            )
            .is_err()
        );
    }
}
