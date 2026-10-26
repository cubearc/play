use std::{
    io::{BufRead, BufReader},
    net::TcpListener,
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::Duration,
};

fn command(address: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_play-local"));
    command
        .env("CUBE_PLAY_BIND", address)
        .env("CUBE_PLAY_ORIGIN", format!("http://{address}"))
        .env_remove("CUBE_PLAY_CONTAINER")
        .env_remove("RUST_LOG");
    command
}

fn available_address() -> String {
    TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .to_string()
}

struct Server(Child);
impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

#[test]
fn starts_real_server_and_checks_readiness() {
    let address = available_address();
    let mut server = Server(
        command(&address)
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap(),
    );
    let stdout = server.0.stdout.take().unwrap();
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        let _ = BufReader::new(stdout).read_line(&mut line);
        let _ = sender.send(line);
    });
    let line = receiver
        .recv_timeout(Duration::from_secs(10))
        .expect("server readiness output");
    assert!(line.starts_with(&format!("Open Cube Play: http://{address}/#session=")));
    let result = command(&address).arg("--healthcheck").output().unwrap();
    assert!(result.status.success());
    #[cfg(unix)]
    {
        let result = Command::new("kill")
            .args(["-TERM", &server.0.id().to_string()])
            .status()
            .unwrap();
        assert!(result.success());
        let deadline = std::time::Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(status) = server.0.try_wait().unwrap() {
                assert!(status.success());
                break;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "graceful shutdown timed out"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

#[test]
fn healthcheck_fails_when_server_is_unavailable() {
    assert!(
        !command(&available_address())
            .arg("--healthcheck")
            .output()
            .unwrap()
            .status
            .success()
    );
}

#[test]
fn invalid_invocation_and_unsafe_configuration_fail_explicitly() {
    for argument in ["unexpected", "--healthcheck extra"] {
        let output = command(&available_address())
            .arg(argument)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("Usage:"));
    }
    let output = command("0.0.0.0:7331").output().unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("Bind to loopback"));
}
