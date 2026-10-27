//! Bounded native container readers; unsupported codecs fail without external tools.
use super::{
    field,
    images::{bytes, channels, image},
};
use cube_core::{
    errors::{ErrorCode, OperationError},
    media::video::{self, audio, comparison, frames},
};
use serde_json::{Value, json};
use std::io::Cursor;
enum Adapter {
    Audio,
    Inspect,
    Compare,
    Frames,
}
fn adapter(op: &str) -> Option<Adapter> {
    Some(match op {
        "video.audio.extract" => Adapter::Audio,
        "video.container.inspect" => Adapter::Inspect,
        "video.frames.compare" => Adapter::Compare,
        "video.frames.extract" => Adapter::Frames,
        _ => return None,
    })
}
pub(super) fn supported(op: &str) -> bool {
    adapter(op).is_some()
}
fn invalid() -> OperationError {
    OperationError::new(
        ErrorCode::InvalidInput,
        "use explicit track IDs and bounded frame selections",
    )
}
pub(super) fn execute(op: &str, inputs: Value, p: Value) -> Result<Value, OperationError> {
    let identity = cube_core::operations::require(op)?.id;
    let selected = adapter(op).ok_or_else(invalid)?;
    let result = (|| {
        let mut reader = Cursor::new(super::bytes(&inputs["input"])?);
        Ok(match selected {
            Adapter::Inspect => json!({"output":video::inspect(&mut reader)?}),
            Adapter::Audio => {
                let r = audio::extract(&mut reader, field(&p, "track_id")?)?;
                json!({"output":r,"data":bytes(&r.data)})
            }
            Adapter::Frames => {
                let r = frames::extract(
                    &mut reader,
                    field(&p, "track_id")?,
                    &field::<Vec<usize>>(&p, "indexes")?,
                )?;
                let images: Vec<_> = r.iter().map(|f| image(&f.image)).collect();
                json!({"output":r,"images":images})
            }
            Adapter::Compare => {
                let indexes: Vec<usize> = field(&p, "pairs")?;
                if !indexes.len().is_multiple_of(2) {
                    return Err(invalid().parameter("pairs"));
                }
                let pairs: Vec<_> = indexes
                    .as_chunks::<2>()
                    .0
                    .iter()
                    .map(|p| (p[0], p[1]))
                    .collect();
                json!({"output":comparison::compare(&mut reader,field(&p,"track_id")?,&pairs,channels(&p)?.as_deref(),field(&p,"max_results")?)?})
            }
        })
    })();
    result.map_err(|e: OperationError| e.operation(identity))
}
#[cfg(test)]
mod tests {
    use super::*;
    fn run(op: &str, data: &[u8], mut p: Value) -> Result<Value, OperationError> {
        let c = cube_core::operations::require(op)?.contract().unwrap();
        for (k, s) in c.parameters.as_value()["properties"].as_object().unwrap() {
            if let Some(d) = s.get("default") {
                p.as_object_mut()
                    .unwrap()
                    .entry(k)
                    .or_insert_with(|| d.clone());
            }
        }
        let inputs = json!({"input":bytes(data)});
        c.validate_inputs(&inputs).unwrap();
        c.validate_parameters(&p).unwrap();
        let result = execute(op, inputs, p)?;
        c.validate_outputs(&result)
            .unwrap_or_else(|e| panic!("{op}: {e}"));
        Ok(result)
    }
    #[test]
    fn real_jpeg_frames_and_pcm_tracks_keep_selected_order_and_source_provenance() {
        let movie = include_bytes!("../../../../../tests/fixtures/jpeg-frames.mov");
        let info = run("video.container.inspect", movie, json!({})).unwrap();
        assert_eq!(info["output"]["tracks"][0]["id"], 1);
        let frames = run(
            "video.frames.extract",
            movie,
            json!({"track_id":1,"indexes":[1,0]}),
        )
        .unwrap();
        assert_eq!(frames["output"][0]["provenance"]["frame_index"], 1);
        assert_eq!(frames["output"][1]["provenance"]["frame_index"], 0);
        let left = super::super::images::raster(&frames["images"][0]).unwrap();
        let right = super::super::images::raster(&frames["images"][1]).unwrap();
        assert_eq!(left.samples(), [192, 192]);
        assert_eq!(right.samples(), [64, 64]);
        let report = run(
            "video.frames.compare",
            movie,
            json!({"track_id":1,"pairs":[0,1,0,0]}),
        )
        .unwrap();
        assert_eq!(report["output"]["pairs"][0]["comparison"]["equal"], false);
        assert_eq!(report["output"]["pairs"][1]["comparison"]["equal"], true);
        let audio = run(
            "video.audio.extract",
            include_bytes!("../../../../../tests/fixtures/pcm-track.mov"),
            json!({"track_id":1}),
        )
        .unwrap();
        let data = super::super::bytes(&audio["data"]).unwrap();
        let decoded =
            cube_core::media::pcm::extract(&mut Cursor::new(&data), &Default::default()).unwrap();
        assert_eq!(decoded.raw(), [0, 128, 255, 64]);
        assert_eq!(
            audio["output"]["provenance"]["pcm_sha256"],
            cube_core::hashing::sha256(&[0, 128, 255, 64])
        );
        assert!(
            run(
                "video.frames.extract",
                movie,
                json!({"track_id":1,"indexes":[99]})
            )
            .is_err()
        );
        assert!(
            run(
                "video.frames.compare",
                movie,
                json!({"track_id":1,"pairs":[0,1,0]})
            )
            .is_err()
        );
        assert!(run("video.audio.extract", movie, json!({"track_id":1})).is_err());
        assert!(run("video.container.inspect", b"malformed", json!({})).is_err());
    }
}
