#![cfg(target_os = "linux")]
use play_runner::{
    protocol::{Client, Location, Server, Size, Store},
    transport::{Channel, channel, receive, send},
};
use std::{
    io,
    path::Path,
    process::{Child, Command, Stdio},
    time::Duration,
};
struct Runner(Child);
impl Drop for Runner {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
async fn frame(socket: &mut Channel) -> Server {
    tokio::time::timeout(Duration::from_secs(5), receive(socket))
        .await
        .unwrap()
        .unwrap()
        .unwrap()
}
async fn until(socket: &mut Channel, needle: &str) -> String {
    let mut output = Vec::new();
    loop {
        match frame(socket).await {
            Server::Resized { .. } => {}
            Server::Output { data } => {
                output.extend(data);
                let text = String::from_utf8_lossy(&output);
                if text.contains(needle) {
                    return text.into_owned();
                }
            }
            other => panic!("Unexpected terminal state: {other:?}"),
        }
    }
}
#[test]
fn real_cli_is_isolated_and_session_resources_are_released() {
    use std::os::unix::process::CommandExt;
    // Build-stage tests run their actual runner as the same non-root account used at runtime.
    if nix::unistd::geteuid().is_root() {
        let status = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "real_cli_is_isolated_and_session_resources_are_released",
                "--nocapture",
            ])
            .uid(65532)
            .gid(65532)
            .status()
            .unwrap();
        assert!(status.success());
        return;
    }
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        use std::os::unix::fs::{DirBuilderExt, symlink};
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let source = root.join("source");
        let imports = root.join("data/imports");
        let output = root.join("output");
        let ipc = root.join("ipc");
        for path in [&source, &imports, &output, &ipc] {
            std::fs::DirBuilder::new()
                .recursive(true)
                .mode(0o700)
                .create(path)
                .unwrap();
        }
        std::fs::create_dir(source.join("nested")).unwrap();
        std::fs::write(source.join("original.txt"), b"original source bytes").unwrap();
        std::fs::write(root.join("data/private.txt"), b"PRIVATE_DATABASE_SENTINEL").unwrap();
        symlink(root.join("data"), source.join("escape")).unwrap();
        let cube = std::env::var_os("CUBE_RUNNER_CUBE")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| {
                Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../cube/target/debug/cube")
            });
        assert!(
            cube.is_file(),
            "Build the real Cube CLI before runner integration tests"
        );
        let socket = ipc.join("runner.sock");
        let mut runner = Runner(
            Command::new(env!("CARGO_BIN_EXE_cube-runner"))
                .env("CUBE_RUNNER_SOCKET", &socket)
                .env("CUBE_RUNNER_SOURCE", &source)
                .env("CUBE_RUNNER_IMPORTS", &imports)
                .env("CUBE_RUNNER_OUTPUT", &output)
                .env("CUBE_RUNNER_CUBE", cube.canonicalize().unwrap())
                .stdout(Stdio::null())
                .stderr(Stdio::inherit())
                .spawn()
                .unwrap(),
        );
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        let connect = || async {
            Ok::<_, io::Error>(channel(tokio::net::UnixStream::connect(&socket).await?))
        };
        let mut first = loop {
            if let Ok(socket) = connect().await {
                break socket;
            }
            assert!(
                runner.0.try_wait().unwrap().is_none(),
                "Runner failed its required sandbox preflight"
            );
            assert!(
                tokio::time::Instant::now() < deadline,
                "Runner did not start"
            );
            tokio::time::sleep(Duration::from_millis(20)).await;
        };
        let size = Size { cols: 80, rows: 24 };
        send(
            &mut first,
            &Client::Start {
                location: Location::default(),
                size,
            },
        )
        .await
        .unwrap();
        assert!(matches!(frame(&mut first).await, Server::Ready { .. }));
        until(&mut first, "source:/> ").await;
        send(
            &mut first,
            &Client::Input {
                data: "cube encode base64 hello\r".into(),
            },
        )
        .await
        .unwrap();
        let encoded = until(&mut first, "source:/> ").await;
        assert!(encoded.contains("aGVsbG8="), "Unexpected CLI output: {encoded}");
        // A burst of small keyboard messages must wait for the bounded writer queue.
        for character in "cube encode base64 burst\r".chars() {
            send(&mut first, &Client::Input {data:character.to_string()}).await.unwrap();
        }
        assert!(until(&mut first,"source:/> ").await.contains("YnVyc3Q="));
        send(
            &mut first,
            &Client::Input {
                data: "cube nonexistent-command\r".into(),
            },
        )
        .await
        .unwrap();
        until(&mut first, "source:/> ").await;
        send(
            &mut first,
            &Client::Resize {
                size: Size {
                    cols: 100,
                    rows: 30,
                },
            },
        )
        .await
        .unwrap();
        assert!(matches!(frame(&mut first).await,Server::Resized{size} if size.cols==100 && size.rows==30));
        send(
            &mut first,
            &Client::Input {
                data: "cd source nested\r".into(),
            },
        )
        .await
        .unwrap();
        until(&mut first, "source:/nested> ").await;
        send(
            &mut first,
            &Client::Input {
                data: "pwd\r".into(),
            },
        )
        .await
        .unwrap();
        until(&mut first, "source:/nested> ").await;
        for path in [source.join("escape/private.txt"), root.join("data/private.txt")] {
            send(&mut first, &Client::Input {
                data: format!("cube encode base64 --file {}\r", path.display()),
            }).await.unwrap();
            let denied = until(&mut first, "source:/nested> ").await;
            assert!(denied.contains("PermissionDenied"), "{denied}");
            assert!(!denied.contains("UFJJVkFURV9EQVRBQkFTRV9TRU5USU5FTA=="));
        }
        send(&mut first, &Client::Input {
            data: format!("cube encode base64 --file {}\r", source.join("original.txt").display()),
        }).await.unwrap();
        assert!(until(&mut first, "source:/nested> ").await.contains("b3JpZ2luYWwgc291cmNlIGJ5dGVz"));
        let mut second = connect().await.unwrap();
        send(
            &mut second,
            &Client::Start {
                location: Location {
                    store: Store::Output,
                    path: String::new(),
                },
                size,
            },
        )
        .await
        .unwrap();
        assert!(matches!(frame(&mut second).await, Server::Ready { .. }));
        until(&mut second, "output:/> ").await;
        // Leave a real CLI child blocked on PTY stdin, then disconnect below.
        send(&mut second, &Client::Input { data: "cube encode base64\r".into() }).await.unwrap();
        until(&mut second, "cube encode base64").await;
        let mut third = connect().await.unwrap();
        send(
            &mut third,
            &Client::Start {
                location: Location::default(),
                size,
            },
        )
        .await
        .unwrap();
        assert!(matches!(frame(&mut third).await,Server::Error{code} if code=="session_limit"));
        drop(first);
        drop(second);
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            let mut probe = connect().await.unwrap();
            send(&mut probe, &Client::Probe).await.unwrap();
            if matches!(frame(&mut probe).await, Server::Status { active: 0, .. }) {
                break;
            }
            assert!(tokio::time::Instant::now() < deadline);
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        let mut invalid = connect().await.unwrap();
        send(
            &mut invalid,
            &Client::Start {
                location: Location {
                    store: Store::Source,
                    path: "escape".into(),
                },
                size,
            },
        )
        .await
        .unwrap();
        assert!(matches!(frame(&mut invalid).await,Server::Error{code} if code=="path_denied"));
        let mut oversized = connect().await.unwrap();
        send(&mut oversized, &Client::Start { location: Location::default(), size }).await.unwrap();
        assert!(matches!(frame(&mut oversized).await, Server::Ready { .. }));
        until(&mut oversized, "source:/> ").await;
        send(&mut oversized, &Client::Input { data: "x".repeat(play_runner::protocol::INPUT_FRAME + 1) }).await.unwrap();
        assert!(matches!(frame(&mut oversized).await, Server::End {reason} if reason == "invalid_input"));

        // Exercise the real output limiter with CLI output, rather than a mocked stream.
        std::fs::write(source.join("large.bin"), vec![0u8; 7 * 1024 * 1024]).unwrap();
        let mut bounded = connect().await.unwrap();
        send(&mut bounded, &Client::Start { location: Location::default(), size }).await.unwrap();
        assert!(matches!(frame(&mut bounded).await, Server::Ready { .. }));
        until(&mut bounded, "source:/> ").await;
        send(&mut bounded, &Client::Input { data: "cube encode base64 --file large.bin\r".into() }).await.unwrap();
        let mut bytes = 0;
        loop {
            match frame(&mut bounded).await {
                Server::Output {data} => { bytes += data.len(); assert!(bytes <= play_runner::protocol::OUTPUT_LIMIT); },
                Server::End {reason} => { assert_eq!(reason, "output_limit"); break; },
                other => panic!("Unexpected bounded session state: {other:?}"),
            }
        }
        assert!(bytes > 7 * 1024 * 1024);
        let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
        loop {
            let mut probe = connect().await.unwrap();
            send(&mut probe, &Client::Probe).await.unwrap();
            if matches!(frame(&mut probe).await, Server::Status { active: 0, .. }) { break; }
            assert!(tokio::time::Instant::now() < deadline, "Bounded session did not release its processes");
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
        assert_eq!(
            std::fs::read(source.join("original.txt")).unwrap(),
            b"original source bytes"
        );
    });
}
