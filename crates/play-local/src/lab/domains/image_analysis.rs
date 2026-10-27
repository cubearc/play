//! Core raster analyses and explicit representations; no image algorithms in Play.
use super::{
    field,
    images::{bytes, channels, image, raster, region},
};
use cube_core::{
    errors::{ErrorCode, OperationError},
    image::{self, Image},
};
use serde_json::{Value, json};
use std::io::Cursor;

enum Adapter {
    Plane,
    Bitwise,
    Colors,
    ColorExports,
    Qr,
    CsvExport,
    CsvImport,
    Geometry,
    Lsb,
    Pairs,
    Png,
    Raw,
    Spatial,
}
fn adapter(op: &str) -> Option<Adapter> {
    Some(match op {
        "image.bit_plane.extract" => Adapter::Plane,
        "image.bitwise" => Adapter::Bitwise,
        "image.colors.extract" => Adapter::Colors,
        "image.colors.export" => Adapter::ColorExports,
        "image.qr.decode" => Adapter::Qr,
        "image.csv.export" => Adapter::CsvExport,
        "image.csv.import" => Adapter::CsvImport,
        "image.geometry.inspect" => Adapter::Geometry,
        "image.lsb.extract" => Adapter::Lsb,
        "image.pairs" => Adapter::Pairs,
        "image.png.inspect" => Adapter::Png,
        "image.raw.interpret" => Adapter::Raw,
        "image.spatial.autocorrelation" => Adapter::Spatial,
        _ => return None,
    })
}
pub(super) fn supported(op: &str) -> bool {
    adapter(op).is_some()
}
fn invalid() -> OperationError {
    OperationError::new(
        ErrorCode::InvalidInput,
        "use exact logical image inputs and parameters",
    )
}
fn text<'a>(p: &'a Value, key: &str) -> Result<&'a str, OperationError> {
    p[key].as_str().ok_or_else(invalid)
}
fn layout(p: &Value) -> Result<image::RawLayout, OperationError> {
    Ok(image::RawLayout {
        width: field(p, "width")?,
        height: field(p, "height")?,
        layout: image::SampleLayout::parse(text(p, "layout")?)?,
        row_stride: p
            .get("row_stride")
            .map(|_| field(p, "row_stride"))
            .transpose()?,
        offset: field(p, "offset")?,
    })
}
pub(super) fn execute(op: &str, inputs: Value, p: Value) -> Result<Value, OperationError> {
    let identity = cube_core::operations::require(op)?.id;
    let selected = adapter(op).ok_or_else(invalid)?;
    let result = (|| {
        match selected {
            Adapter::CsvImport => {
                return Ok(json!({"output":image(&Image::from_csv(text(&inputs,"input")?)?)}));
            }
            Adapter::Geometry | Adapter::Raw => {
                let data = super::bytes(&inputs["input"])?;
                let layout = layout(&p)?;
                return Ok(match selected {
                    Adapter::Geometry => json!({"output":layout.inspect_bytes(&data)?}),
                    _ => json!({"output":image(&Image::from_raw(&data,&layout)?)}),
                });
            }
            Adapter::Png => {
                return Ok(
                    json!({"output":cube_core::forensics::png::inspect(&mut Cursor::new(super::bytes(&inputs["input"])?),field(&p,"strict")?,field(&p,"payload_budget")?)?}),
                );
            }
            Adapter::Pairs => {
                let values = inputs["images"].as_array().ok_or_else(invalid)?;
                let images: Vec<Image> = values.iter().map(raster).collect::<Result<_, _>>()?;
                let refs: Vec<&Image> = images.iter().collect();
                let operation = image::pairs::PairOperation::parse(text(&p, "operation")?)?;
                let max_pairs = field(&p, "max_pairs")?;
                let max_differences = field(&p, "max_differences")?;
                let batch = if field(&p, "plan_only")? {
                    image::pairs::Batch {
                        plan: image::pairs::plan(&refs, operation, max_pairs, max_differences)?,
                        successes: 0,
                        failures: 0,
                        encoded_bytes: 0,
                        retained_differences: 0,
                        results: vec![],
                    }
                } else {
                    image::pairs::execute(&refs, operation, max_pairs, max_differences)?
                };
                let outputs: Vec<Value> = batch
                    .results
                    .iter()
                    .map(|r| r.png.as_ref().map(|png| bytes(png)).unwrap_or(Value::Null))
                    .collect();
                return Ok(json!({"output":batch,"images":outputs}));
            }
            _ => {}
        }
        let source = raster(
            &inputs[match selected {
                Adapter::Bitwise => "left",
                Adapter::Spatial => "image",
                _ => "input",
            }],
        )?;
        let expand = p
            .get("expand_palette")
            .map(|_| field(&p, "expand_palette"))
            .transpose()?
            .unwrap_or(false);
        let source = source.view(expand)?;
        Ok(match selected {
            Adapter::Bitwise => {
                json!({"output":image(&source.bitwise(&raster(&inputs["right"])?,image::BitwiseOperation::parse(text(&p,"operation")?)?)?)})
            }
            Adapter::CsvExport => json!({"output":source.to_csv()?}),
            Adapter::Qr => {
                let report = source.qr(field(&p, "max_symbols")?)?;
                let payloads: Vec<Value> = report
                    .symbols
                    .iter()
                    .map(|s| s.payload.as_ref().map(|p| bytes(p)).unwrap_or(Value::Null))
                    .collect();
                json!({"output":report,"payloads":payloads})
            }
            Adapter::ColorExports => {
                let colors = p
                    .get("colors")
                    .map(|_| field::<Vec<String>>(&p, "colors"))
                    .transpose()?;
                let colors = colors
                    .map(|v| {
                        v.iter()
                            .map(|s| {
                                cube_core::encoding::hex_decode(s)
                                    .map_err(|e| invalid().caused_by(e))
                            })
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .transpose()?;
                let result = source.color_layers(
                    colors.as_deref(),
                    image::ColorOutput::parse(text(&p, "representation")?)?,
                    field(&p, "max_colors")?,
                    field(&p, "max_outputs")?,
                )?;
                let outputs: Vec<Value> = result.outputs.iter().map(|v| bytes(&v.png)).collect();
                json!({"output":result,"images":outputs})
            }
            Adapter::Colors => {
                let colors: Vec<String> = field(&p, "colors")?;
                let colors = colors
                    .iter()
                    .map(|s| cube_core::encoding::hex_decode(s).map_err(|e| invalid().caused_by(e)))
                    .collect::<Result<Vec<_>, _>>()?;
                let layer = source.exact_colors(
                    &colors,
                    image::ColorOutput::parse(text(&p, "representation")?)?,
                )?;
                json!({"output":layer,"image":image(&layer.image)})
            }
            Adapter::Plane => {
                let plane = source.bit_plane(
                    image::Channel::parse(text(&p, "channel")?)?,
                    field(&p, "bit")?,
                    region(&p)?,
                )?;
                json!({"output":plane,"data":bytes(&plane.packed)})
            }
            Adapter::Lsb => {
                let result = source.lsb_with_corner(
                    &image::LsbOptions {
                        region: region(&p)?,
                        channels: channels(&p)?,
                        bits: field(&p, "bits")?,
                        traversal: image::Traversal::parse(text(&p, "traversal")?)?,
                        reverse: field(&p, "reverse")?,
                        bit_order: image::BitOrder::parse(text(&p, "bit_order")?)?,
                        incomplete: image::Incomplete::parse(text(&p, "incomplete")?)?,
                    },
                    image::Corner::parse(text(&p, "start_corner")?)?,
                )?;
                json!({"output":result,"data":bytes(&result.result.data)})
            }
            Adapter::Spatial => {
                let lags = image::spatial::parse_lags(text(&inputs, "lags")?)?;
                json!({"output":source.autocorrelation(text(&p,"region")?.parse()?,image::Channel::parse(text(&p,"channel")?)?,&lags,image::spatial::Limits { max_work:field(&p,"max_work")?, max_memory_bytes:field(&p,"max_memory_bytes")?, max_report_bytes:field(&p,"max_report_bytes")? })?})
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
        c.validate_parameters(&p).unwrap();
        let result = execute(op, inputs, p)?;
        c.validate_outputs(&result)
            .unwrap_or_else(|e| panic!("{op}: {e}"));
        Ok(result)
    }
    #[test]
    fn eleven_raster_adapters_preserve_real_bytes_and_core_measurements() {
        let source = Image::from_bytes(
            8,
            1,
            image::ColorModel::Gray,
            &[0, 1, 0, 1, 0, 1, 0, 1],
            &[],
            &[],
        )
        .unwrap();
        let logical = image(&source);
        let csv = source.to_csv().unwrap();
        let cases = [
            (
                "image.bit_plane.extract",
                json!({"input":logical}),
                json!({"channel":"gray","bit":0}),
                "/data/base64",
                json!("VQ=="),
            ),
            (
                "image.bitwise",
                json!({"left":logical,"right":logical}),
                json!({"operation":"xor"}),
                "/output/samples/base64",
                json!("AAAAAAAAAAA="),
            ),
            (
                "image.colors.extract",
                json!({"input":logical}),
                json!({"colors":["01"]}),
                "/output/selected_pixels",
                json!(4),
            ),
            (
                "image.csv.export",
                json!({"input":logical}),
                json!({}),
                "/output",
                json!(csv),
            ),
            (
                "image.csv.import",
                json!({"input":csv}),
                json!({}),
                "/output",
                logical.clone(),
            ),
            (
                "image.geometry.inspect",
                json!({"input":bytes(&[0,1,0,1,0,1,0,1])}),
                json!({"width":8,"height":1,"layout":"gray"}),
                "/output/interpretation",
                json!("caller_supplied_hypothesis"),
            ),
            (
                "image.lsb.extract",
                json!({"input":logical}),
                json!({}),
                "/data/base64",
                json!("VQ=="),
            ),
            (
                "image.pairs",
                json!({"images":[logical,logical]}),
                json!({"operation":"xor"}),
                "/output/successes",
                json!(1),
            ),
            (
                "image.png.inspect",
                json!({"input":bytes(&source.encode_png().unwrap())}),
                json!({}),
                "/output/crc_valid",
                json!(true),
            ),
            (
                "image.raw.interpret",
                json!({"input":bytes(&[0,1,0,1,0,1,0,1])}),
                json!({"width":8,"height":1,"layout":"gray"}),
                "/output",
                logical.clone(),
            ),
            (
                "image.spatial.autocorrelation",
                json!({"image":logical,"lags":"[{\"dx\":0,\"dy\":0}]"}),
                json!({"region":"0,0,8,1","channel":"gray"}),
                "/output/measurements/0/correlation",
                json!(1.0),
            ),
        ];
        for (op, i, p, pointer, expected) in cases {
            let actual = run(op, i, p).unwrap_or_else(|e| panic!("{op}: {e}"));
            assert_eq!(actual.pointer(pointer), Some(&expected), "{op}");
        }
        let plan = run(
            "image.pairs",
            json!({"images":[logical,logical]}),
            json!({"operation":"xor","plan_only":true}),
        )
        .unwrap();
        assert_eq!(plan["images"], json!([]));
        assert_eq!(plan["output"]["results"], json!([]));
    }
    #[test]
    fn binary_collections_keep_alignment_and_qr_payloads_are_exact_bytes() {
        let source = Image::from_bytes(2, 1, image::ColorModel::Gray, &[0, 255], &[], &[]).unwrap();
        let layers = run(
            "image.colors.export",
            json!({"input":image(&source)}),
            json!({}),
        )
        .unwrap();
        assert_eq!(layers["images"].as_array().unwrap().len(), 2);
        let first = super::super::bytes(&layers["images"][0]).unwrap();
        assert_eq!(Image::decode(&first).unwrap().samples(), [255, 0]);
        let rows: Vec<_> = include_str!("../../../../../tests/fixtures/qr-binary.matrix")
            .lines()
            .collect();
        let side = (rows.len() + 8) * 2;
        let mut data = vec![255; side * side];
        for (y, row) in rows.iter().enumerate() {
            for (x, b) in row.bytes().enumerate() {
                for dy in 0..2 {
                    for dx in 0..2 {
                        data[((y + 4) * 2 + dy) * side + (x + 4) * 2 + dx] =
                            if b == b'1' { 0 } else { 255 };
                    }
                }
            }
        }
        let qr = Image::from_samples(
            side as u32,
            side as u32,
            image::ColorModel::Gray,
            data,
            vec![],
            vec![],
        )
        .unwrap();
        let decoded = run("image.qr.decode", json!({"input":image(&qr)}), json!({})).unwrap();
        assert_eq!(decoded["payloads"], json!([bytes(b"Cube\0\xff")]));
        let blank =
            Image::from_bytes(16, 16, image::ColorModel::Gray, &[255; 256], &[], &[]).unwrap();
        let empty = run("image.qr.decode", json!({"input":image(&blank)}), json!({})).unwrap();
        assert_eq!(empty["payloads"], json!([]));
        assert_eq!(empty["output"]["status"], "no_detection");
    }
    #[test]
    fn raw_shapes_pair_compatibility_and_lsb_partial_bytes_fail_clearly() {
        assert!(
            run(
                "image.raw.interpret",
                json!({"input":bytes(&[1,2])}),
                json!({"width":2,"height":2,"layout":"gray"})
            )
            .is_err()
        );
        let raster =
            image(&Image::from_bytes(1, 1, image::ColorModel::Gray, &[1], &[], &[]).unwrap());
        assert!(run("image.lsb.extract", json!({"input":raster}), json!({})).is_err());
        assert!(
            run(
                "image.colors.extract",
                json!({"input":raster}),
                json!({"colors":["not-hex"]})
            )
            .is_err()
        );
        assert!(
            run(
                "image.spatial.autocorrelation",
                json!({"image":raster,"lags":"[{\"dx\":0,\"dy\":0}]"}),
                json!({"region":"0,0,1,1","channel":"gray","max_work":0})
            )
            .is_err()
        );
    }
}
