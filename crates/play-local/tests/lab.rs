//! End-to-end API coverage uses the real server and its actual Core worker executable.
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
    sync::{Mutex, mpsc},
    time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

// Hold the fixture allocator until the child binds. Operations remain concurrent.
static SERVER_STARTUP: Mutex<()> = Mutex::new(());

#[tokio::test]
async fn notes_bind_actual_attempts_and_ports_without_following_a_later_graph_revision() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = Uuid::new_v4();
    let server = Server::start(directory.path(), workspace);
    let graph = Uuid::new_v4();
    let step = Uuid::new_v4();
    let mut definition = json!({"schema_version":1,"id":graph,"revision":0,"title":"Evidence notes","hypothesis":"","scope":null,"steps":[{"id":step,"operation":"text.reverse","operation_version":1,"inputs":{"input":{"type":"literal","value":"ABC"}},"parameters":{},"private_parameters":[],"position":[0,0]}]});
    assert_eq!(
        server
            .api("PUT", "/api/explorations", definition.clone())
            .await
            .0,
        200
    );
    let invoke = |id, revision, input| json!({"schema_version":2,"id":id,"operation":"text.reverse","operation_version":1,"inputs":{"input":input},"parameters":{},"exploration":{"id":graph,"revision":revision,"step":step}});
    let first = Uuid::new_v4();
    let (status, result) = server
        .api(
            "POST",
            "/api/lab/v2/execute",
            invoke(first, 1, json!({"type":"literal","value":"ABC"})),
        )
        .await;
    assert_eq!(status, 200, "{result}");
    let note_id = Uuid::new_v4();
    let mut note = json!({"id":note_id,"revision":0,"title":"Original output","body":"Keep the actual first attempt","target":{"execution_id":first,"output_port":"output"}});
    assert_eq!(
        server
            .api("PUT", "/api/investigation/notes", note.clone())
            .await
            .0,
        200
    );
    for target in [
        json!({"execution_id":Uuid::new_v4(),"output_port":null}),
        json!({"execution_id":first,"output_port":"missing"}),
        json!({"execution_id":first,"output_port":"output","path":"../outside"}),
    ] {
        let mut invalid = note.clone();
        invalid["id"] = json!(Uuid::new_v4());
        invalid["target"] = target;
        assert_eq!(
            server
                .api("PUT", "/api/investigation/notes", invalid)
                .await
                .0,
            400
        );
    }
    let other_directory = tempfile::tempdir().unwrap();
    let other = Server::start(other_directory.path(), Uuid::new_v4());
    assert_eq!(
        other
            .api("PUT", "/api/investigation/notes", note.clone())
            .await
            .0,
        400
    );
    drop(other);
    definition["revision"] = json!(1);
    definition["steps"][0]["inputs"]["input"]["value"] = json!("XYZ");
    assert_eq!(
        server
            .api("PUT", "/api/explorations", definition.clone())
            .await
            .0,
        200
    );
    let second = Uuid::new_v4();
    assert_eq!(
        server
            .api(
                "POST",
                "/api/lab/v2/execute",
                invoke(second, 2, json!({"type":"literal","value":"XYZ"}))
            )
            .await
            .0,
        200
    );
    let (_, run) = server
        .api(
            "GET",
            &format!("/api/explorations/executions/{first}"),
            Value::Null,
        )
        .await;
    assert_eq!(run["id"], first.to_string());
    assert_eq!(run["exploration_id"], graph.to_string());
    assert_eq!(run["configuration_revision"], 1);
    assert_eq!(run["stale"], true);
    assert_eq!(run["evidence"]["result_id"], first.to_string());
    note["revision"] = json!(1);
    note["body"] = json!("Editable commentary on immutable old evidence");
    assert_eq!(
        server
            .api("PUT", "/api/investigation/notes", note.clone())
            .await
            .0,
        200
    );
    assert_eq!(
        server
            .api("PUT", "/api/investigation/notes", note.clone())
            .await
            .0,
        409
    );
    let (_, notes) = server
        .api("GET", "/api/investigation/notes", Value::Null)
        .await;
    assert_eq!(notes[0]["target"], note["target"]);
    assert_eq!(notes[0]["revision"], 2);
    std::fs::write(directory.path().join("source/invalid.bin"), [255]).unwrap();
    let file = json!({"type":"file","store":"source","path":"invalid.bin","representation":"text"});
    definition["revision"] = json!(2);
    definition["steps"][0]["inputs"]["input"] = file.clone();
    assert_eq!(
        server.api("PUT", "/api/explorations", definition).await.0,
        200
    );
    let failed = Uuid::new_v4();
    assert_ne!(
        server
            .api("POST", "/api/lab/v2/execute", invoke(failed, 3, file))
            .await
            .0,
        200
    );
    let (_, run) = server
        .api(
            "GET",
            &format!("/api/explorations/executions/{failed}"),
            Value::Null,
        )
        .await;
    assert_eq!(run["status"], "failed");
    assert!(run["evidence"]["error"].is_object());
    let failure_note = json!({"id":Uuid::new_v4(),"revision":0,"title":"Failed attempt","body":"No output was produced","target":{"execution_id":failed,"output_port":null}});
    assert_eq!(
        server
            .api("PUT", "/api/investigation/notes", failure_note.clone())
            .await
            .0,
        200
    );
    let mut invalid = failure_note;
    invalid["id"] = json!(Uuid::new_v4());
    invalid["target"]["output_port"] = json!("output");
    assert_eq!(
        server
            .api("PUT", "/api/investigation/notes", invalid)
            .await
            .0,
        400
    );
    drop(server);
    let server = Server::start(directory.path(), workspace);
    let (_, notes) = server
        .api("GET", "/api/investigation/notes", Value::Null)
        .await;
    let preserved = notes
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == note_id.to_string())
        .unwrap();
    assert_eq!(preserved["target"], note["target"]);
    assert_eq!(preserved["body"], note["body"]);
    std::fs::write(
        directory
            .path()
            .join(format!("artifacts/lab/{first}/value.bin")),
        b"corrupt",
    )
    .unwrap();
    assert_ne!(
        server
            .api("GET", &format!("/api/lab/results/{first}"), Value::Null)
            .await
            .0,
        200
    );
    note["revision"] = json!(2);
    note["body"] = json!("Evidence is unavailable; keep its reference for recovery");
    assert_eq!(
        server
            .api("PUT", "/api/investigation/notes", note.clone())
            .await
            .0,
        200
    );
    let (_, notes) = server
        .api("GET", "/api/investigation/notes", Value::Null)
        .await;
    assert_eq!(
        notes
            .as_array()
            .unwrap()
            .iter()
            .find(|n| n["id"] == note_id.to_string())
            .unwrap()["target"],
        note["target"]
    );
}

