use serde_json::{Value, json};
use std::{
    collections::BTreeSet,
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
};

#[test]
fn complete_catalog_is_bounded_paged_and_explicit_about_unavailable_execution() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_play-engine"))
        .arg("--ipc")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let mut request = |value: Value| {
        writeln!(input, "{value}").unwrap();
        let mut line = String::new();
        output.read_line(&mut line).unwrap();
        assert!(line.len() <= play_engine::protocol::contract().frame_bytes);
        serde_json::from_str::<Value>(&line).unwrap()
    };
    let contract = play_engine::protocol::contract();
    assert_eq!(
        request(
            json!({"type":"hello","id":1,"protocol":contract.protocol,"engine":contract.engine,"nonce":"0123456789abcdef0123456789abcdef"})
        )["type"],
        "ready"
    );
    let mut offset = 0;
    let mut id = 1;
    let mut ids = BTreeSet::new();
    loop {
        id += 1;
        let page = request(json!({"type":"catalog","id":id,"offset":offset}));
        assert_eq!(page["type"], "catalog", "{page}");
        assert_eq!(page["offset"], offset);
        let entries = page["operations"].as_array().unwrap();
        assert!(!entries.is_empty());
        for entry in entries {
            assert!(ids.insert(entry["operation"].as_str().unwrap().to_owned()));
            assert!(matches!(
                entry["availability"]["state"].as_str(),
                Some("available" | "unavailable")
            ));
        }
        offset += entries.len();
        if page["nextOffset"].is_null() {
            assert_eq!(page["total"], offset);
            break;
        }
        assert_eq!(page["nextOffset"], offset);
    }
    assert_eq!(
        ids,
        cube_core::operations::execution::catalog(Default::default())
            .unwrap()
            .iter()
            .map(|d| d.operation.to_owned())
            .collect()
    );
    id += 1;
    assert_eq!(
        request(json!({"type":"catalog","id":id,"offset":offset + 1}))["code"],
        "invalid_catalog_offset"
    );
    id += 1;
    assert_eq!(
        request(json!({"type":"catalog","id":id,"offset":0}))["type"],
        "catalog"
    );
    drop(request);
    drop(input);
    assert!(child.wait().unwrap().success());
}
