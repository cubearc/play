use axum::{
    Router,
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use play_local::{
    PersistenceHandle, application,
    config::{Config, WorkspaceContext},
};
use serde_json::{Value, json};
use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
use std::{fs, path::PathBuf};
use tower::ServiceExt;
use uuid::Uuid;

struct Fixture {
    _directory: tempfile::TempDir,
    config: Config,
    running: Option<(Router, PersistenceHandle)>,
}
impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().canonicalize().unwrap();
        let source = root.join("source");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("original.txt"), b"original private source").unwrap();
        let mut config = Config::parse(
            "127.0.0.1:7331",
            "http://127.0.0.1:7331",
            "unused".into(),
            false,
        )
        .unwrap();
        config.source_dir = Some(source);
        config.data_dir = Some(root.join("imports"));
        config.workspace = Some(WorkspaceContext {
            id: Uuid::new_v4(),
            name: "Private investigation 空".into(),
        });
        let running = Some(application(config.clone(), "private-capability".into()));
        Self {
            _directory: directory,
            config,
            running,
        }
    }
    fn path(&self) -> PathBuf {
        self.config
            .data_dir
            .as_ref()
            .unwrap()
            .parent()
            .unwrap()
            .join("state/investigation.sqlite3")
    }
    fn start(&mut self) {
        self.running = Some(application(
            self.config.clone(),
            "private-capability".into(),
        ));
    }
    async fn stop(&mut self) {
        if let Some((app, handle)) = self.running.take() {
            handle.close().await.unwrap();
            drop(app);
            drop(handle);
        }
    }
    async fn sql(&self) -> SqliteConnection {
        SqliteConnection::connect_with(&SqliteConnectOptions::new().filename(self.path()))
            .await
            .unwrap()
    }
    async fn request(&self, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
        let builder = Request::builder()
            .uri(path)
            .method(method)
            .header("Host", "127.0.0.1:7331")
            .header("Origin", "http://127.0.0.1:7331")
            .header("Authorization", "Bearer private-capability")
            .header("Content-Type", "application/json");
        let response = self
            .running
            .as_ref()
            .unwrap()
            .0
            .clone()
            .oneshot(
                builder
                    .body(if method == "GET" {
                        Body::empty()
                    } else {
                        Body::from(body.to_string())
                    })
                    .unwrap(),
            )
            .await
            .unwrap();
        let status = response.status();
        let bytes = response.into_body().collect().await.unwrap().to_bytes();
        (
            status,
            serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        )
    }
    async fn get(&self, path: &str) -> (StatusCode, Value) {
        self.request("GET", path, Value::Null).await
    }
    async fn put(&self, path: &str, value: Value) -> (StatusCode, Value) {
        self.request("PUT", path, value).await
    }
    async fn upload(&self, name: &str, bytes: &str) -> StatusCode {
        self.running
            .as_ref()
            .unwrap()
            .0
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/workspace/import?name={name}"))
                    .method("POST")
                    .header("Host", "127.0.0.1:7331")
                    .header("Origin", "http://127.0.0.1:7331")
                    .header("Authorization", "Bearer private-capability")
                    .body(Body::from(bytes.to_owned()))
                    .unwrap(),
            )
            .await
            .unwrap()
            .status()
    }
}

