//! Actual Windows acceptance, not a Unix/WSL substitute.
#![cfg(windows)]
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
    sync::mpsc,
    thread,
    time::Duration,
};
#[test]
fn windows_worker_executes_the_real_core_inside_a_required_native_boundary() {
    let binary = env!("CARGO_BIN_EXE_play-engine");
    let mut child = Command::new(binary)
        .args(["--ipc", "--worker", binary])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let output = child.stdout.take().unwrap();
    let (send, recv) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(output).lines() {
            let Ok(line) = line else { break };
            if send
                .send(serde_json::from_str::<Value>(&line).unwrap())
                .is_err()
            {
                break;
            }
        }
    });
    let limits = play_engine::protocol::contract();
    writeln!(input,"{}",json!({"type":"hello","id":1,"protocol":limits.protocol,"engine":limits.engine,"nonce":"0123456789abcdef0123456789abcdef"})).unwrap();
    assert_eq!(
        recv.recv_timeout(Duration::from_secs(10)).unwrap()["type"],
        "ready"
    );
    writeln!(input,"{}",json!({"type":"invoke","id":2,"invocation":{"operation":"cipher.rot13","operation_version":1,"input":"Gur xrl.","parameters":{}}})).unwrap();
    let result = loop {
        let frame = recv.recv_timeout(Duration::from_secs(10));
        match frame {
            Ok(frame) if frame["type"] == "started" => continue,
            other => break other,
        }
    };
    drop(input);
    let _ = child.kill();
    let _ = child.wait();
    let result = result.expect("bounded actual Windows worker response");
    assert_eq!(
        result["status"], "completed",
        "native execution must be available, not an unrestricted fallback: {result}"
    );
    assert_eq!(result["output"]["text"], "The key.");
}
