use play_launcher::process::HostProcess;
use serde_json::{Value, json};
use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, atomic::AtomicBool},
    time::{Duration, Instant},
};

fn compile_fixture(target: &Path) {
    let deps = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let library = fs::read_dir(&deps)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .find(|path| {
            path.file_name().is_some_and(|name| {
                name.to_string_lossy().starts_with("libserde_json-")
                    && name.to_string_lossy().ends_with(".rlib")
            })
        })
        .unwrap();
    let mut command = Command::new("rustc");
    command
        .args(["--edition=2024", "--crate-name", "cube_play_docker_fixture"])
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support/docker_fixture.rs"))
        .arg("--extern")
        .arg(format!("serde_json={}", library.display()))
        .arg("-L")
        .arg(format!("dependency={}", deps.display()))
        .arg("-o")
        .arg(target);
    if std::env::var_os("LLVM_PROFILE_FILE").is_some() {
        command.arg("-Cinstrument-coverage");
    }
    assert!(command.status().unwrap().success());
}

fn launch(fixture: &Path, home: &Path, workspace: &Path, executable: &Path) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_cube-play"));
    command
        .current_dir(workspace)
        .env("CUBE_PLAY_HOME", home)
        .env("CUBE_PLAY_DOCKER", executable)
        .env("CUBE_PLAY_IMAGE", "cube-play-local:0.1.0")
        .env(
            "CUBE_PLAY_FIXTURE_USER",
            play_launcher::docker::runtime_user().unwrap(),
        )
        .env("CUBE_PLAY_FIXTURE_DIR", fixture)
        .env("CUBE_PLAY_FIXTURE_SOURCE", workspace)
        .env_remove("DOCKER_HOST")
        .env_remove("DOCKER_CONTEXT");
    command
}

fn error(command: &mut Command, code: &str) {
    let output = command.output().unwrap();
    assert!(!output.status.success(), "unexpected success");
    let diagnostic: Value = serde_json::from_slice(&output.stderr).unwrap_or_else(|_| {
        panic!(
            "unexpected diagnostics: {}",
            String::from_utf8_lossy(&output.stderr)
        )
    });
    assert_eq!(diagnostic["error"]["code"], code);
}