#[tokio::test]
async fn records_navigation_sessions_and_artifacts_survive_reopen_without_source_changes() {
    let mut fixture = Fixture::new();
    let (status, first) = fixture.get("/api/investigation").await;
    assert_eq!(status, StatusCode::OK, "{first}");
    assert_eq!(
        first["workspace_id"],
        fixture.config.workspace.as_ref().unwrap().id.to_string()
    );
    assert_eq!(first["view_revision"], 0);
    let view = json!({"store":"source","path":"","offset":0,"selected":{"store":"source","path":"original.txt"}});
    assert_eq!(
        fixture
            .put("/api/investigation/view", json!({"revision":0,"view":view}))
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        fixture
            .put(
                "/api/investigation",
                json!({"revision":1,"title":"Case '01' 空"})
            )
            .await
            .1["revision"],
        2
    );
    let note = Uuid::new_v4();
    let note_value = json!({"id":note,"title":"Hypothesis","body":"Private notes; SELECT 'literal';","revision":0});
    assert_eq!(
        fixture.put("/api/investigation/notes", note_value).await.1["revision"],
        1
    );
    let progress = json!({"challenge_key":"opaque-local-id","stage_key":"stage-1","local_status":"in_progress","revision":0});
    assert_eq!(
        fixture.put("/api/investigation/progress", progress).await.1["revision"],
        1
    );
    assert_eq!(
        fixture.upload("sample.txt", "imported private bytes").await,
        StatusCode::CREATED
    );
    let artifacts = fixture.get("/api/investigation/artifacts").await.1;
    assert_eq!(artifacts[0]["status"], "ready");
    assert_eq!(artifacts[0]["byte_size"], 22);
    let source = fixture.config.source_dir.as_ref().unwrap().clone();
    assert_eq!(fs::read_dir(&source).unwrap().count(), 1);
    assert_eq!(
        fs::read(source.join("original.txt")).unwrap(),
        b"original private source"
    );
    assert!(!first.to_string().contains(&source.display().to_string()));
    fixture.stop().await;
    let mut connection = fixture.sql().await;
    let status: String = sqlx::query_scalar("SELECT status FROM sessions WHERE id=?")
        .bind(first["session_id"].as_str().unwrap())
        .fetch_one(&mut connection)
        .await
        .unwrap();
    assert_eq!(status, "closed");
    let blobs: i64 =
        sqlx::query_scalar("SELECT count(*) FROM pragma_table_info('artifacts') WHERE type='BLOB'")
            .fetch_one(&mut connection)
            .await
            .unwrap();
    assert_eq!(blobs, 0);
    connection.close().await.unwrap();
    fixture.start();
    let (_, second) = fixture.get("/api/investigation").await;
    assert_eq!(second["id"], first["id"]);
    assert_eq!(second["title"], "Case '01' 空");
    assert_eq!(second["view"], view);
    assert_eq!(second["interrupted_sessions"], 0);
    let sessions = fixture.get("/api/investigation/sessions").await.1;
    assert_eq!(sessions.as_array().unwrap().len(), 2);
    assert!(
        sessions
            .as_array()
            .unwrap()
            .iter()
            .any(|session| session["status"] == "closed" && session["ended_ms"].is_number())
    );
    assert_ne!(second["session_id"], first["session_id"]);
    assert_eq!(
        fixture.get("/api/investigation/notes").await.1[0]["body"],
        "Private notes; SELECT 'literal';"
    );
    assert_eq!(
        fixture.get("/api/investigation/progress").await.1[0]["local_status"],
        "in_progress"
    );
    assert_eq!(
        fixture.get("/api/investigation/artifacts").await.1,
        artifacts
    );
    fixture.stop().await;
}

