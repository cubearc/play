use super::{execute, types::*};
use std::{
    io::{Read, Write},
    process::ExitCode,
};

pub fn main() -> ExitCode {
    // This branch runs before creating an async runtime, with no workspace paths.
    #[cfg(unix)]
    {
        use rustix::process::{Resource, Rlimit, setrlimit};
        for (resource, limit) in [
            (Resource::Core, 0),
            (Resource::Cpu, 4),
            (Resource::Nofile, 32),
        ] {
            if setrlimit(
                resource,
                Rlimit {
                    current: Some(limit),
                    maximum: Some(limit),
                },
            )
            .is_err()
            {
                return ExitCode::FAILURE;
            }
        }
        #[cfg(target_os = "linux")]
        if setrlimit(
            Resource::As,
            Rlimit {
                current: Some(128 * 1024 * 1024),
                maximum: Some(128 * 1024 * 1024),
            },
        )
        .is_err()
        {
            return ExitCode::FAILURE;
        }
    }
    let result = (|| -> Result<WorkerOutput> {
        let mut input = Vec::new();
        std::io::stdin()
            .take(MAX_WIRE as u64 + 1)
            .read_to_end(&mut input)
            .map_err(|_| Error::invalid())?;
        if input.len() > MAX_WIRE {
            return Err(Error::limit());
        }
        let request =
            serde_json::from_slice::<WorkerRequest>(&input).map_err(|_| Error::invalid())?;
        execute::run(request)
    })();
    match execute::json_bytes(&result, MAX_WIRE).and_then(|bytes| {
        std::io::stdout()
            .write_all(&bytes)
            .map_err(|_| Error::invalid())
    }) {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn request(operation: &str, input: Data) -> WorkerRequest {
        WorkerRequest {
            request: Request {
                id: uuid::Uuid::new_v4(),
                operation: operation.into(),
                version: 1,
                input: Input::Text {
                    data: String::new(),
                },
                other: None,
                parameters: Vec::new(),
                timeout_ms: TIMEOUT_MS,
                max_output_bytes: MAX_OUTPUT,
                retain_result: true,
            },
            input,
            other: None,
        }
    }
    #[test]
    fn real_core_agreement_binary_types_defaults_and_errors() {
        let input = Data::new(Representation::Bytes, "bytes", &[0, 255, 128], json!(null));
        let encoded = execute::run(request("encoding.base64.encode", input)).unwrap();
        assert_eq!(
            encoded.data.bytes(MAX_OUTPUT).unwrap(),
            cube_core::encoding::base64_encode(&[0, 255, 128]).as_bytes()
        );
        let input = Data::new(Representation::Text, "text", b"AP+A", json!(null));
        let decoded = execute::run(request("encoding.base64.decode", input)).unwrap();
        assert_eq!(decoded.data.bytes(MAX_OUTPUT).unwrap(), [0, 255, 128]);
        assert!(
            execute::run(request(
                "encoding.base64.decode",
                Data::new(Representation::Bytes, "bytes", b"AP+A", json!(null))
            ))
            .is_err()
        );
        let mut job = request(
            "cipher.caesar.encode",
            Data::new(Representation::Text, "text", b"hello", json!(null)),
        );
        assert_eq!(
            execute::run(job.clone()).unwrap_err().code,
            "invalid_parameter"
        );
        job.request.parameters = vec![Binding {
            name: "shift".into(),
            value: cube_core::pipeline::Parameter::Integer(1),
        }];
        assert!(
            execute::run(job.clone()).unwrap().parameters[0]["redacted"]
                .as_bool()
                .unwrap()
        );
        job.request
            .parameters
            .push(job.request.parameters[0].clone());
        assert_eq!(execute::run(job).unwrap_err().code, "invalid_parameter");
    }
    #[test]
    fn image_direct_adapters_use_core_even_without_pipeline_support() {
        let pixels = [255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255];
        let image = cube_core::image::Image::from_bytes(
            2,
            2,
            cube_core::image::ColorModel::Rgb,
            &pixels,
            &[],
            &[],
        )
        .unwrap();
        let png = image.encode_png().unwrap();
        let input = Data::new(Representation::Image, "image.samples", &png, json!(null));
        let histogram = execute::run(request("image.histogram", input.clone())).unwrap();
        assert_eq!(
            cube_core::pipeline::adapter_version("image.histogram"),
            None
        );
        let actual: serde_json::Value =
            serde_json::from_slice(&histogram.data.bytes(MAX_OUTPUT).unwrap()).unwrap();
        assert_eq!(
            actual,
            serde_json::to_value(image.histogram(None, None, 1000).unwrap()).unwrap()
        );
        let mut job = request("image.pixels.read", input.clone());
        job.request.parameters = vec![
            Binding {
                name: "x".into(),
                value: cube_core::pipeline::Parameter::Integer(1),
            },
            Binding {
                name: "y".into(),
                value: cube_core::pipeline::Parameter::Integer(0),
            },
        ];
        assert_eq!(
            execute::run(job).unwrap().data.bytes(MAX_OUTPUT).unwrap(),
            image.pixel(1, 0).unwrap()
        );
        let mut job = request("image.channel.extract", input.clone());
        job.request.parameters = vec![Binding {
            name: "channel".into(),
            value: cube_core::pipeline::Parameter::Text("red".into()),
        }];
        assert_eq!(
            execute::run(job).unwrap().data.semantic_type,
            "image.pixel_data"
        );
        let mut job = request("image.compare", input.clone());
        job.request.other = Some(Input::Artifact {
            id: uuid::Uuid::new_v4(),
        });
        job.other = Some(input.clone());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(
                &execute::run(job).unwrap().data.bytes(MAX_OUTPUT).unwrap()
            )
            .unwrap()["equal"],
            true
        );
        assert_eq!(
            execute::run(request("image.invert", input))
                .unwrap()
                .data
                .representation,
            Representation::Image
        );
    }

    #[test]
    fn every_direct_adapter_has_a_working_core_contract() {
        let source = cube_core::image::Image::from_bytes(
            2,
            1,
            cube_core::image::ColorModel::Rgb,
            &[1, 2, 3, 4, 5, 6],
            &[],
            &[],
        )
        .unwrap();
        let png = source.encode_png().unwrap();
        let input = Data::new(Representation::Image, "image.samples", &png, json!(null));
        for (operation, bindings) in [
            ("image.encode", json!([])),
            ("image.pixels.export", json!([])),
            ("image.grayscale", json!([])),
            ("image.rotate", json!([{"name":"degrees","value":90}])),
            (
                "image.mirror",
                json!([{"name":"axis","value":"horizontal"}]),
            ),
            (
                "image.crop",
                json!([{"name":"region","value":{"unsigned_list":[1,0,1,1]}}]),
            ),
            (
                "image.reorder",
                json!([{"name":"channels","value":{"text_list":["blue","green","red"]}}]),
            ),
        ] {
            let mut job = request(operation, input.clone());
            job.request.parameters = serde_json::from_value(bindings).unwrap();
            assert!(execute::run(job).is_ok(), "{operation}");
        }
        assert!(execute::run(request("image.palette.expand", input.clone())).is_err());
        let indexed = cube_core::image::Image::from_bytes(
            2,
            1,
            cube_core::image::ColorModel::Indexed,
            &[0, 1],
            &[[1, 2, 3], [4, 5, 6]],
            &[],
        )
        .unwrap()
        .encode_png()
        .unwrap();
        assert!(
            execute::run(request(
                "image.palette.expand",
                Data::new(
                    Representation::Image,
                    "image.samples",
                    &indexed,
                    json!(null)
                )
            ))
            .is_ok()
        );
        assert!(
            execute::run(request(
                "image.decode",
                Data::new(Representation::Bytes, "bytes", &png, json!(null))
            ))
            .is_ok()
        );
        let mut job = request("image.compare", input.clone());
        job.request.other = Some(Input::Artifact {
            id: uuid::Uuid::new_v4(),
        });
        job.other = Some(input.clone());
        job.request.parameters =
            serde_json::from_value(json!([{"name":"mask","value":true}])).unwrap();
        let comparison = execute::run(job).unwrap();
        assert!(
            serde_json::from_slice::<serde_json::Value>(
                &comparison.data.bytes(MAX_OUTPUT).unwrap()
            )
            .unwrap()["mask_png_base64"]
                .is_string()
        );
        let mut job = request("image.histogram", input);
        job.request.max_output_bytes = 1;
        assert_eq!(execute::run(job).unwrap_err().code, "resource_limit");
    }
}