#[test]
fn launch_contract_reuse_recovery_and_security_failures() {
    let directory = tempfile::tempdir().unwrap();
    let directory = directory.path().canonicalize().unwrap();
    let fixture = directory.join("fixtures");
    fs::create_dir(&fixture).unwrap();
    let source = directory.join("files $private 空");
    fs::create_dir(&source).unwrap();
    let home = directory.join("state $local");
    let executable = directory.join(format!("docker-fixture{}", std::env::consts::EXE_SUFFIX));
    compile_fixture(&executable);
    let protocol = Command::new(env!("CARGO_BIN_EXE_cube-play"))
        .arg("--launcher-protocol")
        .output()
        .unwrap();
    assert_eq!(protocol.stdout, b"cube-play-launcher 1\n");
    for (mode, code) in [
        ("remote", "remote_docker"),
        ("daemon-off", "docker_unavailable"),
        ("windows-engine", "unsupported_runtime"),
        ("old-compose", "compose_unavailable"),
        ("bad-image", "incompatible_runtime"),
        ("old-runtime", "incompatible_runtime"),
        ("no-landlock", "unsupported_isolation"),
    ]
    .into_iter()
    .chain(if cfg!(unix) {
        Some(("socket-exposure", "unsafe_workspace"))
    } else {
        None
    }) {
        fs::write(fixture.join("mode"), mode).unwrap();
        error(
            launch(&fixture, &home, &source, &executable).args(["--json-errors", "--no-browser"]),
            code,
        );
    }
    fs::write(fixture.join("mode"), "missing-image").unwrap();
    error(
        launch(&fixture, &home, &source, &executable).args([
            "--json-errors",
            "--no-browser",
            "--offline",
        ]),
        "runtime_missing",
    );
    // Building is a host installation action against trusted source, not an investigation command.
    let install = launch(&fixture, &home, &source, &executable)
        .arg("--install-runtime")
        .arg("--runtime-source")
        .arg(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .output()
        .unwrap();
    assert!(
        install.status.success(),
        "{}",
        String::from_utf8_lossy(&install.stderr)
    );
    fs::write(fixture.join("mode"), "startup-failure").unwrap();
    error(
        launch(&fixture, &home, &source, &executable).args(["--json-errors", "--no-browser"]),
        "startup_failed",
    );
    fs::write(fixture.join("mode"), "").unwrap();
    let first = launch(&fixture, &home, &source, &executable)
        .args(["--no-browser", "--offline"])
        .output()
        .unwrap();
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    let first = String::from_utf8(first.stdout).unwrap();
    assert!(first.contains("Open Cube Play: http://127.0.0.1:"));
    let registry: Value =
        serde_json::from_slice(&fs::read(home.join("workspaces.json")).unwrap()).unwrap();
    let id = registry["workspaces"][0]["id"].as_str().unwrap();
    let context = home.join(id.replace('-', ""));
    let compose: Value =
        serde_json::from_slice(&fs::read(context.join("compose.json")).unwrap()).unwrap();
    assert_eq!(
        compose["services"]["play-local"]["volumes"][0]["source"],
        play_launcher::state::docker_path(&source)
            .unwrap()
            .replace('$', "$$")
    );
    let before = fs::read(fixture.join("events")).unwrap();
    let context_modified = fs::metadata(context.join("workspace.json"))
        .unwrap()
        .modified()
        .unwrap();
    for (name, intent) in [("sample 空.mp4", "file"), ("rabbit.cube", "challenge")] {
        fs::write(source.join(name), b"original").unwrap();
        let output = launch(&fixture, &home, &source, &executable)
            .arg("--no-browser")
            .arg(source.join(name))
            .output()
            .unwrap();
        assert!(output.status.success());
        let text = String::from_utf8(output.stdout).unwrap();
        let url = text
            .lines()
            .find_map(|line| line.strip_prefix("Open Cube Play: "))
            .unwrap();
        let fragment = url.split_once('#').unwrap().1;
        let values: std::collections::HashMap<_, _> = form_urlencoded::parse(fragment.as_bytes())
            .into_owned()
            .collect();
        assert_eq!(values["file"], name);
        assert_eq!(values["intent"], intent);
        assert_eq!(fs::read(source.join(name)).unwrap(), b"original");
    }
    assert_eq!(
        fs::metadata(context.join("workspace.json"))
            .unwrap()
            .modified()
            .unwrap(),
        context_modified,
        "reusing a runtime must preserve the mounted context file"
    );
    let events = fs::read_to_string(fixture.join("events")).unwrap();
    let old = String::from_utf8(before).unwrap();
    assert_eq!(
        events.matches("\"up\"").count(),
        old.matches("\"up\"").count(),
        "reused sessions must not recreate containers"
    );
    for (mode, code) in [
        ("unsafe-container", "unsafe_runtime"),
        ("host-network", "unsafe_runtime"),
        ("host-pids", "unsafe_runtime"),
        ("capabilities", "unsafe_runtime"),
        ("writable-source", "unsafe_runtime"),
        ("wrong-port", "unsafe_runtime"),
        ("unhealthy", "runtime_unhealthy"),
        ("bad-volume", "ownership_mismatch"),
        ("missing-session", "session_missing"),
        ("wrong-owner", "ownership_mismatch"),
        ("changed-image", "runtime_changed"),
    ] {
        fs::write(fixture.join("mode"), mode).unwrap();
        error(
            launch(&fixture, &home, &source, &executable).args(["--json-errors", "--no-browser"]),
            code,
        );
    }
    fs::write(fixture.join("mode"), "").unwrap();
    let record = fixture.join("browser-record");
    assert!(
        launch(&fixture, &home, &source, &executable)
            .arg("--offline")
            .env("CUBE_PLAY_BROWSER", &executable)
            .env("CUBE_PLAY_BROWSER_RECORD", &record)
            .output()
            .unwrap()
            .status
            .success()
    );
    let deadline = Instant::now() + Duration::from_secs(5);
    while !record.exists() {
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(fs::read(&record).unwrap(), b"opened");
    fs::write(fixture.join("mode"), "owned-volume").unwrap();
    assert!(
        launch(&fixture, &home, &source, &executable)
            .arg("--no-browser")
            .output()
            .unwrap()
            .status
            .success()
    );
    fs::write(fixture.join("mode"), "").unwrap();
    assert!(
        launch(&fixture, &home, &source, &executable)
            .arg("--stop")
            .status()
            .unwrap()
            .success()
    );
    let container_file = fixture.join(format!("cube-play-{}-local.json", id.replace('-', "")));
    fs::write(fixture.join("mode"), "fail-after-start").unwrap();
    error(
        launch(&fixture, &home, &source, &executable).args(["--json-errors", "--no-browser"]),
        "startup_failed",
    );
    let is_running = || {
        serde_json::from_slice::<Value>(&fs::read(&container_file).unwrap()).unwrap()["State"]["Running"]
            == true
    };
    assert!(
        !is_running(),
        "failed startup must stop its newly created runtime"
    );
    #[cfg(unix)]
    {
        use std::process::Stdio;
        fn interrupt(mut command: Command, ready: impl Fn() -> bool) {
            let mut child = command
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            let deadline = Instant::now() + Duration::from_secs(120);
            while !ready() {
                assert!(
                    Instant::now() < deadline,
                    "launcher did not reach cancellation checkpoint"
                );
                assert!(
                    child.try_wait().unwrap().is_none(),
                    "launcher exited before cancellation"
                );
                std::thread::sleep(Duration::from_millis(20));
            }
            assert!(
                Command::new("/bin/kill")
                    .args(["-TERM", &child.id().to_string()])
                    .status()
                    .unwrap()
                    .success()
            );
            let deadline = Instant::now() + Duration::from_secs(30);
            while child.try_wait().unwrap().is_none() {
                assert!(
                    Instant::now() < deadline,
                    "launcher did not finish cancellation"
                );
                std::thread::sleep(Duration::from_millis(20));
            }
            let output = child.wait_with_output().unwrap();
            assert_eq!(output.status.code(), Some(130));
            let diagnostic: Value = serde_json::from_slice(&output.stderr).unwrap();
            assert_eq!(diagnostic["error"]["code"], "cancelled");
        }
        fs::write(fixture.join("mode"), "cancel-after-start").unwrap();
        let mut command = launch(&fixture, &home, &source, &executable);
        command.args(["--json-errors", "--no-browser"]);
        interrupt(command, is_running);
        assert!(!is_running(), "cancellation must stop a new runtime");
        fs::write(fixture.join("mode"), "").unwrap();
        assert!(
            launch(&fixture, &home, &source, &executable)
                .arg("--no-browser")
                .output()
                .unwrap()
                .status
                .success()
        );
        fs::write(fixture.join("mode"), "cancel-existing-session").unwrap();
        let mut command = launch(&fixture, &home, &source, &executable);
        command.args(["--json-errors", "--no-browser"]);
        interrupt(command, || fixture.join("session-waiting").exists());
        assert!(is_running(), "cancellation must preserve a reused runtime");
    }
    fs::write(fixture.join("mode"), "").unwrap();
    assert!(
        launch(&fixture, &home, &source, &executable)
            .arg("--stop")
            .status()
            .unwrap()
            .success()
    );
    assert!(
        launch(&fixture, &home, &source, &executable)
            .arg("--no-browser")
            .status()
            .unwrap()
            .success()
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(home.join("workspaces.json")).unwrap()).unwrap(),
        registry
    );
    error(
        launch(&fixture, &home, &source, &executable).args(["--json-errors", "--protocol", "99"]),
        "incompatible_launcher",
    );
    error(
        launch(&fixture, &home, &source, &executable).args([
            "--json-errors",
            "--stop",
            "--no-browser",
        ]),
        "invalid_usage",
    );
    error(
        launch(&fixture, &home, &source, &executable)
            .args(["--json-errors"])
            .arg(source.join("missing")),
        "invalid_path",
    );
    error(
        launch(&fixture, &home, &source, &executable)
            .args(["--json-errors"])
            .env("CUBE_PLAY_DOCKER", source.join("missing")),
        "docker_unavailable",
    );
    let relative = PathBuf::from("relative-state");
    error(
        launch(&fixture, &relative, &source, &executable).args(["--json-errors"]),
        "invalid_parameter",
    );
    let cancelled = Arc::new(AtomicBool::new(true));
    let process = HostProcess {
        executable: executable.clone(),
        cancelled,
    };
    assert_eq!(
        process
            .run(&[OsStr::new("info")], &source, Duration::from_secs(1))
            .unwrap_err()
            .code,
        "cancelled"
    );
    let process = HostProcess {
        executable,
        cancelled: Arc::new(AtomicBool::new(false)),
    };
    for scenario in ["hang", "orphan-pipe"] {
        let started = Instant::now();
        assert_eq!(
            process
                .run(&[OsStr::new(scenario)], &source, Duration::from_millis(200))
                .unwrap_err()
                .code,
            "process_timeout"
        );
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "timeout must clean descendant pipes rather than wait for orphaned children"
        );
    }
    let cancel = process.cancelled.clone();
    let worker = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        cancel.store(true, std::sync::atomic::Ordering::Relaxed);
    });
    assert_eq!(
        process
            .run(&[OsStr::new("hang")], &source, Duration::from_secs(5))
            .unwrap_err()
            .code,
        "cancelled"
    );
    worker.join().unwrap();
    process
        .cancelled
        .store(false, std::sync::atomic::Ordering::Relaxed);
    assert_eq!(
        process
            .run(&[OsStr::new("flood")], &source, Duration::from_secs(5))
            .unwrap_err()
            .code,
        "resource_limit"
    );
}

