//! Logical raster adapters. Core owns decoding, exact samples and transformations.
use cube_core::{
    errors::{ErrorCode, OperationError},
    image::{Channel, ColorModel, Corner, Image, Region, Traversal},
    operations::contracts::values,
};
use serde_json::{Value, json};

pub(super) fn supported(operation: &str) -> bool {
    super::super::execute::direct(operation)
}
fn invalid() -> OperationError {
    OperationError::new(
        ErrorCode::InvalidInput,
        "use the exact logical raster fields and parameters",
    )
}
fn raster(value: &Value) -> Result<Image, OperationError> {
    let model: String = super::field(value, "color_model")?;
    Image::from_samples(
        super::field(value, "width")?,
        super::field(value, "height")?,
        ColorModel::parse(&model)?,
        super::bytes(&value["samples"])?,
        super::field(value, "palette")?,
        super::bytes(&value["transparency"])?,
    )
}
fn image(value: &Image) -> Value {
    json!(values::Image::from(value))
}
fn bytes(value: &[u8]) -> Value {
    json!({"base64":cube_core::encoding::base64_encode(value)})
}
fn region(p: &Value) -> Result<Option<Region>, OperationError> {
    let Some(value) = p.get("region") else {
        return Ok(None);
    };
    let [x, y, width, height]: [u32; 4] =
        serde_json::from_value(value.clone()).map_err(|_| invalid())?;
    Ok(Some(Region {
        x,
        y,
        width,
        height,
    }))
}
fn channels(p: &Value) -> Result<Option<Vec<Channel>>, OperationError> {
    p.get("channels")
        .map(|value| {
            let names: Vec<String> =
                serde_json::from_value(value.clone()).map_err(|_| invalid())?;
            names.iter().map(|name| Channel::parse(name)).collect()
        })
        .transpose()
}
pub(super) fn execute(op: &str, inputs: Value, p: Value) -> Result<Value, OperationError> {
    if op == "image.decode" {
        return Ok(json!({"output":image(&Image::decode(&super::bytes(&inputs["input"] )?)?)}));
    }
    let source = raster(
        &inputs[if op == "image.compare" {
            "left"
        } else {
            "input"
        }],
    )?;
    let expand = p
        .get("expand_palette")
        .map(|_| super::field::<bool>(&p, "expand_palette"))
        .transpose()?
        .unwrap_or(false);
    let source = source.view(expand)?;
    let channels = channels(&p)?;
    let channels = channels.as_deref();
    let region = region(&p)?;
    let output = match op {
        "image.encode" => bytes(&source.encode_png()?),
        "image.palette.expand" => image(&source.expand_palette()?),
        "image.pixels.read" => {
            bytes(&source.pixel(super::field(&p, "x")?, super::field(&p, "y")?)?)
        }
        "image.histogram" => {
            json!(source.histogram(region, channels, super::field(&p, "max_colors")?)?)
        }
        "image.pixels.export" => {
            let traversal: String = super::field(&p, "traversal")?;
            let corner: String = super::field(&p, "start_corner")?;
            let pixels = source.pixels_with_traversal(
                region,
                channels,
                Traversal::parse(&traversal)?,
                Corner::parse(&corner)?,
                super::field(&p, "reverse")?,
            )?;
            return Ok(json!({"output":pixels,"data":bytes(&pixels.pixels.data)}));
        }
        "image.channel.extract" => {
            let channel: String = super::field(&p, "channel")?;
            let pixels = source.channel(Channel::parse(&channel)?, region)?;
            return Ok(
                json!({"output":pixels,"data":bytes(&pixels.data),"image":image(&pixels.grayscale_image()?)}),
            );
        }
        "image.compare" => {
            let other = raster(&inputs["right"])?;
            let comparison = source.compare(
                &other,
                region,
                channels,
                super::field(&p, "max_results")?,
                super::field(&p, "mask")?,
            )?;
            let mut output = json!({"output":comparison});
            if let Some(mask) = &comparison.mask {
                output["mask"] = image(mask);
            }
            return Ok(output);
        }
        "image.rotate" => image(&source.rotate(super::field(&p, "degrees")?)?),
        "image.mirror" => {
            let axis: String = super::field(&p, "axis")?;
            image(&source.mirror(&axis)?)
        }
        "image.crop" => image(&source.crop(region.ok_or_else(invalid)?)?),
        "image.invert" => image(&source.invert(channels)?),
        "image.reorder" => image(&source.reorder(channels.ok_or_else(invalid)?)?),
        "image.grayscale" => image(&source.grayscale()?),
        _ => return Err(invalid()),
    };
    Ok(json!({"output":output}))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn every_existing_image_capability_executes_with_exact_named_outputs() {
        let source =
            Image::from_bytes(2, 1, ColorModel::Rgb, &[1, 2, 3, 10, 20, 30], &[], &[]).unwrap();
        let logical = image(&source);
        let indexed = Image::from_bytes(
            2,
            1,
            ColorModel::Indexed,
            &[0, 1],
            &[[1, 2, 3], [10, 20, 30]],
            &[],
        )
        .unwrap();
        let cases = [
            (
                "image.decode",
                json!({"input":bytes(&source.encode_png().unwrap())}),
                json!({}),
            ),
            ("image.encode", json!({"input":logical}), json!({})),
            (
                "image.palette.expand",
                json!({"input":image(&indexed)}),
                json!({}),
            ),
            (
                "image.pixels.read",
                json!({"input":logical}),
                json!({"x":1,"y":0}),
            ),
            ("image.pixels.export", json!({"input":logical}), json!({})),
            (
                "image.channel.extract",
                json!({"input":logical}),
                json!({"channel":"red"}),
            ),
            ("image.histogram", json!({"input":logical}), json!({})),
            (
                "image.compare",
                json!({"left":logical,"right":logical}),
                json!({"mask":true}),
            ),
            (
                "image.rotate",
                json!({"input":logical}),
                json!({"degrees":90}),
            ),
            (
                "image.mirror",
                json!({"input":logical}),
                json!({"axis":"horizontal"}),
            ),
            (
                "image.crop",
                json!({"input":logical}),
                json!({"region":[0,0,1,1]}),
            ),
            ("image.invert", json!({"input":logical}), json!({})),
            (
                "image.reorder",
                json!({"input":logical}),
                json!({"channels":["blue","green","red"]}),
            ),
            ("image.grayscale", json!({"input":logical}), json!({})),
        ];
        let tested = cases.iter().map(|c| c.0).collect::<BTreeSet<_>>();
        let advertised = cube_core::operations::catalog()
            .into_iter()
            .filter(|d| supported(d.id))
            .map(|d| d.id)
            .collect::<BTreeSet<_>>();
        assert_eq!(tested, advertised);
        for (operation, inputs, parameters) in cases {
            let worker=serde_json::from_value(json!({"operation":operation,"operation_version":1,"inputs":inputs,"parameters":parameters})).unwrap();
            let result = super::super::super::typed::run(worker).unwrap();
            let outputs: Value = serde_json::from_slice(
                &result
                    .data
                    .bytes(super::super::super::types::MAX_OUTPUT)
                    .unwrap(),
            )
            .unwrap();
            let output = &outputs["output"];
            match operation {
                "image.decode" => assert_eq!(output, &logical),
                "image.palette.expand" => assert_eq!(
                    raster(output).unwrap().samples(),
                    [1, 2, 3, 255, 10, 20, 30, 255]
                ),
                "image.encode" => assert_eq!(
                    Image::decode(&super::super::bytes(output).unwrap())
                        .unwrap()
                        .samples(),
                    source.samples()
                ),
                "image.pixels.read" => {
                    assert_eq!(super::super::bytes(output).unwrap(), [10, 20, 30])
                }
                "image.pixels.export" => assert_eq!(
                    super::super::bytes(&outputs["data"]).unwrap(),
                    source.samples()
                ),
                "image.channel.extract" => {
                    assert_eq!(super::super::bytes(&outputs["data"]).unwrap(), [1, 10]);
                    assert_eq!(raster(&outputs["image"]).unwrap().samples(), [1, 10]);
                }
                "image.histogram" => {
                    assert_eq!(output["pixels"], 2);
                    assert_eq!(output["colors"].as_array().unwrap().len(), 2);
                }
                "image.compare" => {
                    assert_eq!(output["equal"], true);
                    assert_eq!(raster(&outputs["mask"]).unwrap().samples(), [0, 0]);
                }
                "image.rotate" => {
                    assert_eq!(output["width"], 1);
                    assert_eq!(output["height"], 2);
                }
                "image.mirror" => {
                    assert_eq!(raster(output).unwrap().samples(), [10, 20, 30, 1, 2, 3])
                }
                "image.crop" => assert_eq!(raster(output).unwrap().samples(), [1, 2, 3]),
                "image.invert" => assert_eq!(
                    raster(output).unwrap().samples(),
                    [254, 253, 252, 245, 235, 225]
                ),
                "image.reorder" => {
                    assert_eq!(raster(output).unwrap().samples(), [3, 2, 1, 30, 20, 10])
                }
                "image.grayscale" => assert_eq!(raster(output).unwrap().samples(), [2, 18]),
                _ => panic!("untested advertised capability"),
            }
        }
    }
}
