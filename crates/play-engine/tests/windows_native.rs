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
fn execute_core(clear_environment: bool, worker: &std::path::Path) {
    let binary = env!("CARGO_BIN_EXE_play-engine");
    let mut command = Command::new(binary);
    if clear_environment {
        command.env_clear();
    }
    let mut child = command
        .args(["--ipc", "--worker"])
        .arg(worker)
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

#[test]
fn windows_worker_executes_the_real_core_inside_a_required_native_boundary() {
    execute_core(
        false,
        std::path::Path::new(env!("CARGO_BIN_EXE_play-engine")),
    );
}
#[test]
fn engine_started_with_the_desktop_empty_environment_executes_without_inheriting_secrets() {
    execute_core(
        true,
        std::path::Path::new(env!("CARGO_BIN_EXE_play-engine")),
    );
}

#[test]
fn worker_sources_with_spaces_unicode_and_long_paths_execute_without_changing_bytes() {
    let root = tempfile::Builder::new()
        .prefix("Cube Play ź ")
        .tempdir()
        .unwrap();
    let mut directory = root.path().to_path_buf();
    while directory.as_os_str().len() < 320 {
        directory.push("clue spaces and unicode ł");
        std::fs::create_dir(&directory).unwrap();
    }
    let worker = directory.join("Cube Worker ź.exe");
    std::fs::copy(env!("CARGO_BIN_EXE_play-engine"), &worker).unwrap();
    let before = std::fs::read(&worker).unwrap();
    execute_core(true, &worker);
    assert_eq!(std::fs::read(&worker).unwrap(), before);
}

fn limits() -> play_windows_boundary::Limits {
    let l = play_engine::protocol::contract();
    play_windows_boundary::Limits {
        private_memory_bytes: l.vm_growth_bytes as usize,
        cpu_seconds: l.cpu_seconds,
    }
}
fn probe() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_BIN_EXE_play-engine"))
        .parent()
        .unwrap()
        .join("examples/windows-boundary-probe.exe")
}
fn output(mut child: play_windows_boundary::Child) -> (std::process::ExitStatus, String) {
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        use std::io::Read;
        let mut text = String::new();
        let mut stream = stdout;
        let _ = stream.read_to_string(&mut text);
        let _ = tx.send(text);
    });
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            return (status, rx.recv_timeout(Duration::from_secs(1)).unwrap());
        }
        if std::time::Instant::now() >= deadline {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("bounded native probe did not terminate");
        }
        thread::sleep(Duration::from_millis(10));
    }
}
#[test]
fn appcontainer_denies_private_files_network_and_children_without_changing_sources() {
    let root = tempfile::tempdir().unwrap();
    let secret = root.path().join("private clue ź.txt");
    std::fs::write(&secret, "original private evidence").unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let args = vec![
        std::ffi::OsString::from("boundaries"),
        secret.as_os_str().to_owned(),
        listener.local_addr().unwrap().port().to_string().into(),
    ];
    let child = play_windows_boundary::Child::spawn(&probe(), &args, limits()).unwrap();
    let (status, text) = output(child);
    assert!(status.success(), "{text}");
    let value: Value = serde_json::from_str(&text).unwrap();
    for key in [
        "image_readable",
        "private_read_denied",
        "private_write_denied",
        "network_denied",
        "child_denied",
        "image_directory_write_denied",
    ] {
        assert_eq!(value[key], true, "{key}: {value}");
    }
    assert_eq!(
        std::fs::read_to_string(secret).unwrap(),
        "original private evidence"
    );
}
#[test]
fn windows_job_rejects_excess_private_commit() {
    let (status, text) =
        output(play_windows_boundary::Child::spawn(&probe(), &["memory"], limits()).unwrap());
    assert!(status.success(), "{text}");
    assert_eq!(text.trim(), "true");
}
#[test]
fn windows_job_stops_cpu_loops() {
    let (status, _) =
        output(play_windows_boundary::Child::spawn(&probe(), &["cpu"], limits()).unwrap());
    assert!(!status.success());
}
#[test]
fn launching_a_worker_without_the_native_launcher_refuses_before_reading_input() {
    let output = Command::new(env!("CARGO_BIN_EXE_play-engine"))
        .arg("--worker")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("native_policy_unavailable"));
}

struct HostProcess(std::process::Child);
impl Drop for HostProcess {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
fn line(stream: impl std::io::Read + Send + 'static) -> String {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut value = String::new();
        let result = BufReader::new(stream).read_line(&mut value);
        let _ = tx.send(result.map(|_| value));
    });
    rx.recv_timeout(Duration::from_secs(10)).unwrap().unwrap()
}
#[test]
fn killing_the_owner_reaps_a_worker_that_does_not_observe_stdin() {
    let mut owner = HostProcess(
        Command::new(probe())
            .arg("owner")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    let pid: u32 = line(owner.0.stdout.take().unwrap()).trim().parse().unwrap();
    // Hold the exact live process handle before killing its owner, avoiding a PID-only check.
    let script = format!(
        "$ErrorActionPreference='Stop'; $p=[Diagnostics.Process]::GetProcessById({pid}); $held=$p.Handle; [Console]::WriteLine('watching'); if($p.WaitForExit(5000)){{exit 0}}else{{exit 1}}"
    );
    let mut watcher = HostProcess(
        Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command", &script])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .spawn()
            .unwrap(),
    );
    assert_eq!(line(watcher.0.stdout.take().unwrap()).trim(), "watching");
    owner.0.kill().unwrap();
    owner.0.wait().unwrap();
    assert!(
        watcher.0.wait().unwrap().success(),
        "kernel job must reap an orphaned worker"
    );
}

#[test]
fn another_live_contained_worker_cannot_keep_a_private_input_pipe_open() {
    let mut first = play_windows_boundary::Child::spawn(&probe(), &["echo"], limits()).unwrap();
    let mut second = play_windows_boundary::Child::spawn(&probe(), &["idle"], limits()).unwrap();
    let pid: u32 = line(second.stdout.take().unwrap()).trim().parse().unwrap();
    assert!(pid > 0);
    first
        .stdin
        .as_mut()
        .unwrap()
        .write_all(b"first private worker input")
        .unwrap();
    drop(first.stdin.take());
    let (status, text) = output(first);
    assert!(status.success());
    assert_eq!(text, "first private worker input");
    assert!(
        second.try_wait().unwrap().is_none(),
        "the independent worker stays alive"
    );
}