#[test]
fn registry_rejects_corruption_and_cross_workspace_identity_aliases() {
    let directory = tempfile::tempdir().unwrap();
    let home = directory.path().canonicalize().unwrap();
    let cancelled = Arc::new(AtomicBool::new(false));
    let root = home.join("source");
    fs::create_dir(&root).unwrap();
    let workspace = play_launcher::state::workspace(&home, &root, true, &cancelled).unwrap();
    let value = json!({"version":1,"workspaces":[workspace.clone(),workspace]});
    play_launcher::state::write_json(&home.join("workspaces.json"), &value).unwrap();
    assert_eq!(
        play_launcher::state::workspace(&home, &root, true, &cancelled)
            .unwrap_err()
            .code,
        "invalid_state"
    );
    fs::write(home.join("workspaces.json"), b"incomplete").unwrap();
    assert!(play_launcher::state::workspace(&home, &root, true, &cancelled).is_err());
    let started = Instant::now();
    let lock = play_launcher::state::lock(&home.join("test.lock"), &cancelled).unwrap();
    let flag = Arc::new(AtomicBool::new(true));
    assert!(play_launcher::state::lock(&home.join("test.lock"), &flag).is_err());
    drop(lock);
    assert!(started.elapsed() < Duration::from_secs(1));
}
