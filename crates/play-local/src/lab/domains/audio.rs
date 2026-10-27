//! Native Core audio analyses. Transport never synthesizes a container or decodes samples.
use super::{field, images::bytes};
use cube_core::{
    errors::{ErrorCode, OperationError},
    media::{bits, dtmf, envelope, metadata, morse, pcm, reverse, spectrogram, spectrum, wave},
    operations::contracts::values,
};
use serde_json::{Value, json};
use std::io::Cursor;

enum Adapter {
    Bits,
    Dtmf,
    Metadata,
    Detect,
    Transcribe,
    Pcm,
    Reverse,
    Spectrogram,
    Spectrum,
    Wave,
    Envelope,
}
fn adapter(op: &str) -> Option<Adapter> {
    Some(match op {
        "audio.bits.extract" => Adapter::Bits,
        "audio.dtmf.detect" => Adapter::Dtmf,
        "audio.metadata.inspect" => Adapter::Metadata,
        "audio.morse.detect" => Adapter::Detect,
        "audio.morse.transcribe" => Adapter::Transcribe,
        "audio.pcm.extract" => Adapter::Pcm,
        "audio.reverse" => Adapter::Reverse,
        "audio.spectrogram" => Adapter::Spectrogram,
        "audio.spectrum" => Adapter::Spectrum,
        "audio.wave.inspect" => Adapter::Wave,
        "audio.waveform.envelope" => Adapter::Envelope,
        _ => return None,
    })
}
pub(super) fn supported(op: &str) -> bool {
    adapter(op).is_some()
}
fn invalid() -> OperationError {
    OperationError::new(
        ErrorCode::InvalidInput,
        "use exact logical PCM fields and audio parameters",
    )
}
fn text<'a>(p: &'a Value, k: &str) -> Result<&'a str, OperationError> {
    p[k].as_str().ok_or_else(invalid)
}
fn number(p: &Value, k: &str) -> Result<u64, OperationError> {
    p[k].as_u64()
        .or_else(|| p[k].as_str()?.parse().ok())
        .ok_or_else(invalid)
}
fn samples(value: &Value) -> Result<pcm::Samples, OperationError> {
    let f = &value["format"];
    pcm::Samples::from_raw(
        wave::Format {
            format_tag: field(f, "format_tag")?,
            channels: field(f, "channels")?,
            sample_rate: field(f, "sample_rate")?,
            bits_per_sample: field(f, "bits_per_sample")?,
            block_align: field(f, "block_align")?,
            byte_rate: field(f, "byte_rate")?,
        },
        field(value, "channels")?,
        number(value, "start_frame")?,
        number(value, "frames")?,
        super::bytes(&value["data"])?,
    )
}
fn selection(p: &Value) -> Result<pcm::Selection, OperationError> {
    Ok(pcm::Selection {
        start_frame: number(p, "start_frame")?,
        frames: p.get("frames").map(|_| number(p, "frames")).transpose()?,
        channels: p
            .get("channels")
            .map(|_| field(p, "channels"))
            .transpose()?,
    })
}
pub(super) fn execute(op: &str, inputs: Value, p: Value) -> Result<Value, OperationError> {
    let identity = cube_core::operations::require(op)?.id;
    let selected = adapter(op).ok_or_else(invalid)?;
    let result = (|| {
        if matches!(
            selected,
            Adapter::Wave | Adapter::Pcm | Adapter::Metadata | Adapter::Envelope
        ) {
            let mut reader = Cursor::new(super::bytes(&inputs["input"])?);
            return Ok(match selected {
                Adapter::Wave => json!({"output":wave::inspect(&mut reader)?}),
                Adapter::Metadata => json!({"output":metadata::inspect(&mut reader)?}),
                Adapter::Pcm => {
                    json!({"output":values::Pcm::from(&pcm::extract(&mut reader,&selection(&p)?)?)})
                }
                _ => {
                    json!({"output":envelope::from_reader(&mut reader,&selection(&p)?,number(&p,"bucket_frames")?,field(&p,"max_buckets")?)?})
                }
            });
        }
        if matches!(selected, Adapter::Transcribe) {
            let detection: morse::Detection = field(&inputs, "input")?;
            let rate = field(&p, "sample_rate")?;
            if rate != detection.sample_rate {
                return Err(invalid().parameter("sample_rate"));
            }
            return Ok(
                json!({"output":morse::transcribe(&detection.intervals,rate,field(&p,"unit_ms")?,field(&p,"tolerance_percent")?)?}),
            );
        }
        let source = samples(&inputs["input"])?;
        Ok(match selected {
            Adapter::Bits => {
                json!({"output":bytes(&bits::extract(&source,&bits::Options {bits:field(&p,"bits")?,reverse_frames:field(&p,"reverse_frames")?,bit_order:bits::BitOrder::parse(text(&p,"bit_order")?)?,incomplete:bits::Incomplete::parse(text(&p,"incomplete")?)?})?.data)})
            }
            Adapter::Reverse => json!({"output":bytes(&reverse::wave(&source)?)}),
            Adapter::Dtmf => {
                json!({"output":dtmf::detect(&source,dtmf::Options {window_ms:field(&p,"window_ms")?,min_tone_ms:field(&p,"min_tone_ms")?,min_gap_ms:field(&p,"min_gap_ms")?,min_level_dbfs:field(&p,"min_level_dbfs")?,dominance_db:field(&p,"dominance_db")?,max_twist_db:field(&p,"max_twist_db")?,purity_percent:field(&p,"purity_percent")?})?})
            }
            Adapter::Detect => {
                json!({"output":morse::detect(&source,morse::Options {frequency_hz:field(&p,"frequency_hz")?,window_ms:field(&p,"window_ms")?,min_level_dbfs:field(&p,"min_level_dbfs")?,purity_percent:field(&p,"purity_percent")?})?})
            }
            Adapter::Spectrum => {
                json!({"output":spectrum::analyze(&source,spectrum::Options {fft_size:field(&p,"fft_size")?,window:spectrum::Window::parse(text(&p,"window")?)?})?})
            }
            Adapter::Spectrogram => {
                json!({"output":spectrogram::analyze(&source,spectrogram::Options {fft_size:field(&p,"fft_size")?,hop_size:field(&p,"hop_size")?,window:spectrum::Window::parse(text(&p,"window")?)?,final_window:spectrogram::FinalWindow::parse(text(&p,"final_window")?)?})?})
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
        for (k, s) in c.parameters.as_value()["properties"].as_object().unwrap() {
            if let Some(d) = s.get("default") {
                p.as_object_mut()
                    .unwrap()
                    .entry(k)
                    .or_insert_with(|| d.clone());
            }
        }
        c.validate_inputs(&inputs)
            .unwrap_or_else(|e| panic!("{op} inputs: {e}"));
        c.validate_parameters(&p)
            .unwrap_or_else(|e| panic!("{op} parameters: {e}"));
        let result = execute(op, inputs, p)?;
        c.validate_outputs(&result)
            .unwrap_or_else(|e| panic!("{op} outputs: {e}"));
        Ok(result)
    }
    #[test]
    fn eleven_audio_adapters_keep_exact_pcm_bytes_and_explicit_time_bases() {
        let wav = include_bytes!("../../../../../tests/fixtures/sample.wav");
        let pcm =
            run("audio.pcm.extract", json!({"input":bytes(wav)}), json!({})).unwrap()["output"]
                .clone();
        assert_eq!(super::super::bytes(&pcm["data"]).unwrap(), vec![0; 1600]);
        let mut eight = pcm.clone();
        eight["frames"] = json!(8);
        eight["data"] = bytes(&[0; 16]);
        let detected = run(
            "audio.morse.detect",
            json!({"input":pcm}),
            json!({"frequency_hz":700}),
        )
        .unwrap()["output"]
            .clone();
        assert_eq!(
            detected["intervals"],
            json!([{"start_frame":0,"frames":800,"state":"silence"}])
        );
        let tone = json!({"sample_rate":8000,"channel":0,"frequency_hz":700,"start_frame":0,"frames":800,"window_frames":80,"min_level_dbfs":-40,"purity_percent":80,"intervals":[{"start_frame":0,"frames":800,"state":"tone"}]});
        let mut flac = b"fLaC\x80\0\0\x22".to_vec();
        let mut info = [0u8; 34];
        info[..2].copy_from_slice(&16u16.to_be_bytes());
        info[2..4].copy_from_slice(&16u16.to_be_bytes());
        info[10..18].copy_from_slice(&((8000u64 << 44) | (15u64 << 36) | 800).to_be_bytes());
        flac.extend(info);
        let cases = [
            (
                "audio.bits.extract",
                json!({"input":eight}),
                json!({}),
                "/output/base64",
                json!("AA=="),
            ),
            (
                "audio.dtmf.detect",
                json!({"input":pcm}),
                json!({}),
                "/output/events",
                json!([]),
            ),
            (
                "audio.metadata.inspect",
                json!({"input":bytes(&flac)}),
                json!({}),
                "/output/sample_rate",
                json!(8000),
            ),
            (
                "audio.morse.transcribe",
                json!({"input":tone}),
                json!({"sample_rate":8000,"unit_ms":100}),
                "/output/text",
                json!("E"),
            ),
            (
                "audio.reverse",
                json!({"input":pcm}),
                json!({}),
                "/output",
                bytes(wav),
            ),
            (
                "audio.spectrogram",
                json!({"input":eight}),
                json!({"fft_size":8,"hop_size":8,"window":"rectangular"}),
                "/output/valid_frames",
                json!([8]),
            ),
            (
                "audio.spectrum",
                json!({"input":eight}),
                json!({"fft_size":8,"window":"rectangular"}),
                "/output/amplitudes",
                json!([[0.0, 0.0, 0.0, 0.0, 0.0]]),
            ),
            (
                "audio.wave.inspect",
                json!({"input":bytes(wav)}),
                json!({}),
                "/output/frames",
                json!(800),
            ),
            (
                "audio.waveform.envelope",
                json!({"input":bytes(wav)}),
                json!({"bucket_frames":800}),
                "/output/buckets/0/maximum",
                json!([0]),
            ),
        ];
        for (op, i, p, pointer, expected) in cases {
            let actual = run(op, i, p).unwrap_or_else(|e| panic!("{op}: {e}"));
            assert_eq!(actual.pointer(pointer), Some(&expected), "{op}");
        }
    }
    #[test]
    fn malformed_pcm_and_mismatched_time_bases_are_rejected() {
        let p = run(
            "audio.pcm.extract",
            json!({"input":bytes(include_bytes!("../../../../../tests/fixtures/sample.wav"))}),
            json!({}),
        )
        .unwrap()["output"]
            .clone();
        let mut short = p.clone();
        short["frames"] = json!(801);
        assert!(run("audio.reverse", json!({"input":short}), json!({})).is_err());
        let mut duplicate = p;
        duplicate["channels"] = json!([0, 0]);
        assert!(run("audio.reverse", json!({"input":duplicate}), json!({})).is_err());
        assert!(
            run(
                "audio.wave.inspect",
                json!({"input":bytes(b"not-wave")}),
                json!({})
            )
            .is_err()
        );
        assert!(
            run(
                "audio.spectrum",
                json!({"input":short}),
                json!({"fft_size":8})
            )
            .is_err()
        );
    }
}
