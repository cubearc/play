//! Existing Core lexical searches, scalar traversals and explicit text selections.
use super::field;
use cube_core::{
    errors::{ErrorCode, OperationError},
    text::{
        anagram, combinations, grid, lexical, multiset, positions, statistics, transform,
        traversal, word, word_ladder,
    },
};
use serde::Serialize;
use serde_json::{Value, json};

enum Adapter {
    Anagram,
    Combinations,
    Josephus,
    Compare,
    Subtract,
    Keyed,
    Masked,
    Words,
    Shortest,
    Validate,
    Grid,
}
fn adapter(op: &str) -> Option<Adapter> {
    Some(match op {
        "text.anagram.search" => Adapter::Anagram,
        "text.combinations" => Adapter::Combinations,
        "text.josephus.extract" => Adapter::Josephus,
        "text.multiset.compare" => Adapter::Compare,
        "text.multiset.subtract" => Adapter::Subtract,
        "text.skip.keyed" => Adapter::Keyed,
        "text.transform.masked" => Adapter::Masked,
        "text.word.extract" => Adapter::Words,
        "text.word_ladder.shortest" => Adapter::Shortest,
        "text.word_ladder.validate" => Adapter::Validate,
        "text.grid.render" => Adapter::Grid,
        _ => return None,
    })
}
pub(super) fn supported(op: &str) -> bool {
    adapter(op).is_some()
}
fn invalid() -> OperationError {
    OperationError::new(
        ErrorCode::InvalidParameter,
        "use the exact text inputs and explicit selection fields",
    )
}
fn string<'a>(v: &'a Value, name: &str) -> Result<&'a str, OperationError> {
    v.get(name).and_then(Value::as_str).ok_or_else(invalid)
}
fn unsigned(v: &Value) -> Result<u64, OperationError> {
    v.as_u64()
        .or_else(|| v.as_str()?.parse().ok())
        .ok_or_else(invalid)
}
fn optional_number(p: &Value, name: &str) -> Result<Option<u64>, OperationError> {
    p.get(name).map(unsigned).transpose()
}
fn optional_size(p: &Value, name: &str) -> Result<Option<usize>, OperationError> {
    optional_number(p, name)?
        .map(|v| usize::try_from(v).map_err(|_| invalid()))
        .transpose()
}
fn sizes(p: &Value, name: &str) -> Result<Option<Vec<usize>>, OperationError> {
    p.get(name)
        .map(|v| {
            v.as_array()
                .ok_or_else(invalid)?
                .iter()
                .map(|v| usize::try_from(unsigned(v)?).map_err(|_| invalid()))
                .collect()
        })
        .transpose()
}
fn profile(p: &Value) -> Result<multiset::Profile, OperationError> {
    Ok(multiset::Profile {
        normalization: statistics::Normalization::parse(string(p, "normalization")?)?,
        case: statistics::CaseMode::parse(string(p, "case")?)?,
        filter: p
            .get("filter")
            .map(|_| statistics::TextFilter::parse(string(p, "filter")?))
            .transpose()?
            .unwrap_or(statistics::TextFilter::All),
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
    let result = (|| match selected {
        Adapter::Grid => {
            let grid = grid::render(
                string(&inputs, "input")?,
                field(&p, "width")?,
                field(&p, "drop_whitespace")?,
                cube_core::image::Traversal::parse(string(&p, "traversal")?)?,
            )?;
            let mut result = json!({"output":grid});
            if let Some(colors) = p.get("colors") {
                let colors = grid::parse_colors(colors.as_str().ok_or_else(invalid)?)?;
                result["image"] = super::images::image(&grid.image(&colors)?);
            }
            Ok(result)
        }
        Adapter::Combinations => output(
            combinations::Combinations::new(
                field(&inputs, "input")?,
                field(&p, "separator")?,
                field(&p, "max_results")?,
            )?
            .collect::<Vec<_>>(),
        ),
        Adapter::Compare | Adapter::Subtract => {
            let limits = multiset::Limits {
                max_work: field(&p, "max_work")?,
                max_memory_bytes: field(&p, "max_memory_bytes")?,
                max_report_bytes: field(&p, "max_report_bytes")?,
            };
            if matches!(selected, Adapter::Compare) {
                output(multiset::compare(
                    string(&inputs, "input")?,
                    string(&p, "requested")?,
                    profile(&p)?,
                    limits,
                )?)
            } else {
                output(multiset::subtract(
                    string(&inputs, "input")?,
                    string(&p, "requested")?,
                    profile(&p)?,
                    limits,
                )?)
            }
        }
        Adapter::Anagram => {
            let dictionary = lexical::parse_dictionary(
                string(&inputs, "dictionary")?,
                anagram::MAX_DICTIONARY_BYTES,
                anagram::MAX_ENTRIES,
            )?;
            output(anagram::search(
                string(&inputs, "input")?,
                &dictionary,
                profile(&p)?,
                anagram::Options {
                    max_words: field(&p, "max_words")?,
                    reuse: field(&p, "reuse")?,
                    max_results: field(&p, "max_results")?,
                    max_work: field(&p, "max_work")?,
                    max_memory_bytes: field(&p, "max_memory_bytes")?,
                    max_report_bytes: field(&p, "max_report_bytes")?,
                },
            )?)
        }
        Adapter::Shortest | Adapter::Validate => {
            let dictionary = lexical::parse_dictionary(
                string(&inputs, "dictionary")?,
                word_ladder::MAX_DICTIONARY_BYTES,
                anagram::MAX_ENTRIES,
            )?;
            let options = word_ladder::Options {
                max_work: field(&p, "max_work")?,
                max_memory_bytes: field(&p, "max_memory_bytes")?,
                max_report_bytes: field(&p, "max_report_bytes")?,
            };
            if matches!(selected, Adapter::Shortest) {
                output(word_ladder::shortest(
                    &dictionary,
                    string(&inputs, "start")?,
                    string(&inputs, "end")?,
                    profile(&p)?,
                    options,
                )?)
            } else {
                let path = lexical::parse_dictionary(
                    string(&inputs, "path")?,
                    word_ladder::MAX_DICTIONARY_BYTES,
                    cube_core::graph::MAX_VERTICES,
                )?;
                output(word_ladder::validate(
                    &dictionary,
                    &path,
                    profile(&p)?,
                    options,
                )?)
            }
        }
        Adapter::Josephus | Adapter::Keyed => {
            let steps = p["steps"]
                .as_array()
                .ok_or_else(invalid)?
                .iter()
                .map(unsigned)
                .collect::<Result<Vec<_>, _>>()?;
            let options = traversal::Options {
                start: optional_number(&p, "start")?,
                index_base: field(&p, "index_base")?,
                direction: field(&p, "direction")?,
                wrap: field(&p, "wrap")?,
                max_work: field(&p, "max_work")?,
                max_memory: field(&p, "max_memory")?,
                max_output: field(&p, "max_output")?,
            };
            if matches!(selected, Adapter::Keyed) {
                output(traversal::keyed(
                    string(&inputs, "input")?,
                    &steps,
                    field(&p, "count")?,
                    &options,
                )?)
            } else {
                output(traversal::josephus(
                    string(&inputs, "input")?,
                    &steps,
                    optional_size(&p, "count")?,
                    &options,
                )?)
            }
        }
        Adapter::Masked => {
            let selector = positions::Selector::from_options(
                string(&p, "selector")?,
                sizes(&p, "indices")?,
                optional_size(&p, "nth")?,
                sizes(&p, "exclude")?,
            )?;
            let overrides: Vec<String> = p
                .get("overrides")
                .map(|_| field(&p, "overrides"))
                .transpose()?
                .unwrap_or_default();
            let config = transform::Config {
                index_base: positions::IndexBase::try_from(field::<u8>(&p, "index_base")?)?,
                selector,
                selected: transform::Transformation::parse(
                    string(&p, "selected")?,
                    p.get("selected_shift")
                        .map(|_| field(&p, "selected_shift"))
                        .transpose()?,
                )?,
                otherwise: transform::Transformation::parse(
                    string(&p, "otherwise")?,
                    p.get("otherwise_shift")
                        .map(|_| field(&p, "otherwise_shift"))
                        .transpose()?,
                )?,
                overrides: overrides
                    .iter()
                    .map(|s| transform::Override::parse(s))
                    .collect::<Result<Vec<_>, _>>()?,
            };
            output(transform::masked(string(&inputs, "input")?, &config)?.final_text)
        }
        Adapter::Words => {
            let shared = p
                .get("positions")
                .map(|_| word::parse_positions(string(&p, "positions")?))
                .transpose()?;
            let per_word = p
                .get("per_word")
                .map(|_| word::parse_per_word(string(&p, "per_word")?))
                .transpose()?;
            let positions = match (shared.as_deref(), per_word.as_deref()) {
                (Some(p), None) => word::Positions::Shared(p),
                (None, Some(p)) => word::Positions::PerWord(p),
                _ => return Err(invalid()),
            };
            output(word::extract(
                string(&inputs, "input")?,
                positions,
                field(&p, "index_base")?,
                string(&p, "short_word")?,
            )?)
        }
    })();
    result.map_err(|e: OperationError| e.operation(identity))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn defaults(op: &str, mut p: Value) -> Value {
        let c = cube_core::operations::require(op)
            .unwrap()
            .contract()
            .unwrap();
        for (k, v) in c.parameters.as_value()["properties"].as_object().unwrap() {
            if let Some(v) = v.get("default") {
                p.as_object_mut()
                    .unwrap()
                    .entry(k)
                    .or_insert_with(|| v.clone());
            }
        }
        p
    }
    #[test]
    fn ten_text_adapters_preserve_known_scalars_reports_and_explicit_models() {
        let dictionary = "[\"CAT\",\"COT\",\"COG\",\"DOG\"]";
        let cases = [
            (
                "text.combinations",
                json!({"input":[["A","B"],["1","2"]]}),
                json!({"separator":"-"}),
                "/output",
                json!(["A-1", "A-2", "B-1", "B-2"]),
            ),
            (
                "text.anagram.search",
                json!({"input":"AB","dictionary":"[\"AB\"]"}),
                json!({"max_words":1}),
                "/output/solutions",
                json!([[0]]),
            ),
            (
                "text.josephus.extract",
                json!({"input":"ABCDE"}),
                json!({"steps":[1],"count":1}),
                "/output/extracted",
                json!("A"),
            ),
            (
                "text.skip.keyed",
                json!({"input":"ABCDE"}),
                json!({"steps":[1],"count":1}),
                "/output/extracted",
                json!("A"),
            ),
            (
                "text.multiset.compare",
                json!({"input":"BABA"}),
                json!({"requested":"AB"}),
                "/output/residual",
                json!([{"scalar":"A","count":1},{"scalar":"B","count":1}]),
            ),
            (
                "text.multiset.subtract",
                json!({"input":"BABA"}),
                json!({"requested":"AB"}),
                "/output/residual",
                json!([{"scalar":"A","count":1},{"scalar":"B","count":1}]),
            ),
            (
                "text.transform.masked",
                json!({"input":"ABCD"}),
                json!({"selector":"indices","indices":[0,2],"selected":"atbash"}),
                "/output",
                json!("ZBXD"),
            ),
            (
                "text.word.extract",
                json!({"input":"Cat Dog"}),
                json!({"positions":"[1,-1]"}),
                "/output/extracted",
                json!("CtDg"),
            ),
            (
                "text.word_ladder.shortest",
                json!({"dictionary":dictionary,"start":"CAT","end":"DOG"}),
                json!({}),
                "/output/valid",
                json!(true),
            ),
            (
                "text.word_ladder.validate",
                json!({"dictionary":dictionary,"path":dictionary}),
                json!({}),
                "/output/valid",
                json!(true),
            ),
        ];
        for (op, inputs, p, pointer, expected) in cases {
            assert!(supported(op));
            let contract = cube_core::operations::require(op)
                .unwrap()
                .contract()
                .unwrap();
            let p = defaults(op, p);
            contract
                .validate_inputs(&inputs)
                .unwrap_or_else(|e| panic!("{op} inputs: {e}"));
            contract.validate_parameters(&p).unwrap();
            let actual = execute(op, inputs, p).unwrap_or_else(|e| panic!("{op} execution: {e}"));
            contract
                .validate_outputs(&actual)
                .unwrap_or_else(|e| panic!("{op} outputs: {e}"));
            assert_eq!(actual.pointer(pointer), Some(&expected), "{op}");
            if op == "text.word_ladder.shortest" {
                let path = actual["output"]["path"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|i| {
                        actual["output"]["dictionary"][i.as_u64().unwrap() as usize]["text"]
                            .as_str()
                            .unwrap()
                    })
                    .collect::<Vec<_>>();
                assert_eq!(path, vec!["CAT", "COT", "COG", "DOG"]);
            }
        }
        assert!(!supported("unknown.operation"));
    }
    #[test]
    fn scalar_grid_keeps_text_without_mapping_and_renders_only_explicit_colors() {
        let p = defaults("text.grid.render", json!({"width":2}));
        let text = execute("text.grid.render", json!({"input":"0110"}), p.clone()).unwrap();
        assert_eq!(text["output"]["rows"], json!([["0", "1"], ["1", "0"]]));
        assert!(text.get("image").is_none());
        let mut p = p;
        p["colors"] = json!(r#"[["0","000000ff"],["1","ffffffff"]]"#);
        let output = execute("text.grid.render", json!({"input":"0110"}), p.clone()).unwrap();
        cube_core::operations::require("text.grid.render")
            .unwrap()
            .contract()
            .unwrap()
            .validate_outputs(&output)
            .unwrap();
        assert_eq!(
            super::super::images::raster(&output["image"])
                .unwrap()
                .samples(),
            [
                0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 255
            ]
        );
        p["colors"] = json!(r#"[["0","000000ff"]]"#);
        assert!(execute("text.grid.render", json!({"input":"0110"}), p).is_err());
    }
    #[test]
    fn invalid_selections_and_resource_exhaustion_fail_without_partial_results() {
        for (op, inputs, p) in [
            (
                "text.combinations",
                json!({"input":[["A","B"]]}),
                json!({"max_results":1}),
            ),
            (
                "text.word.extract",
                json!({"input":"ABC"}),
                json!({"positions":"[1]","per_word":"[[1]]"}),
            ),
            (
                "text.skip.keyed",
                json!({"input":"ABC"}),
                json!({"steps":[1],"count":1,"max_work":0}),
            ),
            (
                "text.anagram.search",
                json!({"input":"AB","dictionary":"[\"AB\"]"}),
                json!({"max_work":0}),
            ),
            (
                "text.transform.masked",
                json!({"input":"ABC"}),
                json!({"selector":"indices","indices":[99],"selected":"identity"}),
            ),
        ] {
            assert!(execute(op, inputs, defaults(op, p)).is_err(), "{op}");
        }
        let p = defaults(
            "text.skip.keyed",
            json!({"steps":["18446744073709551615"],"count":1}),
        );
        let c = cube_core::operations::require("text.skip.keyed")
            .unwrap()
            .contract()
            .unwrap();
        c.validate_parameters(&p).unwrap();
        assert!(execute("text.skip.keyed", json!({"input":"ABC"}), p).is_ok());
    }
}
