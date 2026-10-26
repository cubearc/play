//! End-to-end API coverage uses the real server and its actual Core worker executable.
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
    sync::mpsc,
    time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;
struct Server {
    process: Child,
    address: String,
    secret: String,
}
impl Drop for Server {
    fn drop(&mut self) {
        if self.process.try_wait().ok().flatten().is_some() {
            return;
        }
        #[cfg(unix)]
        {
            let _ = Command::new("kill")
                .args(["-TERM", &self.process.id().to_string()])
                .status();
            let deadline = std::time::Instant::now() + Duration::from_secs(3);
            while self.process.try_wait().ok().flatten().is_none()
                && std::time::Instant::now() < deadline
            {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        let _ = self.process.kill();
        let _ = self.process.wait();
    }
}
impl Server {
    fn start(root: &std::path::Path, id: Uuid) -> Self {
        let source = root.join("source");
        std::fs::create_dir_all(&source).unwrap();
        let context = root.join("context.json");
        std::fs::write(&context, json!({"id":id,"name":"Lab fixture"}).to_string()).unwrap();
        let address = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap()
            .to_string();
        let mut process = Command::new(env!("CARGO_BIN_EXE_play-local"))
            .env("CUBE_PLAY_BIND", &address)
            .env("CUBE_PLAY_ORIGIN", format!("http://{address}"))
            .env("CUBE_PLAY_SOURCE", &source)
            .env("CUBE_PLAY_WORKSPACE", context)
            .env("CUBE_PLAY_DATA", root.join("imports"))
            .env_remove("CUBE_PLAY_STATE")
            .env_remove("CUBE_PLAY_CONTAINER")
            .env_remove("CUBE_PLAY_RUNNER_SOCKET")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdout = process.stdout.take().unwrap();
        let (send, receive) = mpsc::channel();
        std::thread::spawn(move || {
            let mut line = String::new();
            let _ = BufReader::new(stdout).read_line(&mut line);
            let _ = send.send(line);
        });
        let line = receive.recv_timeout(Duration::from_secs(10)).unwrap();
        let secret = line.split("#session=").nth(1).unwrap().trim().to_owned();
        Self {
            process,
            address,
            secret,
        }
    }
    async fn raw(&self, method: &str, path: &str, body: &[u8], authorized: bool) -> (u16, Vec<u8>) {
        let mut socket = tokio::net::TcpStream::connect(&self.address).await.unwrap();
        let cookie = if authorized {
            format!(
                "Cookie: cube_play_session_{}={}\r\n",
                self.address.split(':').nth(1).unwrap(),
                self.secret
            )
        } else {
            String::new()
        };
        let head = format!(
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nOrigin: http://{}\r\n{cookie}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            self.address,
            self.address,
            body.len()
        );
        socket.write_all(head.as_bytes()).await.unwrap();
        socket.write_all(body).await.unwrap();
        let mut response = Vec::new();
        tokio::time::timeout(Duration::from_secs(12), socket.read_to_end(&mut response))
            .await
            .unwrap()
            .unwrap();
        let split = response.windows(4).position(|w| w == b"\r\n\r\n").unwrap();
        let head = std::str::from_utf8(&response[..split]).unwrap();
        let status = head.split_whitespace().nth(1).unwrap().parse().unwrap();
        let body = &response[split + 4..];
        let body = if head.to_lowercase().contains("transfer-encoding: chunked") {
            let mut rest = body;
            let mut decoded = Vec::new();
            loop {
                let line = rest.windows(2).position(|w| w == b"\r\n").unwrap();
                let n =
                    usize::from_str_radix(std::str::from_utf8(&rest[..line]).unwrap(), 16).unwrap();
                if n == 0 {
                    break;
                }
                decoded.extend_from_slice(&rest[line + 2..line + 2 + n]);
                rest = &rest[line + 2 + n + 2..];
            }
            decoded
        } else {
            body.to_vec()
        };
        (status, body)
    }
    async fn api(&self, method: &str, path: &str, body: Value) -> (u16, Value) {
        let (status, body) = self
            .raw(method, path, &serde_json::to_vec(&body).unwrap(), true)
            .await;
        (status, serde_json::from_slice(&body).unwrap_or(Value::Null))
    }
    async fn execute(&self, operation: &str, input: Value, parameters: Value) -> (u16, Value) {
        self.api("POST","/api/lab/execute",json!({"id":Uuid::new_v4(),"operation":operation,"version":1,"input":input,"parameters":parameters})).await
    }
}
#[tokio::test]
async fn real_worker_preserves_bytes_provenance_and_results_across_restart() {
    let directory = tempfile::tempdir().unwrap();
    let id = Uuid::new_v4();
    let mut server = Server::start(directory.path(), id);
    assert_eq!(
        server.raw("GET", "/api/lab/operations", b"", false).await.0,
        401
    );
    let (status, catalog) = server.api("GET", "/api/lab/operations", Value::Null).await;
    assert_eq!(status, 200);
    let image = catalog["operations"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["core"]["id"] == "image.histogram")
        .unwrap();
    assert!(image["core"]["pipeline"].is_null());
    assert_eq!(image["lab"]["mode"], "direct");
    assert_eq!(catalog["core_version"], "0.18.0");
    let (status, encoded) = server
        .execute(
            "encoding.base64.encode",
            json!({"type":"bytes","base64":"AP+A"}),
            json!([]),
        )
        .await;
    assert_eq!(status, 200, "{encoded}");
    assert_eq!(encoded["inline"]["data"], "AP+A");
    let (status, decoded) = server
        .execute(
            "encoding.base64.decode",
            json!({"type":"artifact","id":encoded["result"]["id"]}),
            json!([]),
        )
        .await;
    assert_eq!(status, 200, "{decoded}");
    assert_eq!(decoded["inline"]["base64"], "AP+A");
    assert_eq!(
        decoded["result"]["provenance"]["inputs"][0]["reference"]["id"],
        encoded["result"]["id"]
    );
    assert_eq!(decoded["result"]["workspace_id"], id.to_string());
    let (status, bytes) = server
        .raw("GET", decoded["content_url"].as_str().unwrap(), b"", true)
        .await;
    assert_eq!(status, 200);
    assert_eq!(bytes, [0, 255, 128]);
    let source = directory.path().join("source/large.bin");
    std::fs::write(&source, vec![0u8; 20000]).unwrap();
    let (status, large) = server
        .execute(
            "encoding.base64.encode",
            json!({"type":"file","store":"source","path":"large.bin","representation":"bytes"}),
            json!([]),
        )
        .await;
    assert_eq!(status, 200, "{large}");
    assert!(large["inline"].is_null());
    assert_eq!(large["result"]["byte_size"], 26668);
    assert_eq!(std::fs::read(&source).unwrap(), vec![0u8; 20000]);
    server.process.kill().unwrap();
    server.process.wait().unwrap();
    drop(server);
    let server = Server::start(directory.path(), id);
    let (status, reopened) = server
        .api(
            "GET",
            &format!(
                "/api/lab/results/{}",
                large["result"]["id"].as_str().unwrap()
            ),
            Value::Null,
        )
        .await;
    assert_eq!(status, 200, "{reopened}");
    assert_eq!(reopened["result"], large["result"]);
    let forged = directory.path().join(format!(
        "artifacts/lab/{}/value.bin",
        large["result"]["id"].as_str().unwrap()
    ));
    std::fs::write(forged, b"tampered").unwrap();
    assert_eq!(
        server
            .api(
                "GET",
                &format!(
                    "/api/lab/results/{}",
                    large["result"]["id"].as_str().unwrap()
                ),
                Value::Null
            )
            .await
            .0,
        503
    );
}
#[tokio::test]
async fn direct_images_validation_authorization_limits_and_cancellation() {
    let directory = tempfile::tempdir().unwrap();
    let server = Server::start(directory.path(), Uuid::new_v4());
    std::fs::write(
        directory.path().join("source/image.png"),
        include_bytes!("../../../tests/fixtures/sample.png"),
    )
    .unwrap();
    let image = json!({"type":"file","store":"source","path":"image.png","representation":"image"});
    let (status, histogram) = server
        .execute("image.histogram", image.clone(), json!([]))
        .await;
    assert_eq!(status, 200, "{histogram}");
    assert_eq!(histogram["inline"]["data"]["pixels"], 4);
    let (status, inverted) = server
        .execute("image.invert", image.clone(), json!([]))
        .await;
    assert_eq!(status, 200, "{inverted}");
    let (status,comparison)=server.api("POST","/api/lab/execute",json!({"id":Uuid::new_v4(),"operation":"image.compare","version":1,"input":image,"other":{"type":"artifact","id":inverted["result"]["id"]}})).await;
    assert_eq!(status, 200, "{comparison}");
    assert_eq!(comparison["inline"]["data"]["equal"], false);
    for (operation, input, parameters, expected) in [
        (
            "missing.operation",
            json!({"type":"text","data":"a"}),
            json!([]),
            404,
        ),
        (
            "image.colors.extract",
            json!({"type":"text","data":"a"}),
            json!([]),
            409,
        ),
        (
            "encoding.base64.decode",
            json!({"type":"bytes","base64":"YQ=="}),
            json!([]),
            400,
        ),
        ("image.pixels.read", image.clone(), json!([]), 400),
        (
            "image.histogram",
            image.clone(),
            json!([{"name":"max_colors","value":0}]),
            400,
        ),
        (
            "image.rotate",
            image.clone(),
            json!([{"name":"degrees","value":"90"}]),
            400,
        ),
        (
            "image.rotate",
            image.clone(),
            json!([{"name":"degrees","value":90},{"name":"degrees","value":90}]),
            400,
        ),
        (
            "encoding.base64.encode",
            json!({"type":"file","store":"source","path":"../context.json","representation":"bytes"}),
            json!([]),
            400,
        ),
        (
            "encoding.base64.encode",
            json!({"type":"artifact","id":Uuid::new_v4()}),
            json!([]),
            404,
        ),
        (
            "encoding.base64.encode",
            json!({"type":"file","store":"source","path":"image.png","representation":"structured"}),
            json!([]),
            400,
        ),
    ] {
        let (status, error) = server.execute(operation, input, parameters).await;
        assert_eq!(status, expected, "{error}");
        assert_eq!(error["schema_version"], 1);
    }
    let (status, error) = server
        .raw("POST", "/api/lab/execute", br#"{"id":0,"id":1}"#, true)
        .await;
    assert_eq!(status, 400);
    assert!(String::from_utf8(error).unwrap().contains("invalid_input"));
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(
            directory.path().join("context.json"),
            directory.path().join("source/escape"),
        )
        .unwrap();
        assert_eq!(server.execute("encoding.base64.encode",json!({"type":"file","store":"source","path":"escape","representation":"bytes"}),json!([])).await.0,403);
    }
    let exact = json!({"id":Uuid::new_v4(),"operation":"encoding.base64.encode","version":1,"input":{"type":"bytes","base64":"YQ=="},"max_output_bytes":4});
    assert_eq!(
        server
            .api("POST", "/api/lab/execute", exact.clone())
            .await
            .0,
        200
    );
    let mut over = exact;
    over["id"] = json!(Uuid::new_v4());
    over["max_output_bytes"] = json!(3);
    assert_eq!(server.api("POST", "/api/lab/execute", over).await.0, 413);
    let pixels = vec![1u8; 1024 * 1024 * 3];
    let large_image = cube_core::image::Image::from_bytes(
        1024,
        1024,
        cube_core::image::ColorModel::Rgb,
        &pixels,
        &[],
        &[],
    )
    .unwrap()
    .encode_png()
    .unwrap();
    std::fs::write(directory.path().join("source/large.png"), large_image).unwrap();
    let large_input =
        json!({"type":"file","store":"source","path":"large.png","representation":"image"});
    let timeout = json!({"id":Uuid::new_v4(),"operation":"image.histogram","version":1,"input":large_input,"timeout_ms":1});
    assert_eq!(server.api("POST", "/api/lab/execute", timeout).await.0, 408);
    assert_eq!(
        server
            .api(
                "DELETE",
                &format!("/api/lab/executions/{}", Uuid::new_v4()),
                Value::Null
            )
            .await
            .0,
        404
    );
    let id = Uuid::new_v4();
    let operation = server.api(
        "POST",
        "/api/lab/execute",
        json!({"id":id,"operation":"image.histogram","version":1,"input":large_input}),
    );
    let cancel = async {
        for _ in 0..100 {
            let (status, _) = server
                .api("DELETE", &format!("/api/lab/executions/{id}"), Value::Null)
                .await;
            if status == 202 {
                return;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
        panic!("Execution never became cancellable");
    };
    let ((status, error), ()) = tokio::join!(operation, cancel);
    assert_eq!(status, 409, "{error}");
    assert_eq!(error["error"]["code"], "cancelled");
    assert_eq!(
        server
            .execute(
                "encoding.base64.encode",
                json!({"type":"bytes","base64":"YQ=="}),
                json!([])
            )
            .await
            .0,
        200
    );
    assert_eq!(
        server
            .api("GET", "/api/lab/results/invalid", Value::Null)
            .await
            .0,
        400
    );
}

#[test]
fn worker_stdio_uses_a_bounded_typed_contract() {
    use std::io::Write;
    let mut child = Command::new(env!("CARGO_BIN_EXE_play-local"))
        .arg("--lab-worker")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let request = json!({"request":{"id":Uuid::new_v4(),"operation":"encoding.base64.encode","version":1,"input":{"type":"bytes","base64":"AP+A"}},"input":{"representation":"bytes","semantic_type":"bytes","base64":"AP+A","metadata":null},"other":null});
    child
        .stdin
        .take()
        .unwrap()
        .write_all(request.to_string().as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["Ok"]["data"]["representation"], "text");
    let mut child = Command::new(env!("CARGO_BIN_EXE_play-local"))
        .arg("--lab-worker")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(b"invalid").unwrap();
    let output = child.wait_with_output().unwrap();
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap()["Err"]["code"],
        "invalid_input"
    );
}

#[tokio::test]
async fn transient_probes_do_not_consume_artifact_quota_or_create_reusable_ids() {
    let directory = tempfile::tempdir().unwrap();
    let server = Server::start(directory.path(), Uuid::new_v4());
    std::fs::write(
        directory.path().join("source/image.png"),
        include_bytes!("../../../tests/fixtures/sample.png"),
    )
    .unwrap();
    let image = json!({"type":"file","store":"source","path":"image.png","representation":"image"});
    for _ in 0..65 {
        let id = Uuid::new_v4();
        let (status, probe) = server.api("POST", "/api/lab/execute", json!({"id":id,"operation":"image.histogram","version":1,"input":image,"parameters":[{"name":"channels","value":{"text_list":["red"]}}],"retain_result":false})).await;
        assert_eq!(status, 200, "{probe}");
        assert_eq!(probe["retained"], false);
        assert!(probe["content_url"].is_null());
        assert_eq!(probe["inline"]["data"]["pixels"], 4);
        assert_eq!(
            server
                .api("GET", &format!("/api/lab/results/{id}"), Value::Null)
                .await
                .0,
            404
        );
    }
    assert_eq!(
        std::fs::read_dir(directory.path().join("artifacts/lab"))
            .unwrap()
            .count(),
        0
    );
    let (status, retained) = server
        .execute("image.invert", image.clone(), json!([]))
        .await;
    assert_eq!(status, 200, "{retained}");
    assert_eq!(retained["retained"], true);
    let (status, failure) = server.api("POST", "/api/lab/execute", json!({"id":Uuid::new_v4(),"operation":"image.invert","version":1,"input":image,"retain_result":false})).await;
    assert_eq!(status, 400, "{failure}");
}