#[tokio::test]
async fn replay_checks_follow_actual_dependencies_and_report_changed_missing_and_private_inputs() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = Uuid::new_v4();
    let server = Server::start(directory.path(), workspace);
    let source = directory.path().join("source/replay.txt");
    std::fs::write(&source, b"original evidence").unwrap();
    let graph = Uuid::new_v4();
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    let file = json!({"type":"file","store":"source","path":"replay.txt","representation":"text"});
    let step = |id, input| json!({"id":id,"operation":"text.reverse","operation_version":1,"inputs":{"input":input},"parameters":{},"private_parameters":[],"position":[0,0]});
    let definition = json!({"schema_version":1,"id":graph,"revision":0,"title":"Replay branch","hypothesis":"","scope":null,"steps":[step(a,file.clone()),step(b,json!({"type":"step","step":a,"port":"output"}))]});
    let (status, saved) = server.api("PUT", "/api/explorations", definition).await;
    assert_eq!(status, 200, "{saved}");
    let invoke = |id, step, input| json!({"schema_version":2,"id":id,"operation":"text.reverse","operation_version":1,"inputs":{"input":input},"parameters":{},"exploration":{"id":graph,"revision":1,"step":step}});
    let first = Uuid::new_v4();
    let (status, result) = server
        .api("POST", "/api/lab/v2/execute", invoke(first, a, file))
        .await;
    assert_eq!(status, 200, "{result}");
    let second = Uuid::new_v4();
    let (status, result) = server
        .api(
            "POST",
            "/api/lab/v2/execute",
            invoke(
                second,
                b,
                json!({"type":"artifact","id":first,"port":"output"}),
            ),
        )
        .await;
    assert_eq!(status, 200, "{result}");
    let uri = format!("/api/lab/results/{second}/replay");
    let (status, available) = server.api("GET", &uri, Value::Null).await;
    assert_eq!(status, 200, "{available}");
    assert_eq!(available["status"], "inputs_available");
    assert_eq!(available["checked_artifacts"], 2);
    std::fs::write(&source, b"changed original").unwrap();
    let (_, changed) = server.api("GET", &uri, Value::Null).await;
    assert!(
        changed["gaps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|g| g["code"] == "source_changed")
    );
    assert_eq!(std::fs::read(&source).unwrap(), b"changed original");
    std::fs::remove_file(&source).unwrap();
    let (_, missing) = server.api("GET", &uri, Value::Null).await;
    assert!(
        missing["gaps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|g| g["code"] == "source_unavailable")
    );
    let (_, runs) = server
        .api(
            "GET",
            &format!("/api/explorations/{graph}/executions"),
            Value::Null,
        )
        .await;
    assert_eq!(
        runs.as_array().unwrap().len(),
        2,
        "Checks must not execute operations"
    );
    let private = Uuid::new_v4();
    let (status, private_result) = server.api("POST", "/api/lab/v2/execute", json!({"schema_version":2,"id":private,"operation":"cipher.vigenere.encode","operation_version":1,"inputs":{"input":{"type":"literal","value":"ABC"}},"parameters":{"key":"SECRET"},"retain_private_output":true})).await;
    assert_eq!(status, 200, "{private_result}");
    let (_, gaps) = server
        .api(
            "GET",
            &format!("/api/lab/results/{private}/replay"),
            Value::Null,
        )
        .await;
    for code in ["private_parameters_required", "input_unavailable"] {
        assert!(
            gaps["gaps"]
                .as_array()
                .unwrap()
                .iter()
                .any(|g| g["code"] == code),
            "{gaps}"
        );
    }
    assert!(!gaps.to_string().contains("SECRET"));
    drop(server);
    let server = Server::start(directory.path(), workspace);
    let (_, recovered) = server.api("GET", &uri, Value::Null).await;
    assert_eq!(recovered["status"], "gaps");
    let (status, retained) = server
        .api("GET", &format!("/api/lab/results/{second}"), Value::Null)
        .await;
    assert_eq!(status, 200, "{retained}");
    assert_eq!(retained["inline"], result["inline"]);
    let record_path = directory
        .path()
        .join(format!("artifacts/lab/{first}/record.json"));
    let original_record = std::fs::read(&record_path).unwrap();
    let mut drifted: Value = serde_json::from_slice(&original_record).unwrap();
    drifted["metadata"]["ports"][0]["schema_sha256"] = json!("0".repeat(64));
    std::fs::write(&record_path, drifted.to_string()).unwrap();
    let (_, drift) = server.api("GET", &uri, Value::Null).await;
    assert!(
        drift["gaps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|g| g["code"] == "contract_changed"),
        "{drift}"
    );
    std::fs::write(&record_path, original_record).unwrap();
    std::fs::write(
        directory
            .path()
            .join(format!("artifacts/lab/{first}/value.bin")),
        b"corrupted",
    )
    .unwrap();
    let (_, corrupt) = server.api("GET", &uri, Value::Null).await;
    assert!(
        corrupt["gaps"]
            .as_array()
            .unwrap()
            .iter()
            .any(|g| g["code"] == "artifact_unavailable"),
        "{corrupt}"
    );
}

#[tokio::test]
async fn named_image_outputs_preserve_samples_multiple_ports_and_restart_references() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = Uuid::new_v4();
    let server = Server::start(directory.path(), workspace);
    let bytes = include_bytes!("../../../tests/fixtures/sample.png");
    let source = directory.path().join("source/sample.png");
    std::fs::write(&source, bytes).unwrap();
    let file = json!({"type":"file","store":"source","path":"sample.png","representation":"image"});
    let request = |operation: &str, inputs: Value, parameters: Value| json!({"schema_version":2,"id":Uuid::new_v4(),"operation":operation,"operation_version":1,"inputs":inputs,"parameters":parameters});
    let (status, channel) = server
        .api(
            "POST",
            "/api/lab/v2/execute",
            request(
                "image.channel.extract",
                json!({"input":file}),
                json!({"channel":"red"}),
            ),
        )
        .await;
    assert_eq!(status, 200, "{channel}");
    assert_eq!(
        channel["result"]["metadata"]["ports"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    assert_eq!(
        channel["inline"]["data"]["output"]["channels"],
        json!(["red"])
    );
    assert_eq!(channel["inline"]["data"]["image"]["color_model"], "gray");
    assert_eq!(
        channel["result"]["provenance"]["inputs"][0]["reference"]["digest_representation"],
        "file_bytes"
    );
    let id = channel["result"]["id"].as_str().unwrap();
    let named = json!({"type":"artifact","id":id,"port":"image"});
    let (status, histogram) = server
        .api(
            "POST",
            "/api/lab/v2/execute",
            request("image.histogram", json!({"input":named}), json!({})),
        )
        .await;
    assert_eq!(status, 200, "{histogram}");
    assert_eq!(histogram["inline"]["data"]["output"]["pixels"], 4);
    assert_eq!(
        histogram["result"]["provenance"]["inputs"][0]["reference"]["digest_representation"],
        "json_wire_value"
    );
    let (status, compare) = server
        .api(
            "POST",
            "/api/lab/v2/execute",
            request(
                "image.compare",
                json!({"left":file,"right":file}),
                json!({"mask":true}),
            ),
        )
        .await;
    assert_eq!(status, 200, "{compare}");
    assert_eq!(compare["inline"]["data"]["output"]["equal"], true);
    assert!(compare["inline"]["data"]["mask"].is_object());
    assert_eq!(std::fs::read(&source).unwrap(), bytes);
    drop(server);
    let server = Server::start(directory.path(), workspace);
    let (status, reopened) = server
        .api("GET", &format!("/api/lab/results/{id}"), Value::Null)
        .await;
    assert_eq!(status, 200, "{reopened}");
    assert_eq!(reopened["inline"], channel["inline"]);
    assert_eq!(
        reopened["result"]["metadata"],
        channel["result"]["metadata"]
    );
}
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
        let _startup = SERVER_STARTUP
            .lock()
            .expect("Another Lab fixture failed during startup");
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
            .env("TOKIO_WORKER_THREADS", "2")
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
        let line = match receive.recv_timeout(Duration::from_secs(10)) {
            Ok(line) => line,
            Err(error) => {
                let _ = process.kill();
                let _ = process.wait();
                panic!(
                    "The real local server did not report readiness within ten seconds: {error}"
                );
            }
        };
        let Some(secret) = line.split("#session=").nth(1) else {
            let status = process.try_wait();
            let _ = process.kill();
            let _ = process.wait();
            panic!(
                "The real local server failed before readiness ({status:?}): {}",
                line.chars().take(1024).collect::<String>()
            );
        };
        let secret = secret.trim().to_owned();
        Self {
            process,
            address,
            secret,
        }
    }
    async fn raw(&self, method: &str, path: &str, body: &[u8], authorized: bool) -> (u16, Vec<u8>) {
        let mut socket = tokio::net::TcpStream::connect(&self.address).await.unwrap();
        let authorization = if authorized {
            format!("Authorization: Bearer {}\r\n", self.secret)
        } else {
            String::new()
        };
        let head = format!(
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nOrigin: http://{}\r\n{authorization}Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            self.address,
            self.address,
            body.len()
        );
        socket.write_all(head.as_bytes()).await.unwrap();
        socket.write_all(body).await.unwrap();
        let mut response = Vec::new();
        tokio::time::timeout(Duration::from_secs(12), socket.read_to_end(&mut response))
            .await
            .unwrap_or_else(|_| panic!("{method} {path} exceeded the real HTTP deadline"))
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

#[tokio::test]
async fn notebook_names_history_exports_conflicts_and_missing_references_survive_restart() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = Uuid::new_v4();
    let server = Server::start(directory.path(), workspace);
    let (_, result) = server
        .execute(
            "encoding.base64.decode",
            json!({"type":"text","data":"AP+A"}),
            json!([]),
        )
        .await;
    let id = result["result"]["id"].as_str().unwrap();
    let name = json!({"id":id,"title":"Private bytes","revision":0});
    assert_eq!(
        server.api("PUT", "/api/lab/library", name.clone()).await.0,
        200
    );
    assert_eq!(server.api("PUT", "/api/lab/library", name).await.0, 409);
    assert_eq!(
        server
            .api(
                "PUT",
                "/api/lab/library",
                json!({"id":id,"title":"Renamed bytes","revision":1})
            )
            .await
            .1["revision"],
        2
    );
    assert_eq!(
        server
            .api(
                "PUT",
                "/api/lab/library",
                json!({"id":Uuid::new_v4(),"title":"Foreign","revision":0})
            )
            .await
            .0,
        404
    );
    let (_, history) = server.api("GET", "/api/lab/history", Value::Null).await;
    assert_eq!(history["records"].as_array().unwrap().len(), 1);
    let (status, bytes) = server
        .raw(
            "GET",
            &format!("/api/lab/results/{id}/export?name=private.bin"),
            b"",
            true,
        )
        .await;
    assert_eq!(status, 200);
    assert_eq!(bytes, b"\0\xff\x80");
    for invalid in ["../escape", "CON", "dir/file", "%0d%0aBad", "x%22name"] {
        assert_ne!(
            server
                .raw(
                    "GET",
                    &format!("/api/lab/results/{id}/export?name={invalid}"),
                    b"",
                    true
                )
                .await
                .0,
            200
        );
    }
    let note = Uuid::new_v4();
    assert_eq!(server.api("PUT","/api/investigation/notes",json!({"id":note,"title":"Hypothesis","body":"<script>never execute</script>","revision":0,"target":{"store":"source","path":"missing.png"}})).await.0,200);
    assert_eq!(
        server
            .api(
                "PUT",
                "/api/investigation/notes",
                json!({"id":note,"title":"Stale","body":"lost","revision":0})
            )
            .await
            .0,
        409
    );
    assert_eq!(server.api("PUT","/api/investigation/notes",json!({"id":Uuid::new_v4(),"title":"Escape","body":"","revision":0,"target":{"store":"source","path":"../private"}})).await.0,400);
    drop(server);
    let server = Server::start(directory.path(), workspace);
    let (_, library) = server.api("GET", "/api/lab/library", Value::Null).await;
    assert_eq!(library["entries"][0]["entry"]["title"], "Renamed bytes");
    assert_eq!(library["entries"][0]["record"], result["result"]);
    let (_, notes) = server
        .api("GET", "/api/investigation/notes", Value::Null)
        .await;
    assert_eq!(notes[0]["target"]["path"], "missing.png");
    assert_eq!(notes[0]["body"], "<script>never execute</script>");
    std::fs::remove_file(
        directory
            .path()
            .join(format!("artifacts/lab/{id}/value.bin")),
    )
    .unwrap();
    let (_, library) = server.api("GET", "/api/lab/library", Value::Null).await;
    assert_eq!(library["entries"][0]["entry"]["id"], id);
    assert!(library["entries"][0]["record"].is_null());
    assert!(library["entries"][0]["error"].is_string());
    assert_eq!(
        server.api("GET", "/api/lab/history", Value::Null).await.0,
        503
    );
}

fn challenge_manifest() -> Value {
    json!({"kind":"cube-play-challenge","format_version":1,"id":"original","title":"Original local challenge","description":"No private answer or executable content.","artifacts":[{"id":"image","path":"sample.png"}],"stages":[{"id":"first","title":"Inspect","instructions":"<script>never execute</script>","steps":[{"id":"read","instructions":"Read the image."}],"hints":["Try the green channel."],"artifacts":["image"],"answer_type":"bytes"},{"id":"second","title":"Record","instructions":"Keep a local hypothesis."}]})
}
#[tokio::test]
async fn non_executable_challenge_validation_navigation_and_resume() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = Uuid::new_v4();
    let server = Server::start(directory.path(), workspace);
    let source = directory.path().join("source");
    std::fs::write(
        source.join("sample.png"),
        include_bytes!("../../../tests/fixtures/sample.png"),
    )
    .unwrap();
    let manifest = challenge_manifest();
    std::fs::write(source.join("rabbit.cube"), manifest.to_string()).unwrap();
    let import = json!({"store":"source","path":"rabbit.cube"});
    let (status, state) = server
        .api("POST", "/api/challenges/import", import.clone())
        .await;
    assert_eq!(status, 200, "{state}");
    let key = state["active"]["definition"]["key"].as_str().unwrap();
    assert_eq!(state["active"]["artifacts"][0]["path"], "sample.png");
    assert_eq!(state["active"]["stage"], "first");
    assert_eq!(
        server
            .api(
                "PUT",
                "/api/challenges/stage",
                json!({"key":key,"stage":"second","revision":1})
            )
            .await
            .0,
        200
    );
    assert_eq!(
        server
            .api(
                "PUT",
                "/api/challenges/stage",
                json!({"key":key,"stage":"first","revision":1})
            )
            .await
            .0,
        409
    );
    assert_eq!(
        server
            .api(
                "PUT",
                "/api/challenges/stage",
                json!({"key":key,"stage":"missing","revision":2})
            )
            .await
            .0,
        404
    );
    assert_eq!(
        server
            .api(
                "PUT",
                "/api/challenges/progress",
                json!({"key":key,"stage":"second","revision":0,"status":"in_progress"})
            )
            .await
            .0,
        200
    );
    assert_eq!(
        server
            .api(
                "PUT",
                "/api/challenges/progress",
                json!({"key":key,"stage":"missing","revision":0,"status":"completed"})
            )
            .await
            .0,
        400
    );
    let mut cases = Vec::new();
    for field in ["script", "commands", "answers"] {
        let mut bad = manifest.clone();
        bad[field] = json!("never execute");
        cases.push(bad);
    }
    let mut bad = manifest.clone();
    bad["format_version"] = json!(2);
    cases.push(bad);
    let mut bad = manifest.clone();
    bad["stages"][1]["id"] = json!("first");
    cases.push(bad);
    let mut bad = manifest.clone();
    bad["artifacts"][0]["path"] = json!("../outside");
    cases.push(bad);
    let mut bad = manifest.clone();
    bad["artifacts"][0]["path"] = json!("missing.png");
    cases.push(bad);
    let mut bad = manifest.clone();
    bad["stages"][0]["artifacts"] = json!(["absent"]);
    cases.push(bad);
    let mut bad = manifest.clone();
    bad["stages"][0]["hints"] = json!(["x".repeat(8193)]);
    cases.push(bad);
    let mut bad = manifest.clone();
    bad["stages"] = json!([]);
    cases.push(bad);
    for bad in cases {
        std::fs::write(source.join("bad.cube"), bad.to_string()).unwrap();
        assert_ne!(
            server
                .api(
                    "POST",
                    "/api/challenges/import",
                    json!({"store":"source","path":"bad.cube"})
                )
                .await
                .0,
            200
        );
    }
    for bad in [
        "{\"kind\":\"x\",\"kind\":\"cube-play-challenge\"}".to_owned(),
        "[".repeat(32),
        " ".repeat(256 * 1024 + 1),
    ] {
        std::fs::write(source.join("bad.cube"), bad).unwrap();
        assert_ne!(
            server
                .api(
                    "POST",
                    "/api/challenges/import",
                    json!({"store":"source","path":"bad.cube"})
                )
                .await
                .0,
            200
        );
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("/etc/passwd", source.join("escape")).unwrap();
        let mut bad = manifest.clone();
        bad["artifacts"][0]["path"] = json!("escape");
        std::fs::write(source.join("bad.cube"), bad.to_string()).unwrap();
        assert_eq!(
            server
                .api(
                    "POST",
                    "/api/challenges/import",
                    json!({"store":"source","path":"bad.cube"})
                )
                .await
                .0,
            403
        );
    }
    assert_eq!(
        server.api("GET", "/api/challenges", Value::Null).await.1["challenges"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    drop(server);
    let server = Server::start(directory.path(), workspace);
    let (status, state) = server.api("POST", "/api/challenges/import", import).await;
    assert_eq!(status, 200, "{state}");
    assert_eq!(state["active"]["stage"], "second");
    assert_eq!(
        state["active"]["progress"][0]["local_status"],
        "in_progress"
    );
    assert_eq!(
        std::fs::read_to_string(source.join("rabbit.cube")).unwrap(),
        manifest.to_string()
    );
}

#[tokio::test]
async fn candidates_preserve_exact_types_explicit_fields_history_and_workspace_references() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = Uuid::new_v4();
    let server = Server::start(directory.path(), workspace);
    let source = directory.path().join("source");
    std::fs::write(
        source.join("sample.png"),
        include_bytes!("../../../tests/fixtures/sample.png"),
    )
    .unwrap();
    std::fs::write(source.join("rabbit.cube"), challenge_manifest().to_string()).unwrap();
    let (_, state) = server
        .api(
            "POST",
            "/api/challenges/import",
            json!({"store":"source","path":"rabbit.cube"}),
        )
        .await;
    let key = state["active"]["definition"]["key"].as_str().unwrap();
    let (_, binary) = server
        .execute(
            "encoding.base64.decode",
            json!({"type":"text","data":"AP+A"}),
            json!([]),
        )
        .await;
    let binary_id = binary["result"]["id"].as_str().unwrap();
    assert_eq!(
        server
            .api(
                "PUT",
                "/api/lab/library",
                json!({"id":binary_id,"title":"Private binary","revision":0})
            )
            .await
            .0,
        200
    );
    let candidate = json!({"key":key,"stage":"first","result_id":binary_id,"revision":0,"selection":{"kind":"whole"}});
    let (status, selected) = server
        .api("PUT", "/api/challenges/candidates", candidate.clone())
        .await;
    assert_eq!(status, 200, "{selected}");
    assert_eq!(selected["candidate"]["byte_size"], 3);
    assert_eq!(
        selected["candidate"]["sha256"],
        cube_core::hashing::sha256(&[0, 255, 128])
    );
    assert_eq!(selected["candidate"]["status"], "private_unverified");
    let mut invalid = candidate.clone();
    invalid["stage"] = json!("second");
    assert_eq!(
        server
            .api("PUT", "/api/challenges/candidates", invalid)
            .await
            .0,
        400
    );
    let (_, text) = server
        .execute(
            "encoding.base64.encode",
            json!({"type":"artifact","id":binary_id}),
            json!([]),
        )
        .await;
    let text_id = text["result"]["id"].as_str().unwrap();
    assert_eq!(
        server
            .api(
                "PUT",
                "/api/lab/library",
                json!({"id":text_id,"title":"Text","revision":0})
            )
            .await
            .0,
        200
    );
    assert_eq!(server.api("PUT","/api/challenges/candidates",json!({"key":key,"stage":"second","result_id":text_id,"revision":0,"selection":{"kind":"whole"}})).await.0,200);
    let (_, channel) = server
        .execute(
            "image.channel.extract",
            json!({"type":"file","store":"source","path":"sample.png","representation":"image"}),
            json!([{"name":"channel","value":"green"}]),
        )
        .await;
    let channel_id = channel["result"]["id"].as_str().unwrap();
    assert_eq!(
        server
            .api(
                "PUT",
                "/api/lab/library",
                json!({"id":channel_id,"title":"Channel","revision":0})
            )
            .await
            .0,
        200
    );
    let field = json!({"key":key,"stage":"first","result_id":channel_id,"revision":1,"selection":{"kind":"field","pointer":"/preview_png_base64","conversion":"base64_bytes"}});
    let (status, revised) = server
        .api("PUT", "/api/challenges/candidates", field.clone())
        .await;
    assert_eq!(status, 200, "{revised}");
    assert_eq!(revised["history"].as_array().unwrap().len(), 2);
    assert_eq!(revised["candidate"]["revision"], 2);
    let field_text = json!({"key":key,"stage":"second","result_id":channel_id,"revision":1,"selection":{"kind":"field","pointer":"/channels/0","conversion":"text"}});
    let (status, text_candidate) = server
        .api("PUT", "/api/challenges/candidates", field_text)
        .await;
    assert_eq!(status, 200, "{text_candidate}");
    assert_eq!(
        text_candidate["candidate"]["sha256"],
        cube_core::hashing::sha256(b"green")
    );
    assert_eq!(
        server
            .api("PUT", "/api/challenges/candidates", field)
            .await
            .0,
        409
    );
    for (pointer, conversion) in [
        ("/absent", "text"),
        ("/bit_depth", "text"),
        ("/channels/0", "base64_bytes"),
        ("/channels/0", "text"),
    ] {
        assert_eq!(server.api("PUT","/api/challenges/candidates",json!({"key":key,"stage":"first","result_id":channel_id,"revision":2,"selection":{"kind":"field","pointer":pointer,"conversion":conversion}})).await.0,400);
    }
    let mut invalid = candidate.clone();
    invalid["revision"] = json!(2);
    invalid["result_id"] = json!(Uuid::new_v4());
    assert_eq!(
        server
            .api("PUT", "/api/challenges/candidates", invalid)
            .await
            .0,
        404
    );
    let (_, unsaved) = server
        .execute(
            "encoding.base64.decode",
            json!({"type":"text","data":"AA=="}),
            json!([]),
        )
        .await;
    let mut invalid = candidate.clone();
    invalid["revision"] = json!(2);
    invalid["result_id"] = unsaved["result"]["id"].clone();
    assert_eq!(
        server
            .api("PUT", "/api/challenges/candidates", invalid)
            .await
            .0,
        409
    );
    std::fs::write(
        source.join("large.b64"),
        cube_core::encoding::base64_encode(&vec![0; 65537]),
    )
    .unwrap();
    let (status, large) = server
        .execute(
            "encoding.base64.decode",
            json!({"type":"file","store":"source","path":"large.b64","representation":"text"}),
            json!([]),
        )
        .await;
    assert_eq!(status, 200, "{large}");
    assert_eq!(
        server
            .api(
                "PUT",
                "/api/lab/library",
                json!({"id":large["result"]["id"],"title":"Large","revision":0})
            )
            .await
            .0,
        200
    );
    let mut invalid = candidate.clone();
    invalid["revision"] = json!(2);
    invalid["result_id"] = large["result"]["id"].clone();
    assert_eq!(
        server
            .api("PUT", "/api/challenges/candidates", invalid)
            .await
            .0,
        400
    );
    let path = format!("/api/challenges/candidates?key={key}&stage=first");
    drop(server);
    let server = Server::start(directory.path(), workspace);
    let (_, restored) = server.api("GET", &path, Value::Null).await;
    assert_eq!(restored["candidate"], revised["candidate"]);
    assert!(restored["error"].is_null());
    assert_eq!(
        server
            .api("GET", &format!("/api/lab/results/{binary_id}"), Value::Null)
            .await
            .0,
        200
    );
    std::fs::remove_file(
        directory
            .path()
            .join(format!("artifacts/lab/{channel_id}/value.bin")),
    )
    .unwrap();
    let (_, missing) = server.api("GET", &path, Value::Null).await;
    assert_eq!(missing["candidate"], revised["candidate"]);
    assert!(missing["error"].is_string());
}

#[tokio::test]
async fn named_ports_execute_real_core_and_restore_immutable_branch_lineage() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = Uuid::new_v4();
    let server = Server::start(directory.path(), workspace);
    let (_, catalog) = server
        .api("GET", "/api/lab/v2/operations", Value::Null)
        .await;
    let operations = catalog["operations"].as_array().unwrap();
    assert_eq!(operations.len(), cube_core::operations::catalog().len());
    assert!(
        operations
            .iter()
            .all(|o| !o["contract"]["outputs"].as_array().unwrap().is_empty())
    );
    assert!(
        operations
            .iter()
            .find(|o| o["contract"]["operation"] == "cipher.rot5")
            .unwrap()["execution"]["available"]
            .as_bool()
            .unwrap()
    );
    let exploration = Uuid::new_v4();
    let first = Uuid::new_v4();
    let branch = Uuid::new_v4();
    let mut definition = json!({"schema_version":1,"id":exploration,"revision":0,"title":"Digit hypothesis","hypothesis":"ROT5 is its own inverse","scope":null,"steps":[
        {"id":first,"operation":"cipher.rot5","operation_version":1,"inputs":{"text":{"type":"literal","value":"A1"}},"parameters":{},"position":[0,0]},
        {"id":branch,"operation":"cipher.rot5","operation_version":1,"inputs":{"text":{"type":"step","step":first,"port":"text"}},"parameters":{},"position":[200,0]}
    ]});
    let (status, saved) = server
        .api("PUT", "/api/explorations", definition.clone())
        .await;
    assert_eq!(status, 200, "{saved}");
    assert_eq!(saved["revision"], 1);
    let request = |step, inputs: Value| json!({"schema_version":2,"id":Uuid::new_v4(),"operation":"cipher.rot5","operation_version":1,"inputs":inputs,"parameters":{},"exploration":{"id":exploration,"revision":1,"step":step}});
    let (status, result) = server
        .api(
            "POST",
            "/api/lab/v2/execute",
            request(first, json!({"text":{"type":"literal","value":"A1"}})),
        )
        .await;
    assert_eq!(status, 200, "{result}");
    assert_eq!(result["inline"]["data"]["text"], "A6");
    assert_eq!(
        result["result"]["metadata"]["ports"][0]["schema_sha256"]
            .as_str()
            .unwrap()
            .len(),
        64
    );
    let result_id = result["result"]["id"].clone();
    let (status, inverse) = server
        .api(
            "POST",
            "/api/lab/v2/execute",
            request(
                branch,
                json!({"text":{"type":"artifact","id":result_id,"port":"text"}}),
            ),
        )
        .await;
    assert_eq!(status, 200, "{inverse}");
    assert_eq!(inverse["inline"]["data"]["text"], "A1");
    let runs_path = format!("/api/explorations/{exploration}/executions");
    let (_, runs) = server.api("GET", &runs_path, Value::Null).await;
    assert_eq!(runs.as_array().unwrap().len(), 2);
    assert_eq!(runs[0]["status"], "succeeded");
    // A caller cannot attribute a different input to a saved step.
    assert_eq!(
        server
            .api(
                "POST",
                "/api/lab/v2/execute",
                request(first, json!({"text":{"type":"literal","value":"forged"}}))
            )
            .await
            .0,
        409
    );
    definition["revision"] = json!(1);
    definition["hypothesis"] = json!("An edited hypothesis");
    let (status, revised) = server
        .api("PUT", "/api/explorations", definition.clone())
        .await;
    assert_eq!(status, 200, "{revised}");
    assert_eq!(revised["revision"], 2);
    assert_eq!(
        server.api("PUT", "/api/explorations", definition).await.0,
        409
    );
    drop(server);
    let server = Server::start(directory.path(), workspace);
    let (_, restored) = server
        .api(
            "GET",
            &format!("/api/explorations/{exploration}"),
            Value::Null,
        )
        .await;
    assert_eq!(restored["hypothesis"], "An edited hypothesis");
    let (_, runs) = server.api("GET", &runs_path, Value::Null).await;
    assert_eq!(runs[0]["evidence"]["result_id"], result_id);
    assert_eq!(runs[0]["stale"], true);
    let (_, original) = server
        .api(
            "GET",
            &format!("/api/lab/results/{}", result_id.as_str().unwrap()),
            Value::Null,
        )
        .await;
    assert_eq!(original["inline"]["data"]["text"], "A6");
}

#[tokio::test]
async fn typed_invocations_reject_wrong_types_paths_versions_and_unsafe_graphs() {
    let directory = tempfile::tempdir().unwrap();
    let server = Server::start(directory.path(), Uuid::new_v4());
    let mut request = json!({"schema_version":2,"id":Uuid::new_v4(),"operation":"cipher.rot5","operation_version":1,"inputs":{"text":{"type":"literal","value":42}},"parameters":{}});
    assert_eq!(
        server
            .api("POST", "/api/lab/v2/execute", request.clone())
            .await
            .0,
        400
    );
    request["inputs"] = json!({"text":{"type":"file","store":"source","path":"../outside","representation":"text"}});
    assert_eq!(
        server
            .api("POST", "/api/lab/v2/execute", request.clone())
            .await
            .0,
        400
    );
    request["operation_version"] = json!(99);
    assert_eq!(
        server
            .api("POST", "/api/lab/v2/execute", request.clone())
            .await
            .0,
        409
    );
    request["operation_version"] = json!(1);
    request["inputs"] = json!({"text":{"type":"literal","value":"A"}});
    request["parameters"] = json!({"script":"unsafe"});
    assert_eq!(
        server.api("POST", "/api/lab/v2/execute", request).await.0,
        400
    );
    let node = Uuid::new_v4();
    let definition = json!({"schema_version":1,"id":Uuid::new_v4(),"revision":0,"title":"Cycle","hypothesis":"","scope":null,"steps":[{"id":node,"operation":"cipher.rot5","operation_version":1,"inputs":{"text":{"type":"step","step":node,"port":"text"}},"parameters":{},"position":[0,0]}]});
    assert_eq!(
        server.api("PUT", "/api/explorations", definition).await.0,
        400
    );
    let valid = json!({"schema_version":1,"id":Uuid::new_v4(),"revision":0,"title":"duplicate-test","hypothesis":"","scope":null,"steps":[]});
    let duplicate = valid.to_string().replace(
        "\"title\":\"duplicate-test\"",
        "\"title\":\"one\",\"title\":\"two\"",
    );
    assert_eq!(
        server
            .raw("PUT", "/api/explorations", duplicate.as_bytes(), true)
            .await
            .0,
        400
    );
    // Invalid data never acquires a successful result or an invented execution.
    let (_, history) = server.api("GET", "/api/lab/history", Value::Null).await;
    assert!(
        history
            .get("results")
            .and_then(Value::as_array)
            .is_none_or(Vec::is_empty)
    );
}

#[tokio::test]
async fn substitution_parameter_variants_are_saved_and_executed_by_the_real_named_worker() {
    let directory = tempfile::tempdir().unwrap();
    let server = Server::start(directory.path(), Uuid::new_v4());
    for (parameters, source, expected) in [
        (json!({"source":"ABC","target":"CAB"}), "ABC", "CAB"),
        (json!({"source":"ABC","keyword":"CAB"}), "ABC", "CAB"),
        (json!({"source":"ABC","keyword":""}), "ABC", "ABC"),
        (json!({"source":"ABC","keyword":""}), "", ""),
    ] {
        let id = Uuid::new_v4();
        let step = Uuid::new_v4();
        let inputs = json!({"text":{"type":"literal","value":source}});
        let definition = json!({"schema_version":1,"id":id,"revision":0,"title":"Substitution hypothesis","hypothesis":"","scope":null,"steps":[{"id":step,"operation":"cipher.substitution.encode","operation_version":1,"inputs":inputs,"parameters":parameters,"position":[0,0]}]});
        let (status, saved) = server.api("PUT", "/api/explorations", definition).await;
        assert_eq!(status, 200, "{saved}");
        let request = json!({"schema_version":2,"id":Uuid::new_v4(),"operation":"cipher.substitution.encode","operation_version":1,"inputs":inputs,"parameters":parameters,"exploration":{"id":id,"revision":1,"step":step}});
        let (status, result) = server.api("POST", "/api/lab/v2/execute", request).await;
        assert_eq!(status, 200, "{result}");
        assert_eq!(result["inline"]["data"]["text"], expected);
    }
}

#[tokio::test]
async fn transposition_fractionation_keep_evidence_across_real_worker_and_restart() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = Uuid::new_v4();
    let server = Server::start(directory.path(), workspace);
    let request = |op: &str, inputs: Value, parameters: Value| json!({"schema_version":2,"id":Uuid::new_v4(),"operation":op,"operation_version":1,"inputs":inputs,"parameters":parameters});
    let (status, encoded) = server
        .api(
            "POST",
            "/api/lab/v2/execute",
            request(
                "cipher.rail_fence.encode",
                json!({"text":{"type":"literal","value":"WEAREDISCOVEREDFLEEATONCE"}}),
                json!({"rails":3,"initial_rail":0,"direction":"down"}),
            ),
        )
        .await;
    assert_eq!(status, 200, "{encoded}");
    assert_eq!(
        encoded["inline"]["data"]["text"],
        "WECRLTEERDSOEEFEAOCAIVDEN"
    );
    let id = encoded["result"]["id"].clone();
    drop(server);
    let server = Server::start(directory.path(), workspace);
    let (status, decoded) = server
        .api(
            "POST",
            "/api/lab/v2/execute",
            request(
                "cipher.rail_fence.decode",
                json!({"text":{"type":"artifact","id":id,"port":"text"}}),
                json!({"rails":3,"initial_rail":0,"direction":"down"}),
            ),
        )
        .await;
    assert_eq!(status, 200, "{decoded}");
    assert_eq!(
        decoded["inline"]["data"]["text"],
        "WEAREDISCOVEREDFLEEATONCE"
    );
    let (status, ambiguous) = server
        .api(
            "POST",
            "/api/lab/v2/execute",
            request(
                "cipher.morbit.decode",
                json!({"text":{"type":"literal","value":"1"}}),
                json!({"digits":"123456789"}),
            ),
        )
        .await;
    assert_eq!(status, 200, "{ambiguous}");
    assert!(ambiguous["inline"]["data"]["text"].is_null());
    assert_eq!(ambiguous["inline"]["data"]["padding_ambiguous"], true);
    let (status, exact) = server
        .api(
            "POST",
            "/api/lab/v2/execute",
            request(
                "cipher.skip.encode",
                json!({"text":{"type":"literal","value":"ABCDE"}}),
                json!({"start":"0","step":"-9223372036854775808"}),
            ),
        )
        .await;
    assert_eq!(status, 200, "{exact}");
    assert_eq!(exact["inline"]["data"]["text"], "ACEBD");
    let (status, bad) = server
        .api(
            "POST",
            "/api/lab/v2/execute",
            request(
                "cipher.skip.encode",
                json!({"text":{"type":"literal","value":"ABCD"}}),
                json!({"start":0,"step":2}),
            ),
        )
        .await;
    assert_eq!(status, 400, "{bad}");
}

