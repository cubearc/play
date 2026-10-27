use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use play_local::{app, config::Config};
use serde_json::Value;
use tower::ServiceExt;
const SECRET: &str = "terminal-private-session";
#[tokio::test]
async fn native_development_does_not_start_host_processes() {
    let config = Config::parse(
        "127.0.0.1:7331",
        "http://127.0.0.1:7331",
        "unused".into(),
        false,
    )
    .unwrap();
    let application = app(config, SECRET.into());
    let response = application
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/terminal")
                .header("Host", "127.0.0.1:7331")
                .header("Authorization", format!("Bearer {SECRET}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let value: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(value["available"], false);
    assert_eq!(value["limit"], 2);
    assert_eq!(
        application
            .oneshot(
                Request::builder()
                    .uri("/api/terminal")
                    .header("Host", "127.0.0.1:7331")
                    .body(Body::empty())
                    .unwrap()
            )
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
}
#[cfg(unix)]
#[tokio::test]
async fn websocket_bridge_enforces_authorization_and_preserves_streams() {
    use futures_util::{SinkExt, StreamExt};
    use play_local::config::WorkspaceContext;
    use play_runner::{
        protocol::{Client, Server},
        transport::{channel, receive, send},
    };
    use serde_json::json;
    use std::os::unix::fs::PermissionsExt;
    use tokio_tungstenite::{
        connect_async,
        tungstenite::{Message, client::IntoClientRequest},
    };
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    let source = root.join("source");
    std::fs::create_dir(&source).unwrap();
    let socket = root.join("runner.sock");
    let listener = tokio::net::UnixListener::bind(&socket).unwrap();
    std::fs::set_permissions(&socket, std::fs::Permissions::from_mode(0o600)).unwrap();
    let (events, mut received) = tokio::sync::mpsc::channel(16);
    // This protocol peer tests forwarding; real CLI/PTYS are exercised by Linux/Docker tests.
    let runner = tokio::spawn(async move {
        loop {
            let (socket, _) = listener.accept().await.unwrap();
            let events = events.clone();
            tokio::spawn(async move {
                let mut socket = channel(socket);
                match receive::<Client>(&mut socket).await.unwrap() {
                    Some(Client::Probe) => {
                        send(
                            &mut socket,
                            &Server::Status {
                                active: 0,
                                limit: 2,
                            },
                        )
                        .await
                        .unwrap();
                    }
                    Some(Client::Start { size, .. }) => {
                        send(
                            &mut socket,
                            &Server::Ready {
                                id: uuid::Uuid::new_v4(),
                                size,
                            },
                        )
                        .await
                        .unwrap();
                        send(
                            &mut socket,
                            &Server::Output {
                                data: vec![0, 255, 128],
                            },
                        )
                        .await
                        .unwrap();
                        while let Some(frame) = receive::<Client>(&mut socket).await.unwrap() {
                            let closed = matches!(frame, Client::Close);
                            events.send(frame).await.unwrap();
                            if closed {
                                break;
                            }
                        }
                    }
                    _ => {}
                }
            });
        }
    });
    let tcp = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = tcp.local_addr().unwrap();
    let origin = format!("http://{address}");
    let mut config = Config::parse(&address.to_string(), &origin, "unused".into(), false).unwrap();
    config.runner_socket = Some(socket);
    config.source_dir = Some(source);
    config.data_dir = Some(root.join("imports"));
    config.workspace = Some(WorkspaceContext {
        id: uuid::Uuid::new_v4(),
        name: "Bridge".into(),
    });
    let server = tokio::spawn(async move {
        axum::serve(tcp, app(config, SECRET.into())).await.unwrap();
    });
    let url = format!("ws://{address}/api/terminal/ws?cols=80&rows=24");
    let make = |url: &str, capability: bool, origin: Option<&str>| {
        let mut request = url.into_client_request().unwrap();
        if capability {
            request.headers_mut().insert(
                "sec-websocket-protocol",
                format!("cube-play.v1, cube-play.auth.{SECRET}")
                    .parse()
                    .unwrap(),
            );
        }
        if let Some(origin) = origin {
            request
                .headers_mut()
                .insert("origin", origin.parse().unwrap());
        }
        request
    };
    for (request, expected) in [
        (make(&url, false, Some(&origin)), 401),
        (make(&url, true, Some("http://attacker.invalid")), 403),
        (make(&url, true, None), 403),
        (
            make(&format!("{url}&path=..%2fescape"), true, Some(&origin)),
            400,
        ),
        (
            make(&format!("{url}&session_id=foreign"), true, Some(&origin)),
            400,
        ),
        (
            make(&url.replace("cols=80", "cols=0"), true, Some(&origin)),
            400,
        ),
    ] {
        let error = connect_async(request).await.unwrap_err();
        assert!(
            matches!(error,tokio_tungstenite::tungstenite::Error::Http(response) if response.status().as_u16()==expected)
        );
    }
    let (mut ws, response) = connect_async(make(&url, true, Some(&origin)))
        .await
        .unwrap();
    assert_eq!(response.headers()["sec-websocket-protocol"], "cube-play.v1");
    assert_eq!(
        serde_json::from_str::<Value>(ws.next().await.unwrap().unwrap().to_text().unwrap())
            .unwrap()["type"],
        "ready"
    );
    assert_eq!(
        ws.next().await.unwrap().unwrap().into_data().as_ref(),
        [0, 255, 128]
    );
    ws.send(Message::Text(
        json!({"type":"input","data":"cube --help\r"})
            .to_string()
            .into(),
    ))
    .await
    .unwrap();
    assert!(matches!(received.recv().await,Some(Client::Input{data}) if data=="cube --help\r"));
    ws.send(Message::Text(
        json!({"type":"resize","size":{"cols":100,"rows":30}})
            .to_string()
            .into(),
    ))
    .await
    .unwrap();
    assert!(
        matches!(received.recv().await,Some(Client::Resize{size}) if size.cols==100 && size.rows==30)
    );
    ws.close(None).await.unwrap();
    assert!(matches!(
        tokio::time::timeout(std::time::Duration::from_secs(3), received.recv())
            .await
            .unwrap(),
        Some(Client::Close)
    ));
    server.abort();
    runner.abort();
}
