//! Explicit presentation adapters; every transformation is implemented by Core.
use super::types::*;
use cube_core::{
    image::{Channel, Corner, Image, Region, Traversal},
    operations, pipeline,
};
use serde::Serialize;
use serde_json::{Value, json};

pub fn pipeline_version(id: &str) -> Option<u32> {
    // Masked transforms require retained audit artifacts beyond this initial adapter contract.
    if id == "text.transform.masked" {
        None
    } else {
        pipeline::adapter_version(id)
    }
}
pub fn direct(id: &str) -> bool {
    matches!(
        id,
        "image.decode"
            | "image.encode"
            | "image.palette.expand"
            | "image.pixels.read"
            | "image.pixels.export"
            | "image.channel.extract"
            | "image.histogram"
            | "image.compare"
            | "image.rotate"
            | "image.mirror"
            | "image.crop"
            | "image.invert"
            | "image.reorder"
            | "image.grayscale"
    )
}
pub fn core_version() -> String {
    pipeline::Plan::validate(
        pipeline::Kind::Bytes,
        Vec::new(),
        pipeline::Limits::default(),
    )
    .expect("empty Core plan")
    .info()
    .engine_version
    .into()
}
pub fn descriptor(id: &str) -> Result<&'static operations::Descriptor> {
    if id.len() > 128 {
        return Err(Error::invalid());
    }
    operations::describe(id).ok_or_else(|| {
        Error::new(
            "unknown_operation",
            "Choose an exact operation ID from the Core catalog.",
        )
    })
}
pub fn validate(request: &Request) -> Result<Vec<Binding>> {
    if request.id.get_version_num() != 4
        || request.max_output_bytes > MAX_OUTPUT
        || request.timeout_ms == 0
        || request.timeout_ms > TIMEOUT_MS
    {
        return Err(Error::limit());
    }
    if !request.retain_result
        && !matches!(
            request.operation.as_str(),
            "image.pixels.read" | "image.histogram" | "image.compare"
        )
    {
        return Err(Error::new(
            "invalid_input",
            "Only pixel, histogram and comparison probes may omit result retention.",
        ));
    }
    let descriptor = descriptor(&request.operation)?;
    let version = if direct(descriptor.id) {
        Some(ADAPTER_API)
    } else {
        pipeline_version(descriptor.id)
    };
    let version = version.ok_or_else(|| {
        Error::new(
            "unsupported_operation",
            "Core describes this operation, but this Lab has no execution adapter for it.",
        )
    })?;
    if version != request.version {
        return Err(Error::new(
            "contract_mismatch",
            "Use the exact Lab adapter version advertised by discovery.",
        ));
    }
    if (request.operation == "image.compare") != request.other.is_some() {
        return Err(Error::new(
            "invalid_input",
            "Only image.compare requires a second image input.",
        ));
    }
    parameters(descriptor, &request.parameters)
}
pub fn discovery(descriptor: &operations::Descriptor) -> Value {
    static ADAPTERS: std::sync::OnceLock<Vec<pipeline::AdapterInfo>> = std::sync::OnceLock::new();
    let pipeline = ADAPTERS
        .get_or_init(pipeline::supported_adapters)
        .iter()
        .find(|adapter| {
            adapter.operation == descriptor.id && pipeline_version(adapter.operation).is_some()
        });
    let lab = if direct(descriptor.id) {
        Some(
            json!({"mode":"direct","version":ADAPTER_API,"input_type":descriptor.input_type,"output_type":descriptor.output_type}),
        )
    } else {
        pipeline.map(|adapter| {let (input,output)=adapter.kinds().expect("Core adapter kinds");json!({"mode":"pipeline","version":adapter.version,"input_type":input,"output_type":output})})
    };
    json!({"core":descriptor.discovery(),"lab":lab})
}
struct Limited {
    bytes: Vec<u8>,
    maximum: usize,
}
impl std::io::Write for Limited {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        if bytes.len() > self.maximum.saturating_sub(self.bytes.len()) {
            return Err(std::io::Error::other("output limit"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
pub fn json_bytes(value: &impl Serialize, maximum: usize) -> Result<Vec<u8>> {
    let mut writer = Limited {
        bytes: Vec::new(),
        maximum,
    };
    serde_json::to_writer(&mut writer, value).map_err(|_| Error::limit())?;
    Ok(writer.bytes)
}
fn structured(kind: &str, value: &impl Serialize, maximum: usize) -> Result<Data> {
    Ok(Data::new(
        Representation::Structured,
        kind,
        &json_bytes(value, maximum)?,
        Value::Null,
    ))
}
fn image(value: Image) -> Result<Data> {
    let info = serde_json::to_value(value.info()).map_err(|_| Error::invalid())?;
    if json_bytes(&info, 64 * 1024).is_err() {
        return Err(Error::limit());
    }
    let png = value.encode_png()?;
    if png.len() > MAX_OUTPUT {
        return Err(Error::limit());
    }
    Ok(Data::new(
        Representation::Image,
        "image.samples",
        &png,
        info,
    ))
}
struct Arguments<'a>(&'a [Binding]);
impl<'a> Arguments<'a> {
    fn get(&self, name: &str) -> Option<&'a pipeline::Parameter> {
        self.0.iter().find(|p| p.name == name).map(|p| &p.value)
    }
    fn integer(&self, name: &str) -> Result<u32> {
        match self.get(name) {
            Some(pipeline::Parameter::Integer(v)) => {
                u32::try_from(*v).map_err(|_| Error::invalid())
            }
            _ => Err(Error::invalid()),
        }
    }
    fn boolean(&self, name: &str) -> bool {
        matches!(self.get(name), Some(pipeline::Parameter::Boolean(true)))
    }
    fn text(&self, name: &str) -> Result<&str> {
        match self.get(name) {
            Some(pipeline::Parameter::Text(v)) => Ok(v),
            _ => Err(Error::invalid()),
        }
    }
    fn region(&self) -> Option<Region> {
        match self.get("region") {
            Some(pipeline::Parameter::UnsignedList { values }) => Some(Region {
                x: values[0] as u32,
                y: values[1] as u32,
                width: values[2] as u32,
                height: values[3] as u32,
            }),
            _ => None,
        }
    }
    fn channels(&self) -> Result<Option<Vec<Channel>>> {
        match self.get("channels") {
            Some(pipeline::Parameter::TextList { values }) => Ok(Some(
                values
                    .iter()
                    .map(|v| Channel::parse(v).map_err(Error::from))
                    .collect::<Result<_>>()?,
            )),
            _ => Ok(None),
        }
    }
}
fn raster(data: &Data) -> Result<Image> {
    if data.representation != Representation::Image || data.semantic_type != "image.samples" {
        return Err(Error::new(
            "invalid_input",
            "The operation requires an explicit image representation.",
        ));
    }
    Ok(Image::decode(&data.bytes(MAX_INPUT)?)?)
}
pub fn run(job: WorkerRequest) -> Result<WorkerOutput> {
    let bindings = validate(&job.request)?;
    let args = Arguments(&bindings);
    let maximum = job.request.max_output_bytes;
    let data = if !direct(&job.request.operation) {
        let input = match job.input.representation {
            Representation::Bytes if job.input.semantic_type == "bytes" => pipeline::Value::Bytes {
                data: job.input.bytes(pipeline::MAX_VALUE_BYTES)?,
            },
            Representation::Text if job.input.semantic_type == "text" => pipeline::Value::Text {
                data: String::from_utf8(job.input.bytes(pipeline::MAX_VALUE_BYTES)?)
                    .map_err(|_| Error::invalid())?,
            },
            Representation::Structured => {
                let value = serde_json::from_slice::<pipeline::Value>(
                    &job.input.bytes(pipeline::MAX_VALUE_BYTES)?,
                )
                .map_err(|_| Error::invalid())?;
                let semantic = match value.kind() {
                    pipeline::Kind::ByteRange => "byte_range",
                    pipeline::Kind::ForensicReport => "forensic_report",
                    _ => return Err(Error::invalid()),
                };
                if job.input.semantic_type != semantic {
                    return Err(Error::invalid());
                }
                value
            }
            _ => {
                return Err(Error::new(
                    "invalid_input",
                    "Use the exact pipeline input kind advertised by discovery; conversions must be explicit.",
                ));
            }
        };
        let step = pipeline::Step {
            operation: job.request.operation.clone(),
            version: job.request.version,
            parameters: bindings
                .iter()
                .map(|v| (v.name.clone(), v.value.clone()))
                .collect(),
        };
        let limits = pipeline_limits(maximum);
        let execution = pipeline::Plan::validate(input.kind(), vec![step], limits)?.execute(
            input,
            &[],
            None,
        )?;
        match execution.output {
            pipeline::Value::Bytes { data } => {
                Data::new(Representation::Bytes, "bytes", &data, Value::Null)
            }
            pipeline::Value::Text { data } => {
                Data::new(Representation::Text, "text", data.as_bytes(), Value::Null)
            }
            value => structured(
                match value.kind() {
                    pipeline::Kind::ByteRange => "byte_range",
                    _ => "forensic_report",
                },
                &value,
                maximum,
            )?,
        }
    } else if job.request.operation == "image.decode" {
        if job.input.representation != Representation::Bytes || job.input.semantic_type != "bytes" {
            return Err(Error::invalid());
        }
        image(Image::decode(&job.input.bytes(MAX_INPUT)?)?)?
    } else {
        let source = raster(&job.input)?;
        let source = source.view(args.boolean("expand_palette"))?;
        let channels = args.channels()?;
        let channels = channels.as_deref();
        match job.request.operation.as_str() {
            "image.encode" => Data::new(
                Representation::Bytes,
                "bytes",
                &source.encode_png()?,
                Value::Null,
            ),
            "image.palette.expand" => image(source.expand_palette()?)?,
            "image.pixels.read" => Data::new(
                Representation::Bytes,
                "bytes",
                &source.pixel(args.integer("x")?, args.integer("y")?)?,
                Value::Null,
            ),
            "image.pixels.export" => structured(
                "image.pixel_data",
                &source.pixels_with_traversal(
                    args.region(),
                    channels,
                    Traversal::parse(args.text("traversal")?)?,
                    Corner::parse(args.text("start_corner")?)?,
                    args.boolean("reverse"),
                )?,
                maximum,
            )?,
            "image.channel.extract" => {
                let pixels =
                    source.channel(Channel::parse(args.text("channel")?)?, args.region())?;
                #[derive(Serialize)]
                struct ChannelResult<'a> {
                    #[serde(flatten)]
                    pixels: &'a cube_core::image::PixelData,
                    preview_png_base64: String,
                }
                let preview =
                    cube_core::encoding::base64_encode(&pixels.grayscale_image()?.encode_png()?);
                structured(
                    "image.pixel_data",
                    &ChannelResult {
                        pixels: &pixels,
                        preview_png_base64: preview,
                    },
                    maximum,
                )?
            }
            "image.histogram" => structured(
                "image.histogram",
                &source.histogram(
                    args.region(),
                    channels,
                    args.integer("max_colors")? as usize,
                )?,
                maximum,
            )?,
            "image.compare" => {
                let other = raster(job.other.as_ref().ok_or_else(Error::invalid)?)?;
                let comparison = source.compare(
                    &other,
                    args.region(),
                    channels,
                    args.integer("max_results")? as usize,
                    args.boolean("mask"),
                )?;
                #[derive(Serialize)]
                struct ComparisonResult<'a> {
                    #[serde(flatten)]
                    comparison: &'a cube_core::image::Comparison,
                    mask_png_base64: Option<String>,
                }
                let mask = comparison
                    .mask
                    .as_ref()
                    .map(|image| {
                        image
                            .encode_png()
                            .map(|v| cube_core::encoding::base64_encode(&v))
                    })
                    .transpose()?;
                structured(
                    "image.comparison",
                    &ComparisonResult {
                        comparison: &comparison,
                        mask_png_base64: mask,
                    },
                    maximum,
                )?
            }
            "image.rotate" => image(source.rotate(args.integer("degrees")?)?)?,
            "image.mirror" => image(source.mirror(args.text("axis")?)?)?,
            "image.crop" => image(source.crop(args.region().ok_or_else(Error::invalid)?)?)?,
            "image.invert" => image(source.invert(channels)?)?,
            "image.reorder" => image(source.reorder(channels.ok_or_else(Error::invalid)?)?)?,
            "image.grayscale" => image(source.grayscale()?)?,
            _ => return Err(Error::invalid()),
        }
    };
    data.bytes(maximum)?;
    let parameters = bindings
        .iter()
        .map(|p| {
            if direct(&job.request.operation) {
                json!({"name":p.name,"value":p.value})
            } else {
                json!({"name":p.name,"redacted":true})
            }
        })
        .collect();
    Ok(WorkerOutput { data, parameters })
}

pub fn pipeline_limits(maximum: usize) -> pipeline::Limits {
    pipeline::Limits {
        max_steps: 1,
        max_value_bytes: pipeline::MAX_VALUE_BYTES.min(maximum),
        max_work_bytes: pipeline::MAX_WORK_BYTES,
        max_capture_bytes: 0,
    }
}