#[tokio::test]
async fn caller_owned_puzzle_ports_preserve_sources_arrays_aliases_and_restart() {
    let directory = tempfile::tempdir().unwrap();
    let workspace = Uuid::new_v4();
    let server = Server::start(directory.path(), workspace);
    let request = |op: &str, inputs: Value, parameters: Value| json!({"schema_version":2,"id":Uuid::new_v4(),"operation":op,"operation_version":1,"inputs":inputs,"parameters":parameters});
    let cases: Vec<Value> =
        serde_json::from_str(include_str!("../src/lab/domains/puzzle_codes_vectors.json")).unwrap();
    let book = &cases[0];
    let literals = |v: &Value| {
        Value::Object(
            v.as_object()
                .unwrap()
                .iter()
                .map(|(k, v)| (k.clone(), json!({"type":"literal","value":v})))
                .collect(),
        )
    };
    let (status, encoded) = server
        .api(
            "POST",
            "/api/lab/v2/execute",
            request(
                book["op"].as_str().unwrap(),
                literals(&book["inputs"]),
                book["parameters"].clone(),
            ),
        )
        .await;
    assert_eq!(status, 200, "{encoded}");
    assert_eq!(encoded["inline"]["data"], book["outputs"]);
    let id = encoded["result"]["id"].clone();
    drop(server);
    let server = Server::start(directory.path(), workspace);
    let (status, decoded) = server
        .api(
            "POST",
            "/api/lab/v2/execute",
            request(
                "cipher.book.decode",
                json!({"input":{"type":"artifact","id":id,"port":"coordinates"}}),
                json!({"source":book["inputs"]["source"],"scheme":"word_character","index_base":1}),
            ),
        )
        .await;
    assert_eq!(status, 200, "{decoded}");
    assert_eq!(decoded["inline"]["data"]["output"], book["inputs"]["text"]);
    let coords = cases
        .iter()
        .find(|c| c["op"] == "text.keyboard.coordinates.encode")
        .unwrap();
    let (status, encoded) = server
        .api(
            "POST",
            "/api/lab/v2/execute",
            request(
                "text.keyboard.coordinates.encode",
                literals(&coords["inputs"]),
                coords["parameters"].clone(),
            ),
        )
        .await;
    assert_eq!(status, 200, "{encoded}");
    assert_eq!(encoded["inline"]["data"], coords["outputs"]);
    let (status,decoded)=server.api("POST","/api/lab/v2/execute",request("text.keyboard.coordinates.decode",json!({"coordinates":{"type":"artifact","id":encoded["result"]["id"],"port":"coordinates"}}),coords["parameters"].clone())).await;
    assert_eq!(status, 200, "{decoded}");
    assert_eq!(decoded["inline"]["data"]["text"], coords["inputs"]["text"]);
    for c in cases.iter().skip(1) {
        let (status, result) = server
            .api(
                "POST",
                "/api/lab/v2/execute",
                request(
                    c["op"].as_str().unwrap(),
                    literals(&c["inputs"]),
                    c["parameters"].clone(),
                ),
            )
            .await;
        assert_eq!(status, 200, "{result}");
        assert_eq!(result["inline"]["data"], c["outputs"]);
    }
    let token = cases
        .iter()
        .find(|c| c["op"] == "symbol.tokens.decode")
        .unwrap();
    let (status, bad) = server
        .api(
            "POST",
            "/api/lab/v2/execute",
            request(
                "symbol.tokens.encode",
                json!({"text":{"type":"literal","value":"A"}}),
                token["parameters"].clone(),
            ),
        )
        .await;
    assert_eq!(status, 400, "{bad}");
    let (status, bad) = server
        .api(
            "POST",
            "/api/lab/v2/execute",
            request(
                "text.keyboard.coordinates.decode",
                json!({"coordinates":{"type":"literal","value":"1,1,base"}}),
                coords["parameters"].clone(),
            ),
        )
        .await;
    assert_eq!(status, 400, "{bad}");
    let (status, catalog) = server
        .api("GET", "/api/lab/v2/operations", Value::Null)
        .await;
    assert_eq!(status, 200);
    let ops = catalog["operations"].as_array().unwrap();
    assert_eq!(ops.len(), 295);
    assert_eq!(
        ops.iter()
            .filter(|v| v["execution"]["available"] == true)
            .count(),
        203
    );
}
