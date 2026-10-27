//! Real host API and process lifecycle; only the native folder dialog is a fixture.
use play_launcher::state;
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    net::TcpStream,
    path::PathBuf,
    process::{Child, Command, Stdio},
    sync::{Arc, atomic::AtomicBool},
    time::{Duration, Instant},
};
use uuid::Uuid;
struct Host {
    directory: tempfile::TempDir,
    home: PathBuf,
    child: Child,
    control: Value,
}
impl Host {
    fn start() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        let home = root.join("home");
        state::private_directory(&home).unwrap();
        let picker = root.join(format!("picker{}", std::env::consts::EXE_SUFFIX));
        assert!(
            Command::new("rustc")
                .args(["--edition", "2024"])
                .arg(
                    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                        .join("tests/support/folder_picker.rs")
                )
                .arg("-o")
                .arg(&picker)
                .status()
                .unwrap()
                .success()
        );
        // Keep host validation of fresh unsigned macOS fixtures outside the
        // response deadline; production binaries and security policy are unchanged.
        #[cfg(target_os = "macos")]
        assert!(
            Command::new("codesign")
                .args(["--force", "--sign", "-"])
                .arg(&picker)
                .output()
                .unwrap()
                .status
                .success()
        );
        fs::write(root.join("selection"), "").unwrap();
        let child = Command::new(env!("CARGO_BIN_EXE_cube-play"))
            .arg("--workspace-host")
            .arg("--state-home")
            .arg(&home)
            .env("CUBE_PLAY_PICKER", &picker)
            .env("CUBE_PLAY_PICKER_FIXTURE", &root)
            .env("CUBE_PLAY_DOCKER", root.join("missing-docker"))
            .stdout(Stdio::null())
            .stderr(Stdio::inherit())
            .spawn()
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        let control = loop {
            if let Ok(bytes) = fs::read(home.join("chooser.json"))
                && let Ok(value) = serde_json::from_slice::<Value>(&bytes)
            {
                break value;
            }
            assert!(
                Instant::now() < deadline,
                "Host did not write its private control record"
            );
            std::thread::sleep(Duration::from_millis(30));
        };
        let result = Self {
            directory,
            home,
            child,
            control,
        };
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if result
                .request("GET", "/api/health", None, true, &[])
                .is_ok_and(|r| r.0 == 200)
            {
                break;
            }
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(30));
        }
        result
    }
    fn origin(&self) -> String {
        format!(
            "http://127.0.0.1:{}",
            self.control["port"].as_u64().unwrap()
        )
    }
    fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<&str>,
        auth: bool,
        extra: &[(&str, &str)],
    ) -> std::io::Result<(u16, Value, String)> {
        let mut socket = TcpStream::connect(format!("127.0.0.1:{}", self.control["port"]))?;
        socket.set_read_timeout(Some(Duration::from_secs(8)))?;
        let mut headers = format!(
            "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nOrigin: {}\r\nConnection: close\r\n",
            self.control["port"],
            self.origin()
        );
        if auth {
            headers += &format!(
                "Authorization: Bearer {}\r\n",
                self.control["secret"].as_str().unwrap()
            );
        }
        for (key, value) in extra {
            headers += &format!("{key}: {value}\r\n");
        }
        if let Some(body) = body {
            headers += &format!(
                "Content-Type: application/json\r\nContent-Length: {}\r\n",
                body.len()
            );
        }
        headers += "\r\n";
        socket.write_all(headers.as_bytes())?;
        if let Some(body) = body {
            socket.write_all(body.as_bytes())?;
        }
        let mut response = String::new();
        socket.read_to_string(&mut response)?;
        let (head, body) = response.split_once("\r\n\r\n").unwrap();
        let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
        Ok((
            status,
            serde_json::from_str(body).unwrap_or(Value::Null),
            response,
        ))
    }
    fn api(&self, body: Value) -> (u16, Value, String) {
        self.request("POST", "/api/intents", Some(&body.to_string()), true, &[])
            .unwrap()
    }
}
impl Drop for Host {
    fn drop(&mut self) {
        let _ = play_launcher::host::stop(&self.home);
        let deadline = Instant::now() + Duration::from_secs(5);
        while self.child.try_wait().unwrap().is_none() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(20));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn host_authority_is_separate_bounded_and_resolves_only_native_choices_or_registered_ids() {
    let host = Host::start();
    let root = host.directory.path().canonicalize().unwrap();
    assert_eq!(
        host.request("GET", "/api/workspaces", None, false, &[])
            .unwrap()
            .0,
        401
    );
    assert_eq!(
        host.request(
            "GET",
            "/api/workspaces",
            None,
            false,
            &[("Cookie", &format!("session={}", host.control["secret"]))]
        )
        .unwrap()
        .0,
        401
    );
    assert_eq!(
        host.request(
            "POST",
            "/api/intents",
            Some("{}"),
            true,
            &[("Origin", "http://127.0.0.1:1")]
        )
        .unwrap()
        .0,
        403
    );
    assert_eq!(
        host.request(
            "GET",
            "/api/workspaces",
            None,
            true,
            &[("X-Forwarded-Host", "attacker.invalid")]
        )
        .unwrap()
        .0,
        403
    );
    for body in [
        json!({"schema_version":2,"intent":"open"}),
        json!({"schema_version":1,"intent":"open","path":"/"}),
        json!({"schema_version":1,"intent":"new","name":"../escape"}),
        json!({"schema_version":1,"intent":"new","name":"CON.txt"}),
    ] {
        assert_eq!(host.api(body).0, 400);
    }
    assert!(
        !root.join("events").exists(),
        "Invalid requests must never invoke the system picker"
    );
    assert_eq!(
        host.request(
            "POST",
            "/api/intents",
            Some("{\"intent\":\"open\",\"schema_version\":1,\"schema_version\":2}"),
            true,
            &[]
        )
        .unwrap()
        .0,
        400
    );
    assert_eq!(
        host.request("POST", "/api/intents", Some(&"x".repeat(4097)), true, &[])
            .unwrap()
            .0,
        400
    );
    let cancelled = host.api(json!({"schema_version":1,"intent":"open"}));
    assert_eq!(cancelled.0, 200);
    assert_eq!(cancelled.1["cancelled"], true);
    let flag = Arc::new(AtomicBool::new(false));
    let source = root.join("existing $ 空");
    fs::create_dir(&source).unwrap();
    let workspace = state::workspace(&host.home, &source, true, &flag).unwrap();
    let stale = root.join("stale");
    fs::create_dir(&stale).unwrap();
    let stale_workspace = state::workspace(&host.home, &stale, true, &flag).unwrap();
    fs::remove_dir(&stale).unwrap();
    let rows = host
        .request("GET", "/api/workspaces", None, true, &[])
        .unwrap();
    assert_eq!(rows.0, 200);
    assert!(!rows.1.to_string().contains(root.to_str().unwrap()));
    assert_eq!(
        rows.1["workspaces"][0]["id"],
        stale_workspace.id.to_string()
    );
    assert_eq!(rows.1["workspaces"][0]["available"], false);
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(&source, &stale).unwrap();
        assert_eq!(
            host.api(json!({"schema_version":1,"intent":"recent","id":stale_workspace.id}))
                .1["error"]["code"],
            "source_unavailable"
        );
        let rows = host
            .request("GET", "/api/workspaces", None, true, &[])
            .unwrap();
        assert_eq!(rows.1["workspaces"][0]["available"], false);
        assert_eq!(
            state::workspace(&host.home, &source, true, &flag)
                .unwrap()
                .id,
            workspace.id
        );
        fs::remove_file(&stale).unwrap();
    }
    assert_eq!(
        host.api(json!({"schema_version":1,"intent":"recent","id":Uuid::new_v4()}))
            .1["error"]["code"],
        "workspace_missing"
    );
    fs::write(root.join("selection"), host.home.to_str().unwrap()).unwrap();
    assert_eq!(
        host.api(json!({"schema_version":1,"intent":"open"})).1["error"]["code"],
        "unsafe_source"
    );
    fs::write(root.join("selection"), root.to_str().unwrap()).unwrap();
    let created = host.api(json!({"schema_version":1,"intent":"new","name":"new $ 空"}));
    assert_eq!(created.1["error"]["code"], "docker_unavailable");
    assert!(root.join("new $ 空").is_dir());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(root.join("new $ 空"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
    }
    assert_eq!(
        host.api(json!({"schema_version":1,"intent":"new","name":"new $ 空"}))
            .1["error"]["code"],
        "folder_exists"
    );
    assert_eq!(
        host.api(json!({"schema_version":1,"intent":"recent","id":workspace.id}))
            .1["error"]["code"],
        "docker_unavailable"
    );
    let html = host.request("GET", "/", None, false, &[]).unwrap();
    assert_eq!(html.0, 200);
    assert!(html.2.contains("frame-ancestors 'none'"));
    assert!(!html.2.contains(host.control["secret"].as_str().unwrap()));
    assert!(
        host.request("GET", "/launcher.js", None, false, &[])
            .unwrap()
            .2
            .contains("Your investigations")
    );
}

#[test]
fn host_serializes_native_actions_and_bounds_slow_requests() {
    let host = Host::start();
    let root = host.directory.path().canonicalize().unwrap();
    fs::write(root.join("hold"), b"").unwrap();
    std::thread::scope(|scope| {
        let action = scope.spawn(|| host.api(json!({"schema_version":1,"intent":"open"})));
        let deadline = Instant::now() + Duration::from_secs(5);
        while !root.join("events").exists() {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(20));
        }
        assert_eq!(host.api(json!({"schema_version":1,"intent":"open"})).0, 429);
        assert_eq!(
            host.request("POST", "/api/shutdown", None, true, &[])
                .unwrap()
                .0,
            429
        );
        assert_eq!(
            host.request("GET", "/api/workspaces", None, true, &[])
                .unwrap()
                .0,
            200
        );
        fs::remove_file(root.join("hold")).unwrap();
        assert_eq!(action.join().unwrap().1["cancelled"], true);
    });
    let mut socket = TcpStream::connect(format!("127.0.0.1:{}", host.control["port"])).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(8)))
        .unwrap();
    write!(socket,"POST /api/intents HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nOrigin: {}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: 100\r\nConnection: close\r\n\r\n{{",host.control["port"],host.origin(),host.control["secret"].as_str().unwrap()).unwrap();
    let mut response = String::new();
    socket.read_to_string(&mut response).unwrap();
    assert!(response.starts_with("HTTP/1.1 408 "));
    assert!(response.contains("request_timeout"));
}