#[tokio::test]
async fn optimistic_revisions_and_validation_prevent_lost_updates_and_invalid_paths() {
    let mut fixture = Fixture::new();
    fixture.get("/api/investigation").await;
    let left = fixture.put(
        "/api/investigation",
        json!({"title":"Left tab","revision":1}),
    );
    let right = fixture.put(
        "/api/investigation",
        json!({"title":"Right tab","revision":1}),
    );
    let (left, right) = tokio::join!(left, right);
    assert!([left.0, right.0].contains(&StatusCode::CONFLICT));
    assert!([left.0, right.0].contains(&StatusCode::OK));
    let note = Uuid::new_v4();
    for (revision, expected) in [
        (0, StatusCode::OK),
        (0, StatusCode::CONFLICT),
        (1, StatusCode::OK),
        (1, StatusCode::CONFLICT),
    ] {
        assert_eq!(
            fixture
                .put(
                    "/api/investigation/notes",
                    json!({"id":note,"revision":revision,"title":"Note","body":"Bounded"})
                )
                .await
                .0,
            expected
        );
    }
    let progress = json!({"challenge_key":"local","stage_key":"first","local_status":"completed","revision":0});
    assert_eq!(
        fixture
            .put("/api/investigation/progress", progress.clone())
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        fixture.put("/api/investigation/progress", progress).await.0,
        StatusCode::CONFLICT
    );
    assert_eq!(fixture.put("/api/investigation/progress",json!({"challenge_key":"local","stage_key":"first","local_status":"unstarted","revision":1})).await.0,StatusCode::OK);
    let (_, before) = fixture.get("/api/investigation").await;
    for invalid in [
        json!({"title":"","revision":2}),
        json!({"title":"x".repeat(513),"revision":2}),
        json!({"title":"valid","revision":-1}),
        json!({"title":"valid","revision":i64::MAX}),
        json!({"title":"valid","revision":2,"unrecognized":"field"}),
    ] {
        assert_eq!(
            fixture.put("/api/investigation", invalid).await.0,
            StatusCode::BAD_REQUEST
        );
    }
    for invalid in [
        json!({"store":"source","path":"../escape","offset":0,"selected":null}),
        json!({"store":"source","path":"","offset":1,"selected":null}),
        json!({"store":"source","path":"","offset":4096,"selected":null}),
        json!({"store":"imports","path":"","offset":0,"selected":{"store":"source","path":"C:\\secret"}}),
    ] {
        assert_eq!(
            fixture
                .put(
                    "/api/investigation/view",
                    json!({"revision":0,"view":invalid})
                )
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
    }
    for invalid in [
        json!({"id":Uuid::nil(),"title":"Note","body":"","revision":0}),
        json!({"id":Uuid::new_v4(),"title":"Note","body":"x".repeat(65537),"revision":0}),
    ] {
        assert_eq!(
            fixture.put("/api/investigation/notes", invalid).await.0,
            StatusCode::BAD_REQUEST
        );
    }
    assert_eq!(
        fixture
            .put(
                "/api/investigation/progress",
                json!({"challenge_key":"","stage_key":"one","local_status":"verified","revision":0})
            )
            .await
            .0,
        StatusCode::BAD_REQUEST
    );
    let (_, after) = fixture.get("/api/investigation").await;
    assert_eq!(before, after);
    fixture.stop().await;
}

#[tokio::test]
async fn migrations_upgrade_idempotently_and_reject_changed_or_future_versions() {
    let mut fixture = Fixture::new();
    fixture.get("/api/investigation").await;
    fixture.stop().await;
    let mut connection = fixture.sql().await;
    sqlx::query("DROP TABLE artifacts")
        .execute(&mut connection)
        .await
        .unwrap();
    sqlx::query("DELETE FROM _sqlx_migrations WHERE version=2")
        .execute(&mut connection)
        .await
        .unwrap();
    connection.close().await.unwrap();
    fixture.start();
    assert_eq!(
        fixture.get("/api/investigation/artifacts").await,
        (StatusCode::OK, json!([]))
    );
    fixture.stop().await;
    fixture.start();
    assert_eq!(fixture.get("/healthz").await.0, StatusCode::OK);
    fixture.stop().await;
    let mut connection = fixture.sql().await;
    sqlx::query("UPDATE _sqlx_migrations SET checksum=X'00' WHERE version=1")
        .execute(&mut connection)
        .await
        .unwrap();
    connection.close().await.unwrap();
    fixture.start();
    let (status, error) = fixture.get("/healthz").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(error["code"], "migration_failed");
    fixture.stop().await;
    let mut connection = fixture.sql().await;
    sqlx::query("UPDATE _sqlx_migrations SET version=999 WHERE version=1")
        .execute(&mut connection)
        .await
        .unwrap();
    connection.close().await.unwrap();
    fixture.start();
    assert_eq!(
        fixture.get("/healthz").await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    fixture.stop().await;
}

#[tokio::test]
async fn recovery_preserves_missing_references_and_finishes_published_imports() {
    let mut fixture = Fixture::new();
    fixture.get("/api/investigation").await;
    fixture.upload("missing.txt", "retained bytes").await;
    let artifact = fixture.get("/api/investigation/artifacts").await.1[0].clone();
    fixture.stop().await;
    fs::remove_file(
        fixture
            .config
            .data_dir
            .as_ref()
            .unwrap()
            .join("missing.txt"),
    )
    .unwrap();
    let mut connection = fixture.sql().await;
    sqlx::query("UPDATE sessions SET status='running',ended_ms=NULL")
        .execute(&mut connection)
        .await
        .unwrap();
    sqlx::query("UPDATE investigation SET view_json='malformed'")
        .execute(&mut connection)
        .await
        .unwrap();
    let investigation: String = sqlx::query_scalar("SELECT id FROM investigation")
        .fetch_one(&mut connection)
        .await
        .unwrap();
    for (name, size) in [("published.txt", 9i64), ("unpublished.txt", 4)] {
        sqlx::query("INSERT INTO artifacts(id,investigation_id,path,byte_size,status,temporary,created_ms) VALUES(?,?,?,?,'pending','.cube-play-import-fixture',0)")
            .bind(Uuid::new_v4().to_string()).bind(&investigation).bind(name).bind(size).execute(&mut connection).await.unwrap();
    }
    connection.close().await.unwrap();
    fs::write(
        fixture
            .config
            .data_dir
            .as_ref()
            .unwrap()
            .join("published.txt"),
        b"published",
    )
    .unwrap();
    fs::write(
        fixture
            .config
            .data_dir
            .as_ref()
            .unwrap()
            .join(".cube-play-import-fixture"),
        b"partial",
    )
    .unwrap();
    fixture.start();
    let (status, snapshot) = fixture.get("/api/investigation").await;
    assert_eq!(status, StatusCode::OK, "{snapshot}");
    assert_eq!(snapshot["interrupted_sessions"], 1);
    assert_eq!(snapshot["warnings"].as_array().unwrap().len(), 2);
    assert_eq!(snapshot["view"]["path"], "");
    let records = fixture.get("/api/investigation/artifacts").await.1;
    let records = records.as_array().unwrap();
    assert_eq!(records.len(), 2);
    assert!(
        records
            .iter()
            .any(|record| record["path"] == "published.txt" && record["status"] == "ready")
    );
    assert!(
        records
            .iter()
            .any(|record| record["id"] == artifact["id"] && record["status"] == "missing")
    );
    assert_eq!(
        fixture
            .get("/api/workspace/inspect?store=imports&path=missing.txt")
            .await
            .0,
        StatusCode::CONFLICT
    );
    fixture.stop().await;
    let mut connection = fixture.sql().await;
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT view_json FROM investigation")
            .fetch_one(&mut connection)
            .await
            .unwrap(),
        "malformed"
    );
    connection.close().await.unwrap();
    fs::write(
        fixture
            .config
            .data_dir
            .as_ref()
            .unwrap()
            .join("missing.txt"),
        b"retained bytes",
    )
    .unwrap();
    fixture.start();
    assert_eq!(fixture.get("/api/investigation").await.0, StatusCode::OK);
    assert_eq!(
        fixture
            .get("/api/workspace/inspect?store=imports&path=missing.txt")
            .await
            .0,
        StatusCode::OK
    );
    fixture.stop().await;
}

#[tokio::test]
async fn foreign_corrupt_and_cross_workspace_databases_are_never_reset() {
    let mut fixture = Fixture::new();
    let (_, original) = fixture.get("/api/investigation").await;
    fixture.stop().await;
    fixture.config.workspace.as_mut().unwrap().id = Uuid::new_v4();
    fixture.start();
    assert_eq!(fixture.get("/healthz").await.0, StatusCode::CONFLICT);
    fixture.stop().await;
    let mut connection = fixture.sql().await;
    assert_eq!(
        sqlx::query_scalar::<_, String>("SELECT id FROM investigation")
            .fetch_one(&mut connection)
            .await
            .unwrap(),
        original["id"]
    );
    sqlx::query("PRAGMA application_id=0")
        .execute(&mut connection)
        .await
        .unwrap();
    connection.close().await.unwrap();
    fixture.start();
    assert_eq!(
        fixture.get("/healthz").await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    fixture.stop().await;
    fs::write(fixture.path(), b"invalid sqlite fixture, preserve me").unwrap();
    fixture.start();
    assert_eq!(
        fixture.get("/healthz").await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    fixture.stop().await;
    assert_eq!(
        fs::read(fixture.path()).unwrap(),
        b"invalid sqlite fixture, preserve me"
    );
}

#[tokio::test]
async fn a_second_owner_and_state_paths_within_authorized_files_are_rejected() {
    let mut fixture = Fixture::new();
    fixture.get("/api/investigation").await;
    let second = application(fixture.config.clone(), "private-capability".into());
    let request = Request::builder()
        .uri("/healthz")
        .header("Host", "127.0.0.1:7331")
        .body(Body::empty())
        .unwrap();
    let response = second.0.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    drop(second.1);
    fixture.stop().await;
    for path in [
        fixture.config.source_dir.as_ref().unwrap().join("state"),
        fixture.config.data_dir.as_ref().unwrap().join("state"),
    ] {
        fixture.config.state_dir = Some(path.clone());
        fixture.start();
        assert_eq!(
            fixture.get("/healthz").await.0,
            StatusCode::SERVICE_UNAVAILABLE
        );
        assert!(!path.exists());
        fixture.stop().await;
    }
}

#[cfg(unix)]
#[tokio::test]
async fn private_database_paths_reject_symlinks_hardlinks_and_unsafe_permissions() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let mut fixture = Fixture::new();
    fixture.get("/api/investigation").await;
    fixture.stop().await;
    let path = fixture.path();
    assert_eq!(
        fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(path.parent().unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    let external = fixture
        .config
        .source_dir
        .as_ref()
        .unwrap()
        .join("original.txt");
    let database = fs::read(&path).unwrap();
    fs::remove_file(&path).unwrap();
    symlink(&external, &path).unwrap();
    fixture.start();
    assert_eq!(
        fixture.get("/healthz").await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    fixture.stop().await;
    assert_eq!(fs::read(&external).unwrap(), b"original private source");
    fs::remove_file(&path).unwrap();
    fs::write(&path, &database).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let alias = path.with_extension("alias");
    fs::hard_link(&path, &alias).unwrap();
    fixture.start();
    assert_eq!(
        fixture.get("/healthz").await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    fixture.stop().await;
    fs::remove_file(alias).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    fixture.start();
    assert_eq!(
        fixture.get("/healthz").await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    fixture.stop().await;
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    let sidecar = path.with_file_name("investigation.sqlite3-wal");
    symlink(&external, &sidecar).unwrap();
    fixture.start();
    assert_eq!(
        fixture.get("/healthz").await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    fixture.stop().await;
    fs::remove_file(sidecar).unwrap();
    let state = path.parent().unwrap().to_path_buf();
    let alias = state.with_file_name("alias-state");
    symlink(&state, &alias).unwrap();
    fixture.config.state_dir = Some(alias);
    fixture.start();
    assert_eq!(
        fixture.get("/healthz").await.0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    fixture.stop().await;
}

#[tokio::test]
async fn write_failures_roll_back_and_import_publication_can_recover_without_overwrites() {
    let mut fixture = Fixture::new();
    fixture.get("/api/investigation").await;
    let mut connection = fixture.sql().await;
    sqlx::query("CREATE TRIGGER reject_note BEFORE INSERT ON notes BEGIN SELECT RAISE(ABORT,'fixture full disk'); END").execute(&mut connection).await.unwrap();
    assert_eq!(
        fixture
            .put(
                "/api/investigation/notes",
                json!({"id":Uuid::new_v4(),"revision":0,"title":"Private","body":"sensitive"})
            )
            .await
            .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(fixture.get("/api/investigation/notes").await.1, json!([]));
    sqlx::query("DROP TRIGGER reject_note")
        .execute(&mut connection)
        .await
        .unwrap();
    sqlx::query("CREATE TRIGGER reject_finalize BEFORE UPDATE ON artifacts WHEN NEW.status='ready' BEGIN SELECT RAISE(ABORT,'fixture full disk'); END").execute(&mut connection).await.unwrap();
    assert_eq!(
        fixture.upload("recovered.txt", "complete bytes").await,
        StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        fixture
            .get("/api/workspace/inspect?store=imports&path=recovered.txt")
            .await
            .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        fixture.get("/api/workspace/files?store=imports").await.1["entries"][0]["accessible"],
        false
    );
    assert_eq!(
        fs::read(
            fixture
                .config
                .data_dir
                .as_ref()
                .unwrap()
                .join("recovered.txt")
        )
        .unwrap(),
        b"complete bytes"
    );
    sqlx::query("DROP TRIGGER reject_finalize")
        .execute(&mut connection)
        .await
        .unwrap();
    connection.close().await.unwrap();
    fixture.stop().await;
    fixture.start();
    assert_eq!(
        fixture
            .get("/api/workspace/inspect?store=imports&path=recovered.txt")
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        fixture.upload("recovered.txt", "never overwrite").await,
        StatusCode::CONFLICT
    );
    assert_eq!(
        fs::read(
            fixture
                .config
                .data_dir
                .as_ref()
                .unwrap()
                .join("recovered.txt")
        )
        .unwrap(),
        b"complete bytes"
    );
    fixture.stop().await;
}

#[tokio::test]
async fn state_routes_require_session_origin_and_bounded_bodies() {
    let mut fixture = Fixture::new();
    for endpoint in [
        "/api/investigation",
        "/api/investigation/view",
        "/api/investigation/notes",
        "/api/investigation/progress",
        "/api/investigation/artifacts",
        "/api/investigation/sessions",
    ] {
        let response = fixture
            .running
            .as_ref()
            .unwrap()
            .0
            .clone()
            .oneshot(
                Request::builder()
                    .uri(endpoint)
                    .header("Host", "127.0.0.1:7331")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED, "{endpoint}");
        let response = fixture
            .running
            .as_ref()
            .unwrap()
            .0
            .clone()
            .oneshot(
                Request::builder()
                    .uri(endpoint)
                    .method("PUT")
                    .header("Host", "127.0.0.1:7331")
                    .header("Authorization", "Bearer private-capability")
                    .header("Origin", "http://untrusted.example")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{endpoint}");
    }
    assert!(
        !fixture.path().exists(),
        "unauthorized requests must not initialize private state"
    );
    assert_eq!(fixture.put("/api/investigation/notes",json!({"id":Uuid::new_v4(),"title":"Bounded","body":"x".repeat(600*1024),"revision":0})).await.0,StatusCode::BAD_REQUEST);
    assert_eq!(fixture.get("/api/investigation/notes").await.1, json!([]));
    fixture.stop().await;
}

#[tokio::test]
async fn record_budgets_reject_new_entries_without_changing_existing_rows() {
    let mut fixture = Fixture::new();
    fixture.get("/api/investigation").await;
    let mut connection = fixture.sql().await;
    sqlx::query("WITH RECURSIVE n(i) AS (VALUES(1) UNION ALL SELECT i+1 FROM n WHERE i<100) INSERT INTO notes(id,investigation_id,title,body,revision,updated_ms) SELECT printf('00000000-0000-4000-8000-%012d',i),(SELECT id FROM investigation),'Note','retained',1,0 FROM n").execute(&mut connection).await.unwrap();
    sqlx::query("WITH RECURSIVE n(i) AS (VALUES(1) UNION ALL SELECT i+1 FROM n WHERE i<1000) INSERT INTO progress(challenge_key,stage_key,investigation_id,local_status,revision,updated_ms) SELECT 'local',CAST(i AS TEXT),(SELECT id FROM investigation),'unstarted',1,0 FROM n").execute(&mut connection).await.unwrap();
    sqlx::query("WITH RECURSIVE n(i) AS (VALUES(1) UNION ALL SELECT i+1 FROM n WHERE i<4096) INSERT INTO artifacts(id,investigation_id,path,byte_size,status,created_ms) SELECT CAST(i AS TEXT),(SELECT id FROM investigation),CAST(i AS TEXT),0,'missing',0 FROM n").execute(&mut connection).await.unwrap();
    connection.close().await.unwrap();
    assert_eq!(
        fixture
            .put(
                "/api/investigation/notes",
                json!({"id":Uuid::new_v4(),"revision":0,"title":"New","body":""})
            )
            .await
            .0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert_eq!(fixture.put("/api/investigation/progress",json!({"challenge_key":"local","stage_key":"new","revision":0,"local_status":"unstarted"})).await.0,StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(
        fixture.upload("new.txt", "bounded").await,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert!(
        !fixture
            .config
            .data_dir
            .as_ref()
            .unwrap()
            .join("new.txt")
            .exists()
    );
    assert_eq!(
        fixture
            .get("/api/investigation/notes")
            .await
            .1
            .as_array()
            .unwrap()
            .len(),
        100
    );
    assert_eq!(
        fixture
            .get("/api/investigation/progress")
            .await
            .1
            .as_array()
            .unwrap()
            .len(),
        1000
    );
    fixture.stop().await;
}

#[tokio::test]
async fn exploration_revisions_privacy_recovery_and_storage_limits_preserve_legacy_notes() {
    let mut fixture = Fixture::new();
    assert_eq!(fixture.get("/api/investigation").await.0, StatusCode::OK);
    let note = Uuid::new_v4();
    assert_eq!(fixture.put("/api/investigation/notes",json!({"id":note,"revision":0,"title":"Existing note","body":"Private original evidence"})).await.0,StatusCode::OK);
    let exploration = Uuid::new_v4();
    let step = Uuid::new_v4();
    let mut definition = json!({"schema_version":1,"id":exploration,"revision":0,"title":"Private key hypothesis","hypothesis":"","scope":null,"steps":[{"id":step,"operation":"cipher.vigenere.encode","operation_version":1,"inputs":{"input":{"type":"literal","value":"ABC"}},"parameters":{"key":"DO-NOT-RETAIN"},"private_parameters":["key"],"position":[0,0]}]});
    assert_eq!(
        fixture.put("/api/explorations", definition.clone()).await.0,
        StatusCode::BAD_REQUEST
    );
    definition["steps"][0]["parameters"] = json!({});
    let (status, saved) = fixture.put("/api/explorations", definition.clone()).await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    let run = Uuid::new_v4();
    let mut sql = fixture.sql().await;
    sqlx::query("INSERT INTO exploration_executions(id,exploration_id,configuration_revision,step_id,status,started_ms) VALUES(?,?,1,?,'running',1)").bind(run.to_string()).bind(exploration.to_string()).bind(step.to_string()).execute(&mut sql).await.unwrap();
    drop(sql);
    fixture.stop().await;
    fixture.start();
    let (_, notes) = fixture.get("/api/investigation/notes").await;
    assert_eq!(notes[0]["id"], note.to_string());
    assert_eq!(notes[0]["body"], "Private original evidence");
    let (_, runs) = fixture
        .get(&format!("/api/explorations/{exploration}/executions"))
        .await;
    assert_eq!(runs[0]["status"], "interrupted");
    assert!(runs[0]["ended_ms"].as_i64().is_some());
    let mut sql = fixture.sql().await;
    assert!(
        sqlx::query("UPDATE exploration_executions SET status='succeeded' WHERE id=?")
            .bind(run.to_string())
            .execute(&mut sql)
            .await
            .is_err()
    );
    let encoded:String=sqlx::query_scalar("SELECT definition_json FROM exploration_configurations WHERE exploration_id=? AND revision=1").bind(exploration.to_string()).fetch_one(&mut sql).await.unwrap();
    assert!(!encoded.contains("DO-NOT-RETAIN"));
    assert!(
        sqlx::query("UPDATE exploration_configurations SET definition_json='{}'")
            .execute(&mut sql)
            .await
            .is_err()
    );
    drop(sql);
    definition["revision"] = json!(1);
    definition["title"] = json!("Revised hypothesis");
    assert_eq!(
        fixture.put("/api/explorations", definition.clone()).await.0,
        StatusCode::OK
    );
    assert_eq!(
        fixture.put("/api/explorations", definition).await.0,
        StatusCode::CONFLICT
    );
    for index in 1..32 {
        let value = json!({"schema_version":1,"id":Uuid::new_v4(),"revision":0,"title":format!("Free exploration {index}"),"hypothesis":"","scope":null,"steps":[]});
        assert_eq!(
            fixture.put("/api/explorations", value).await.0,
            StatusCode::OK
        );
    }
    let over = json!({"schema_version":1,"id":Uuid::new_v4(),"revision":0,"title":"Quota exceeded","hypothesis":"","scope":null,"steps":[]});
    assert_eq!(
        fixture.put("/api/explorations", over).await.0,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    let (_, list) = fixture.get("/api/explorations").await;
    assert_eq!(list["explorations"].as_array().unwrap().len(), 32);
    assert_eq!(
        fs::read(
            fixture
                .config
                .source_dir
                .as_ref()
                .unwrap()
                .join("original.txt")
        )
        .unwrap(),
        b"original private source"
    );
    fixture.stop().await;
}
