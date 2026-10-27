use axum::{
    Router,
    body::{Body, Bytes},
    http::{Request, StatusCode, header},
};
use futures_util::StreamExt;
use http_body_util::BodyExt;
use play_local::{
    app,
    config::{Config, WorkspaceContext},
    workspace::{MAX_ENTRIES, MAX_IMPORT, PAGE_SIZE},
};
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
};
use tower::ServiceExt;

const SECRET: &str = "b533dca5-8758-4273-a564-9c9fc31c4cb6";
struct Fixture {
    _directory: tempfile::TempDir,
    source: PathBuf,
    imports: PathBuf,
    app: Router,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        let source = root.join("source");
        fs::create_dir(&source).unwrap();
        let imports = root.join("imports");
        let mut config = Config::parse(
            "127.0.0.1:7331",
            "http://127.0.0.1:7331",
            "unused".into(),
            false,
        )
        .unwrap();
        config.source_dir = Some(source.clone());
        config.data_dir = Some(imports.clone());
        config.workspace = Some(WorkspaceContext {
            id: uuid::Uuid::new_v4(),
            name: "Local fixture".into(),
        });
        Self {
            _directory: directory,
            source,
            imports,
            app: app(config, SECRET.into()),
        }
    }
    async fn get(&self, path: &str) -> (StatusCode, Value) {
        let response = self
            .app
            .clone()
            .oneshot(request(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        let status = response.status();
        let body = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&body)
                .unwrap_or_else(|_| panic!("not JSON: {}", String::from_utf8_lossy(&body))),
        )
    }
    async fn upload(&self, name: &str, body: Body) -> axum::response::Response {
        self.app
            .clone()
            .oneshot(
                request(&format!("/api/workspace/import?name={name}"))
                    .method("POST")
                    .header(header::ORIGIN, "http://127.0.0.1:7331")
                    .body(body)
                    .unwrap(),
            )
            .await
            .unwrap()
    }
}
fn request(path: &str) -> axum::http::request::Builder {
    Request::builder()
        .uri(path)
        .header(header::HOST, "127.0.0.1:7331")
        .header(header::AUTHORIZATION, format!("Bearer {SECRET}"))
}
fn png() -> Vec<u8> {
    cube_core::image::Image::from_samples(
        1,
        1,
        cube_core::image::ColorModel::Rgb,
        vec![12, 34, 56],
        vec![],
        vec![],
    )
    .unwrap()
    .encode_png()
    .unwrap()
}
fn wave() -> Vec<u8> {
    let samples = [0u8; 8];
    let mut bytes = b"RIFF".to_vec();
    bytes.extend((36u32 + samples.len() as u32).to_le_bytes());
    bytes.extend(b"WAVEfmt ");
    bytes.extend(16u32.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(1u16.to_le_bytes());
    bytes.extend(8000u32.to_le_bytes());
    bytes.extend(16000u32.to_le_bytes());
    bytes.extend(2u16.to_le_bytes());
    bytes.extend(16u16.to_le_bytes());
    bytes.extend(b"data");
    bytes.extend((samples.len() as u32).to_le_bytes());
    bytes.extend(samples);
    bytes
}

#[tokio::test]
async fn empty_shallow_navigation_categories_and_direct_files_preserve_sources() {
    let fixture = Fixture::new();
    let (status, empty) = fixture.get("/api/workspace/files").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(empty["entries"], json!([]));
    assert_eq!(empty["imports_available"], true);
    fs::create_dir(fixture.source.join("nested")).unwrap();
    fs::write(fixture.source.join("nested/deep.txt"), b"not scanned").unwrap();
    for (name, bytes) in [
        ("sample.PNG", png()),
        ("clip.mp4", b"unsupported video fixture".to_vec()),
        ("notes.txt", b"private UTF-8 text".to_vec()),
        ("audio.wav", wave()),
        ("bytes.bin", vec![0, 255, 3]),
        ("unknown.xyz", vec![255, 1]),
        (".cube-play-import-original", vec![1]),
    ] {
        fs::write(fixture.source.join(name), bytes).unwrap();
    }
    let (_, listing) = fixture.get("/api/workspace/files").await;
    let entries = listing["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 8);
    assert!(!listing.to_string().contains("deep.txt"));
    for (name, category) in [
        ("sample.PNG", "image"),
        ("clip.mp4", "video"),
        ("audio.wav", "audio"),
        ("notes.txt", "text"),
        ("bytes.bin", "binary"),
        ("unknown.xyz", "unknown"),
    ] {
        let entry = entries.iter().find(|e| e["name"] == name).unwrap();
        assert_eq!(entry["category"], category);
        assert_eq!(entry["accessible"], true);
    }
    assert_eq!(
        fixture.get("/api/workspace/files?path=nested").await.1["entries"][0]["name"],
        "deep.txt"
    );
    let before = fs::read(fixture.source.join("notes.txt")).unwrap();
    let (_, inspection) = fixture.get("/api/workspace/inspect?path=notes.txt").await;
    assert_eq!(inspection["text"]["value"], "private UTF-8 text");
    assert_eq!(inspection["preview"]["kind"], "text");
    assert_eq!(fs::read(fixture.source.join("notes.txt")).unwrap(), before);
    assert!(
        !listing
            .to_string()
            .contains(fixture.source.to_str().unwrap())
    );
}

#[tokio::test]
async fn selected_previews_use_core_facts_and_keep_unsupported_formats_inspectable() {
    let fixture = Fixture::new();
    let image = png();
    fs::write(fixture.source.join("misleading.txt"), &image).unwrap();
    let (_, info) = fixture
        .get("/api/workspace/inspect?path=misleading.txt")
        .await;
    assert_eq!(info["signatures"][0]["format"], "png");
    assert_eq!(info["extension_mismatch"], true);
    assert_eq!(info["entry"]["category"], "text");
    assert_eq!(info["image"]["width"], 1);
    assert_eq!(info["preview"]["mime"], "image/png");
    let response = fixture
        .app
        .clone()
        .oneshot(
            request("/api/workspace/media?path=misleading.txt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()[header::CONTENT_TYPE], "image/png");
    assert_eq!(response.headers()["x-content-type-options"], "nosniff");
    assert_eq!(
        response.into_body().collect().await.unwrap().to_bytes(),
        image
    );
    fs::write(fixture.source.join("invalid.png"), b"not a PNG").unwrap();
    let (_, info) = fixture.get("/api/workspace/inspect?path=invalid.png").await;
    assert_eq!(info["preview"]["available"], false);
    assert!(info["notice"].as_str().unwrap().contains("image"));
    fs::write(
        fixture.source.join("clip.mp4"),
        b"unsupported codec/container",
    )
    .unwrap();
    let (_, info) = fixture.get("/api/workspace/inspect?path=clip.mp4").await;
    assert_eq!(info["preview"]["available"], false);
    assert!(!info["rows"].as_array().unwrap().is_empty());
    fs::write(
        fixture.source.join("unsafe.html"),
        b"<script>fetch('https://attacker.invalid')</script>",
    )
    .unwrap();
    let (_, info) = fixture.get("/api/workspace/inspect?path=unsafe.html").await;
    assert!(
        info["text"]["value"]
            .as_str()
            .unwrap()
            .starts_with("<script>")
    );
    assert_eq!(
        fixture.get("/api/workspace/media?path=unsafe.html").await.0,
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    );
    fs::write(fixture.source.join("invalid.txt"), [0, 255, 1]).unwrap();
    let (_, info) = fixture.get("/api/workspace/inspect?path=invalid.txt").await;
    assert!(info["text"].is_null());
    assert_eq!(info["rows"][0]["hex"], "00ff01");
    fs::write(fixture.source.join("empty.txt"), []).unwrap();
    assert_eq!(
        fixture.get("/api/workspace/inspect?path=empty.txt").await.1["text"]["value"],
        ""
    );
}

#[tokio::test]
async fn media_ranges_and_size_budgets_are_enforced_without_loading_entire_files() {
    let fixture = Fixture::new();
    let audio = wave();
    fs::write(fixture.source.join("audio.wav"), &audio).unwrap();
    for (value, start, end, status) in [
        (Some("bytes=2-7"), 2, 7, StatusCode::PARTIAL_CONTENT),
        (
            Some("bytes=-4"),
            audio.len() - 4,
            audio.len() - 1,
            StatusCode::PARTIAL_CONTENT,
        ),
        (None, 0, audio.len() - 1, StatusCode::OK),
    ] {
        let mut req = request("/api/workspace/media?path=audio.wav");
        if let Some(value) = value {
            req = req.header(header::RANGE, value)
        }
        let response = fixture
            .app
            .clone()
            .oneshot(req.body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        assert_eq!(
            response.headers()[header::CONTENT_LENGTH],
            (end - start + 1).to_string()
        );
        assert_eq!(
            response.into_body().collect().await.unwrap().to_bytes(),
            audio[start..=end]
        );
    }
    let response = fixture
        .app
        .clone()
        .oneshot(
            request("/api/workspace/media?path=audio.wav")
                .header(header::RANGE, "bytes=9999-")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::RANGE_NOT_SATISFIABLE);
    assert_eq!(
        response.headers()[header::CONTENT_RANGE],
        format!("bytes */{}", audio.len())
    );
    let file = fs::File::create(fixture.source.join("large.mp3")).unwrap();
    file.set_len(play_local::workspace::MAX_MEDIA + 1).unwrap();
    assert_eq!(
        fixture.get("/api/workspace/media?path=large.mp3").await.0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    let (_, info) = fixture.get("/api/workspace/inspect?path=large.mp3").await;
    assert_eq!(info["preview"]["available"], false);
    assert_eq!(info["next_offset"], 4096);
    let (_, next) = fixture
        .get("/api/workspace/inspect?path=large.mp3&offset=4096")
        .await;
    assert_eq!(next["rows"][0]["offset"], 4096);
    assert_eq!(
        fixture
            .get("/api/workspace/inspect?path=audio.wav&offset=999999")
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    let mut text = vec![b'a'; 65535];
    text.extend("空".as_bytes());
    fs::write(fixture.source.join("boundary.txt"), &text).unwrap();
    let (_, info) = fixture
        .get("/api/workspace/inspect?path=boundary.txt")
        .await;
    assert_eq!(info["text"]["bytes"], 65535);
    assert_eq!(info["text"]["truncated"], true);
}

#[tokio::test]
async fn directory_limits_and_missing_files_are_explicit() {
    let fixture = Fixture::new();
    for index in 0..MAX_ENTRIES + 1 {
        fs::write(fixture.source.join(format!("file-{index:04}.txt")), []).unwrap();
    }
    let (_, listing) = fixture.get("/api/workspace/files").await;
    assert_eq!(listing["entries"].as_array().unwrap().len(), PAGE_SIZE);
    assert_eq!(listing["truncated"], true);
    assert_eq!(listing["next_offset"], PAGE_SIZE);
    let (_, page) = fixture.get("/api/workspace/files?offset=128").await;
    assert_ne!(listing["entries"][0]["name"], page["entries"][0]["name"]);
    assert_eq!(
        fixture.get("/api/workspace/files?offset=4096").await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        fixture.get("/api/workspace/files?offset=1").await.0,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        fixture
            .get("/api/workspace/inspect?path=missing.txt")
            .await
            .0,
        StatusCode::NOT_FOUND
    );
    fs::write(fixture.source.join("gone.txt"), b"temporary").unwrap();
    fixture.get("/api/workspace/files").await;
    fs::remove_file(fixture.source.join("gone.txt")).unwrap();
    assert_eq!(
        fixture.get("/api/workspace/media?path=gone.txt").await.0,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn traversal_devices_unknown_parameters_and_sessions_are_rejected() {
    let fixture = Fixture::new();
    for path in [
        "..",
        "%2e%2e%2fsecret",
        "%2fetc%2fpasswd",
        "a%2f..%2fsecret",
        "a%5cb",
        "C%3a%5csecret",
        "a%2f",
        "a%2f%2fb",
        "CON",
        "COM%C2%B9.txt",
        "NUL.txt",
        "a.txt%3asecret",
    ] {
        assert_eq!(
            fixture
                .get(&format!("/api/workspace/inspect?path={path}"))
                .await
                .0,
            StatusCode::BAD_REQUEST,
            "{path}"
        );
    }
    for suffix in ["?store=other", "?offset=-1", "?unknown=1", "?path=x&path=y"] {
        assert_eq!(
            fixture
                .get(&format!("/api/workspace/files{suffix}"))
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
    }
    let response = fixture
        .app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/workspace/files")
                .header(header::HOST, "127.0.0.1:7331")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let response = fixture
        .app
        .clone()
        .oneshot(
            request("/api/workspace/import?name=file.txt")
                .method("POST")
                .body(Body::from("no origin"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let request_path = format!("/api/workspace/files?path={}", "a".repeat(16384));
    assert_eq!(fixture.get(&request_path).await.0, StatusCode::URI_TOO_LONG);
}

#[tokio::test]
async fn imports_are_explicit_private_atomic_and_never_overwrite_sources_or_existing_files() {
    let fixture = Fixture::new();
    fs::write(fixture.source.join("notes.txt"), b"source original").unwrap();
    assert_eq!(
        fixture
            .upload("notes.txt", Body::from("imported copy"))
            .await
            .status(),
        StatusCode::CREATED
    );
    assert_eq!(
        fs::read(fixture.imports.join("notes.txt")).unwrap(),
        b"imported copy"
    );
    assert_eq!(
        fs::read(fixture.source.join("notes.txt")).unwrap(),
        b"source original"
    );
    assert_eq!(
        fixture
            .upload("notes.txt", Body::from("must not replace"))
            .await
            .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        fs::read(fixture.imports.join("notes.txt")).unwrap(),
        b"imported copy"
    );
    assert_eq!(
        fixture
            .upload("..%2foutside", Body::from("denied"))
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        fixture
            .upload(".cube-play-import-private", Body::empty())
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    let (_, listing) = fixture.get("/api/workspace/files?store=imports").await;
    assert_eq!(listing["entries"].as_array().unwrap().len(), 1);
    assert_eq!(
        fixture
            .get("/api/workspace/inspect?store=imports&path=notes.txt")
            .await
            .1["text"]["value"],
        "imported copy"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            fs::metadata(&fixture.imports).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            fs::metadata(fixture.imports.join("notes.txt"))
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}

#[tokio::test]
async fn import_size_limits_and_interrupted_body_remove_partial_files() {
    let fixture = Fixture::new();
    let chunk = Bytes::from(vec![0; 1024 * 1024]);
    let first_chunk = chunk.clone();
    let chunks = (0..(MAX_IMPORT / (1024 * 1024)))
        .map(move |_| Ok::<_, std::io::Error>(first_chunk.clone()));
    assert_eq!(
        fixture
            .upload(
                "exact.bin",
                Body::from_stream(futures_util::stream::iter(chunks))
            )
            .await
            .status(),
        StatusCode::CREATED
    );
    assert_eq!(
        fs::metadata(fixture.imports.join("exact.bin"))
            .unwrap()
            .len(),
        MAX_IMPORT
    );
    let chunks = (0..(MAX_IMPORT / (1024 * 1024)))
        .map(move |_| Ok::<_, std::io::Error>(chunk.clone()))
        .chain([Ok(Bytes::from_static(b"x"))]);
    assert!(
        !fixture
            .upload(
                "over.bin",
                Body::from_stream(futures_util::stream::iter(chunks))
            )
            .await
            .status()
            .is_success()
    );
    assert!(!fixture.imports.join("over.bin").exists());
    let chunks = [
        Ok(Bytes::from_static(b"partial")),
        Err(std::io::Error::other("fixture disconnect")),
    ];
    assert_eq!(
        fixture
            .upload(
                "partial.bin",
                Body::from_stream(futures_util::stream::iter(chunks))
            )
            .await
            .status(),
        StatusCode::BAD_REQUEST
    );
    assert_eq!(fs::read_dir(&fixture.imports).unwrap().count(), 1);
}

#[cfg(unix)]
#[tokio::test]
async fn symlinks_special_files_and_replacement_races_never_expose_external_bytes() {
    use std::{
        os::unix::{fs::symlink, net::UnixListener},
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
    };
    let fixture = Fixture::new();
    let outside = fixture.source.parent().unwrap().join("outside");
    fs::create_dir(&outside).unwrap();
    fs::write(outside.join("secret.txt"), b"EXTERNAL_SECRET_SENTINEL").unwrap();
    symlink(&outside, fixture.source.join("escape")).unwrap();
    symlink("escape/", fixture.source.join("indirect")).unwrap();
    for path in ["escape/secret.txt", "indirect/secret.txt"] {
        assert_eq!(
            fixture
                .get(&format!("/api/workspace/inspect?path={path}"))
                .await
                .0,
            StatusCode::FORBIDDEN
        );
    }
    let _socket = UnixListener::bind(fixture.source.join("socket")).unwrap();
    assert_eq!(
        fixture.get("/api/workspace/inspect?path=socket").await.0,
        StatusCode::BAD_REQUEST
    );
    assert!(
        std::process::Command::new("mkfifo")
            .arg(fixture.source.join("fifo"))
            .status()
            .unwrap()
            .success()
    );
    assert_eq!(
        fixture.get("/api/workspace/inspect?path=fifo").await.0,
        StatusCode::BAD_REQUEST
    );
    let path = fixture.source.join("changing");
    fs::write(&path, b"authorized bytes").unwrap();
    let temporary = fixture.source.join("replacement");
    let running = Arc::new(AtomicBool::new(true));
    let active = running.clone();
    let racer = std::thread::spawn(move || {
        while active.load(Ordering::Relaxed) {
            let _ = fs::remove_file(&temporary);
            symlink(outside.join("secret.txt"), &temporary).unwrap();
            let _ = fs::rename(&temporary, &path);
            let _ = fs::remove_file(&temporary);
            fs::write(&temporary, b"authorized bytes").unwrap();
            let _ = fs::rename(&temporary, &path);
        }
    });
    for _ in 0..100 {
        let (_, body) = fixture.get("/api/workspace/inspect?path=changing").await;
        assert!(!body.to_string().contains("EXTERNAL_SECRET_SENTINEL"));
    }
    running.store(false, Ordering::Relaxed);
    racer.join().unwrap();
    let (_, listing) = fixture.get("/api/workspace/files").await;
    let entries = listing["entries"].as_array().unwrap();
    assert_eq!(
        entries.iter().find(|e| e["name"] == "escape").unwrap()["accessible"],
        false
    );
}

#[test]
fn configured_storage_cannot_alias_or_create_directories_in_the_source() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().canonicalize().unwrap();
    let mut config = Config::parse(
        "127.0.0.1:7331",
        "http://127.0.0.1:7331",
        Path::new("unused").into(),
        false,
    )
    .unwrap();
    config.source_dir = Some(root.clone());
    config.data_dir = Some(root.join("must-not-create"));
    assert!(play_local::workspace::Workspace::open(&config).is_err());
    assert!(!root.join("must-not-create").exists());
}

#[tokio::test]
async fn canceled_uploads_remove_temporary_files_and_release_their_slot() {
    let fixture = Fixture::new();
    let (body_started, body_received) = tokio::sync::oneshot::channel();
    let body = Body::from_stream(
        futures_util::stream::once(async move {
            let _ = body_started.send(());
            Ok::<_, std::io::Error>(Bytes::from_static(b"partial"))
        })
        .chain(futures_util::stream::pending()),
    );
    let application = fixture.app.clone();
    let task = tokio::spawn(async move {
        application
            .oneshot(
                request("/api/workspace/import?name=canceled.bin")
                    .method("POST")
                    .header(header::ORIGIN, "http://127.0.0.1:7331")
                    .body(body)
                    .unwrap(),
            )
            .await
    });
    // File creation runs on a blocking worker. Observe body consumption so the
    // request owns its cleanup guard before asserting synchronous cancellation.
    tokio::time::timeout(std::time::Duration::from_secs(3), body_received)
        .await
        .unwrap()
        .unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    assert_eq!(fs::read_dir(&fixture.imports).unwrap().count(), 0);
    assert_eq!(fs::read_dir(&fixture.source).unwrap().count(), 0);
    assert_eq!(
        fixture
            .upload("after-cancel.txt", Body::from("complete"))
            .await
            .status(),
        StatusCode::CREATED
    );
}

#[tokio::test]
async fn live_media_responses_bound_concurrency_until_dropped() {
    let fixture = Fixture::new();
    fs::write(fixture.source.join("audio.wav"), wave()).unwrap();
    let first = fixture
        .app
        .clone()
        .oneshot(
            request("/api/workspace/media?path=audio.wav")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let second = fixture
        .app
        .clone()
        .oneshot(
            request("/api/workspace/media?path=audio.wav")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(second.status(), StatusCode::OK);
    assert_eq!(
        fixture.get("/api/workspace/files").await.0,
        StatusCode::TOO_MANY_REQUESTS
    );
    drop(first);
    drop(second);
    assert_eq!(fixture.get("/api/workspace/files").await.0, StatusCode::OK);
}

#[tokio::test]
async fn image_budget_boundaries_and_unconfigured_roots_fail_clearly() {
    let fixture = Fixture::new();
    let file = fs::File::create(fixture.source.join("bounded.png")).unwrap();
    file.set_len(play_local::workspace::MAX_IMAGE).unwrap();
    assert_eq!(
        fixture.get("/api/workspace/media?path=bounded.png").await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    file.set_len(play_local::workspace::MAX_IMAGE + 1).unwrap();
    assert_eq!(
        fixture.get("/api/workspace/media?path=bounded.png").await.0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    let config = Config::parse(
        "127.0.0.1:7331",
        "http://127.0.0.1:7331",
        "unused".into(),
        false,
    )
    .unwrap();
    let application = app(config.clone(), SECRET.into());
    let response = application
        .oneshot(request("/api/workspace/files").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let mut config = config;
    config.source_dir = Some(fixture.source.clone());
    let application = app(config, SECRET.into());
    let response = application
        .oneshot(
            request("/api/workspace/import?name=denied.txt")
                .method("POST")
                .header(header::ORIGIN, "http://127.0.0.1:7331")
                .body(Body::from("must not reach source"))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert!(!fixture.source.join("denied.txt").exists());
}

#[tokio::test]
async fn challenge_import_waits_for_preview_capacity_instead_of_claiming_a_size_error() {
    let fixture = Fixture::new();
    assert_eq!(fixture.get("/api/investigation").await.0, StatusCode::OK);
    fs::write(fixture.source.join("audio.wav"), wave()).unwrap();
    fs::write(fixture.source.join("wait.cube"),json!({"kind":"cube-play-challenge","format_version":1,"id":"wait","title":"Wait for previews","stages":[{"id":"one","title":"Inspect","instructions":"Read local evidence."}]}).to_string()).unwrap();
    let first = fixture
        .app
        .clone()
        .oneshot(
            request("/api/workspace/media?path=audio.wav")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let second = fixture
        .app
        .clone()
        .oneshot(
            request("/api/workspace/media?path=audio.wav")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(first.status(), StatusCode::OK);
    assert_eq!(second.status(), StatusCode::OK);
    let app = fixture.app.clone();
    let mut pending = tokio::spawn(async move {
        app.oneshot(
            request("/api/challenges/import")
                .method("POST")
                .header(header::ORIGIN, "http://127.0.0.1:7331")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"store":"source","path":"wait.cube"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap()
    });
    assert!(
        tokio::time::timeout(std::time::Duration::from_millis(100), &mut pending)
            .await
            .is_err()
    );
    drop(first);
    assert_eq!(
        tokio::time::timeout(std::time::Duration::from_secs(3), pending)
            .await
            .unwrap()
            .unwrap()
            .status(),
        StatusCode::OK
    );
    drop(second);
}
