use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode, header},
};
use http_body_util::BodyExt;
use play_local::{app, config::Config};
use tower::ServiceExt;

const SECRET: &str = "70b860a2-7365-47cb-92fb-4c2111ee9692";
const ORIGIN: &str = "http://127.0.0.1:7331";
const HOST: &str = "127.0.0.1:7331";

fn application() -> Router {
    app(
        Config::parse(HOST, ORIGIN, ".".into(), false).unwrap(),
        SECRET.into(),
    )
}

fn request(path: &str) -> axum::http::request::Builder {
    Request::builder().uri(path).header(header::HOST, HOST)
}

fn authenticated(path: &str) -> Request<Body> {
    request(path)
        .header(header::COOKIE, format!("cube_play_session_7331={SECRET}"))
        .body(Body::empty())
        .unwrap()
}

#[tokio::test]
async fn readiness_and_authenticated_health_report_real_version() {
    for req in [
        request("/healthz").body(Body::empty()).unwrap(),
        authenticated("/api/health"),
    ] {
        let response = application().oneshot(req).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        assert_eq!(response.headers()["x-content-type-options"], "nosniff");
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&bytes).unwrap(),
            serde_json::json!({"status":"ok","version":env!("CARGO_PKG_VERSION")})
        );
        assert!(!String::from_utf8_lossy(&bytes).contains(SECRET));
    }
}

#[tokio::test]
async fn exchanges_fragment_capability_for_httponly_same_site_cookie() {
    let application = application();
    let response = application
        .clone()
        .oneshot(
            request("/api/session")
                .method("POST")
                .header(header::ORIGIN, ORIGIN)
                .header(header::AUTHORIZATION, format!("Bearer {SECRET}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    let cookie = response.headers()[header::SET_COOKIE].to_str().unwrap();
    assert!(cookie.contains("HttpOnly; SameSite=Strict; Path=/api"));
    let response = application
        .oneshot(
            request("/api/health")
                .header(header::COOKIE, cookie.split(';').next().unwrap())
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn rejects_missing_invalid_and_ambiguous_sessions() {
    for cookie in [
        None,
        Some("cube_play_session_7331=wrong".into()),
        Some(format!(
            "cube_play_session_7331={SECRET}; cube_play_session_7331={SECRET}"
        )),
        Some("other=value".into()),
    ] {
        let mut req = request("/api/health");
        if let Some(cookie) = cookie {
            req = req.header(header::COOKIE, cookie);
        }
        let response = application()
            .oneshot(req.body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
    for authorization in [None, Some("Bearer wrong"), Some(SECRET)] {
        let mut req = request("/api/session")
            .method("POST")
            .header(header::ORIGIN, ORIGIN);
        if let Some(value) = authorization {
            req = req.header(header::AUTHORIZATION, value);
        }
        assert_eq!(
            application()
                .oneshot(req.body(Body::empty()).unwrap())
                .await
                .unwrap()
                .status(),
            StatusCode::UNAUTHORIZED
        );
    }
}

#[tokio::test]
async fn rejects_dns_rebinding_and_cross_origin_requests_on_every_surface() {
    for path in [
        "/",
        "/healthz",
        "/api/health",
        "/api/session",
        "/api/lab/library",
        "/api/lab/history",
        "/api/challenges",
        "/api/challenges/import",
        "/api/challenges/stage",
        "/api/challenges/candidates",
    ] {
        for (name, value) in [
            ("host", "attacker.example:7331"),
            ("origin", "http://attacker.example"),
            ("origin", "null"),
            ("sec-fetch-site", "cross-site"),
            ("x-forwarded-host", HOST),
            ("forwarded", "host=127.0.0.1:7331"),
        ] {
            let mut req = Request::builder().uri(path).header(name, value);
            if name != "host" {
                req = req.header(header::HOST, HOST);
            }
            let response = application()
                .oneshot(req.body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::FORBIDDEN, "{path}: {name}");
            assert!(
                response.headers()["content-security-policy"]
                    .to_str()
                    .unwrap()
                    .contains("frame-ancestors 'none'")
            );
        }
    }
    for req in [
        Request::builder().uri("/healthz"),
        request("/healthz").header(header::HOST, HOST),
        request("/healthz")
            .header(header::ORIGIN, ORIGIN)
            .header(header::ORIGIN, ORIGIN),
    ] {
        assert_eq!(
            application()
                .oneshot(req.body(Body::empty()).unwrap())
                .await
                .unwrap()
                .status(),
            StatusCode::FORBIDDEN
        );
    }
}

#[tokio::test]
async fn mutations_require_same_origin_and_bounded_input() {
    let response = application()
        .oneshot(
            request("/api/session")
                .method("POST")
                .header(header::AUTHORIZATION, format!("Bearer {SECRET}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let response = application()
        .oneshot(
            request("/api/session")
                .method("POST")
                .header(header::ORIGIN, ORIGIN)
                .header(header::AUTHORIZATION, format!("Bearer {SECRET}"))
                .header(header::CONTENT_LENGTH, "1025")
                .body(Body::from(vec![0u8; 1025]))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn serves_frontend_assets_without_exposing_parent_files() {
    let directory = tempfile::tempdir().unwrap();
    let frontend = directory.path().join("frontend");
    std::fs::create_dir(&frontend).unwrap();
    std::fs::write(
        frontend.join("index.html"),
        "<!doctype html><title>Cube Play</title>",
    )
    .unwrap();
    std::fs::write(directory.path().join("private.txt"), "private fixture").unwrap();
    let application = app(
        Config::parse(HOST, ORIGIN, frontend, false).unwrap(),
        SECRET.into(),
    );
    let response = application
        .clone()
        .oneshot(request("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response.headers()[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("text/html")
    );
    assert!(
        response.headers()["content-security-policy"]
            .to_str()
            .unwrap()
            .contains("script-src 'self'")
    );
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    assert!(String::from_utf8_lossy(&bytes).contains("Cube Play"));
    for path in [
        "/../private.txt",
        "/%2e%2e/private.txt",
        "/unknown",
        "/api/unknown",
    ] {
        let response = application
            .clone()
            .oneshot(request(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND, "{path}");
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        assert!(!String::from_utf8_lossy(&bytes).contains("private fixture"));
    }
}

#[test]
fn fresh_sessions_use_distinct_random_capabilities() {
    let first = play_local::session_secret();
    let second = play_local::session_secret();
    assert_ne!(first, second);
    assert_eq!(uuid::Uuid::parse_str(&first).unwrap().get_version_num(), 4);
}

#[tokio::test]
async fn notebook_challenge_and_candidate_mutations_require_session_and_origin() {
    for (method, path) in [
        ("PUT", "/api/investigation/notes"),
        ("PUT", "/api/lab/library"),
        ("POST", "/api/challenges/import"),
        ("PUT", "/api/challenges/stage"),
        ("PUT", "/api/challenges/progress"),
        ("PUT", "/api/challenges/candidates"),
    ] {
        let unsigned = request(path)
            .method(method)
            .header(header::ORIGIN, ORIGIN)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from("{} "))
            .unwrap();
        assert_eq!(
            application().oneshot(unsigned).await.unwrap().status(),
            StatusCode::UNAUTHORIZED
        );
        let originless = request(path)
            .method(method)
            .header(header::COOKIE, format!("cube_play_session_7331={SECRET}"))
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from("{} "))
            .unwrap();
        assert_eq!(
            application().oneshot(originless).await.unwrap().status(),
            StatusCode::FORBIDDEN
        );
    }
}
