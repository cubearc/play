use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
};

#[test]
fn negotiates_the_private_protocol_before_accepting_commands() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_play-engine"))
        .arg("--ipc")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    writeln!(input, r#"{{"type":"hello","id":1,"protocol":3,"engine":"0.2.0","nonce":"0123456789abcdef0123456789abcdef"}}"#).unwrap();
    let mut line = String::new();
    output.read_line(&mut line).unwrap();
    let value: serde_json::Value = serde_json::from_str(&line).unwrap();
    drop(input);
    child.wait().unwrap();
    assert_eq!(
        value["type"], "ready",
        "the desktop must negotiate its exact engine before invocation"
    );
    assert_eq!(value["nonce"], "0123456789abcdef0123456789abcdef");
}

#[test]
fn rejects_unscoped_execution_and_replayed_ids_without_exposing_input() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_play-engine"))
        .arg("--ipc")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = BufReader::new(child.stdout.take().unwrap());
    let mut request = |value: serde_json::Value| {
        writeln!(input, "{value}").unwrap();
        let mut line = String::new();
        output.read_line(&mut line).unwrap();
        assert!(!line.contains("private-input"));
        serde_json::from_str::<serde_json::Value>(&line).unwrap()
    };
    use serde_json::json;
    assert_eq!(
        request(json!({"type":"hello","id":1,"protocol":3,"engine":"0.2.0",
        "nonce":"0123456789abcdef0123456789abcdef"}))["type"],
        "ready"
    );
    let catalog = request(json!({"type":"catalog","id":2}));
    assert!(
        catalog["operations"]
            .as_array()
            .unwrap()
            .iter()
            .any(|entry| entry["operation"] == "esolang.brainfuck.execute"
                && entry["operation_version"] == 2)
    );
    for (id, operation, version, expected) in [
        (3, "cipher.rot13", 99, "contract_mismatch"),
        (4, "file.read", 1, "unsupported_operation"),
        (5, "cipher.rot13", 1, "native_policy_unavailable"),
    ] {
        assert_eq!(
            request(json!({"type":"invoke","id":id,"invocation":{
            "operation":operation,"operation_version":version,"input":"private-input"}}))["code"],
            expected
        );
    }
    assert_eq!(
        request(json!({"type":"cancel","id":6,"run_id":5}))["code"],
        "not_running"
    );
    assert_eq!(
        request(json!({"type":"catalog","id":6}))["code"],
        "invalid_request_id"
    );
    assert_eq!(request(json!({"type":"catalog","id":7}))["type"], "catalog");
    drop(request);
    drop(input);
    assert!(child.wait().unwrap().success());
}
