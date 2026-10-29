use play_engine::workspace::{Command, Manager, RunRequest};
use serde_json::{Value, json};
use std::{fs, path::Path};
use uuid::Uuid;

fn command(manager: &mut Manager, value: Value) -> Value {
    manager
        .handle(serde_json::from_value(value).unwrap())
        .unwrap()
}
fn create(manager: &mut Manager) -> String {
    command(
        manager,
        json!({"kind":"create","key":Uuid::new_v4(),"title":"Notebook workspace"}),
    )["id"]
        .as_str()
        .unwrap()
        .into()
}
fn exploration(manager: &mut Manager, workspace: &str, title: &str) -> Value {
    command(
        manager,
        json!({"kind":"create_exploration","workspace_id":workspace,"key":Uuid::new_v4(),"title":title}),
    )
}
fn open(manager: &mut Manager, workspace: &str, id: &str) -> Value {
    command(
        manager,
        json!({"kind":"open_exploration","workspace_id":workspace,"exploration_id":id,"stage_id":null}),
    )
}
fn files(directory: &Path) -> Vec<(String, Vec<u8>)> {
    let mut entries = fs::read_dir(directory)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (
                entry.file_name().to_string_lossy().to_string(),
                fs::read(entry.path()).unwrap(),
            )
        })
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    entries
}

#[test]
fn two_explorations_reopen_independent_notebooks_and_immutable_notes() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let workspace = create(&mut manager);
    let a = exploration(&mut manager, &workspace, "First");
    let b = exploration(&mut manager, &workspace, "Second");
    let a_id = a["explorationId"].as_str().unwrap();
    let b_id = b["explorationId"].as_str().unwrap();
    let a_view = open(&mut manager, &workspace, a_id);
    let b_view = open(&mut manager, &workspace, b_id);
    let context = json!({"workspace_id":workspace,"exploration_id":a_id,"stage_id":a["stageId"]});
    let note_key = Uuid::new_v4();
    let save = json!({"kind":"save_note","context":context,"key":note_key,"note_id":a_view["stage"]["note"]["id"],"expected_revision":1,"text":"Only the first investigation."});
    let accepted = command(&mut manager, save.clone());
    assert_eq!(accepted["revision"], 2);
    assert_eq!(
        command(&mut manager, save.clone()),
        accepted,
        "same-action retry must be idempotent"
    );
    let mut different = save;
    different["text"] = json!("different payload");
    assert_eq!(
        manager
            .handle(serde_json::from_value(different).unwrap())
            .unwrap_err(),
        "idempotency_conflict"
    );
    let mixed = json!({"kind":"save_note","context":context,"key":Uuid::new_v4(),"note_id":b_view["stage"]["note"]["id"],"expected_revision":1,"text":"must not cross owners"});
    assert!(
        manager
            .handle(serde_json::from_value(mixed).unwrap())
            .is_err()
    );
    let stale = json!({"kind":"save_note","context":context,"key":Uuid::new_v4(),"note_id":a_view["stage"]["note"]["id"],"expected_revision":1,"text":"stale"});
    assert_eq!(
        manager
            .handle(serde_json::from_value(stale).unwrap())
            .unwrap_err(),
        "state_conflict"
    );
    command(
        &mut manager,
        json!({"kind":"add_text","context":context,"key":Uuid::new_v4(),"title":"Caption clue","text":"Gur xrl vf va gur cvkryf."}),
    );
    let saved = open(&mut manager, &workspace, a_id);
    let asset = &saved["stage"]["blocks"][0]["asset"];
    let mut request:RunRequest=serde_json::from_value(json!({"context":context,"key":Uuid::new_v4(),"trail_id":saved["trails"][0]["id"],"pipeline_id":saved["trails"][0]["pipeline"]["id"],"pipeline_revision":1,"source_asset_id":saved["trails"][0]["source"]["id"],"source_revision":saved["trails"][0]["source"]["revision"],"operation":"cipher.rot13","operation_version":1,"retain_output":true})).unwrap();
    let prepared = manager.prepare_run(&request).unwrap();
    assert_eq!(prepared.invocation.input, "Gur xrl vf va gur cvkryf.");
    manager
        .finish_run(
            &prepared.identity,
            json!({"status":"cancelled","code":"validation_cancelled"}),
        )
        .unwrap();
    request.context.exploration_id = Uuid::parse_str(b_id).unwrap();
    request.context.stage_id = Uuid::parse_str(b["stageId"].as_str().unwrap()).unwrap();
    assert!(manager.prepare_run(&request).is_err());
    drop(manager);
    let mut manager = Manager::new(root).unwrap();
    command(
        &mut manager,
        json!({"kind":"open","workspace_id":workspace}),
    );
    let restored = open(&mut manager, &workspace, a_id);
    assert_eq!(
        restored["stage"]["note"]["text"],
        "Only the first investigation."
    );
    assert_eq!(restored["stage"]["note"]["revision"], 2);
    assert_eq!(restored["stage"]["blocks"][0]["asset"]["id"], asset["id"]);
    let separate = open(&mut manager, &workspace, b_id);
    assert_eq!(separate["stage"]["note"]["text"], "");
    assert!(separate["stage"]["blocks"].as_array().unwrap().is_empty());
}

#[test]
fn owner_lock_prevents_a_second_writer_and_releases_after_close() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("private");
    let mut first = Manager::new(root.clone()).unwrap();
    let workspace = create(&mut first);
    let mut second = Manager::new(root).unwrap();
    let request = Command::Open {
        workspace_id: Uuid::parse_str(&workspace).unwrap(),
    };
    assert_eq!(
        second.handle(request.clone()).unwrap_err(),
        "workspace_busy"
    );
    drop(first);
    assert_eq!(second.handle(request).unwrap()["id"], workspace);
}

#[test]
fn incompatible_generation_is_rejected_without_touching_database_or_sidecars() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let workspace = create(&mut manager);
    drop(manager);
    let directory = root.join(&workspace);
    let database = directory.join("workspace.sqlite3");
    let mut bytes = fs::read(&database).unwrap();
    bytes[60..64].copy_from_slice(&99u32.to_be_bytes());
    fs::write(&database, bytes).unwrap();
    fs::write(
        directory.join("workspace.sqlite3-wal"),
        b"unrecognized evidence",
    )
    .unwrap();
    let before = files(&directory);
    let recent = fs::read(root.join("recent.json")).unwrap();
    let mut manager = Manager::new(root.clone()).unwrap();
    assert_eq!(
        manager
            .handle(Command::Open {
                workspace_id: Uuid::parse_str(&workspace).unwrap()
            })
            .unwrap_err(),
        "unsupported_store"
    );
    assert_eq!(files(&directory), before);
    assert_eq!(fs::read(root.join("recent.json")).unwrap(), recent);
}

#[test]
fn creation_retries_reuse_identity_and_incomplete_initialization_is_not_adopted() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let key = Uuid::new_v4();
    let request = Command::Create {
        key,
        title: "Workspace".into(),
    };
    let first = manager.handle(request.clone()).unwrap();
    assert_eq!(manager.handle(request).unwrap()["id"], first["id"]);
    assert_eq!(
        manager
            .handle(Command::Create {
                key,
                title: "Other content".into()
            })
            .unwrap_err(),
        "idempotency_conflict"
    );
    let partial = root.join(format!(".creating-{}", Uuid::new_v4()));
    fs::create_dir(&partial).unwrap();
    fs::write(partial.join("workspace.sqlite3"), b"partial").unwrap();
    let listing = manager.handle(Command::List).unwrap();
    assert_eq!(listing["workspaces"].as_array().unwrap().len(), 1);
    assert_eq!(
        fs::read(partial.join("workspace.sqlite3")).unwrap(),
        b"partial"
    );
}

#[test]
fn png_caption_import_is_atomic_and_preserves_independent_asset_references() {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let workspace = create(&mut manager);
    let exploration = exploration(&mut manager, &workspace, "Image notebook");
    let id = exploration["explorationId"].as_str().unwrap();
    let context =
        json!({"workspace_id":workspace,"exploration_id":id,"stage_id":exploration["stageId"]});
    let png = include_bytes!("../../../tests/fixtures/sample.png");
    let key = Uuid::new_v4();
    let import = json!({"kind":"add_image","context":context,"key":key,"title":"sample.png","caption":"Gur xrl.","png":STANDARD.encode(png)});
    let mut malformed = import.clone();
    malformed["png"] = json!(STANDARD.encode(b"not a PNG"));
    assert_eq!(
        manager
            .handle(serde_json::from_value(malformed).unwrap())
            .unwrap_err(),
        "invalid_image"
    );
    assert!(
        open(&mut manager, &workspace, id)["stage"]["blocks"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let result = command(&mut manager, import.clone());
    assert_eq!(command(&mut manager, import), result);
    let view = open(&mut manager, &workspace, id);
    let block = &view["stage"]["blocks"][0];
    assert_eq!(view["stage"]["blocks"].as_array().unwrap().len(), 1);
    assert_ne!(block["asset"]["id"], block["caption"]["id"]);
    assert_eq!(view["trails"].as_array().unwrap().len(), 2);
    assert_eq!(block["caption"]["content"]["text"], "Gur xrl.");
    let read =
        json!({"kind":"read_image","context":context,"asset_id":block["asset"]["id"],"revision":1});
    assert_eq!(
        command(&mut manager, read.clone())["base64"],
        STANDARD.encode(png)
    );
    drop(manager);
    let mut manager = Manager::new(root).unwrap();
    command(
        &mut manager,
        json!({"kind":"open","workspace_id":workspace}),
    );
    assert_eq!(command(&mut manager, read)["base64"], STANDARD.encode(png));
    assert_eq!(
        open(&mut manager, &workspace, id)["stage"]["blocks"],
        view["stage"]["blocks"]
    );
}

#[test]
fn quotas_reject_atomically_and_stage_ownership_is_enforced() {
    let temporary = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(temporary.path().join("private")).unwrap();
    let workspace = create(&mut manager);
    let a = exploration(&mut manager, &workspace, "First");
    let b = exploration(&mut manager, &workspace, "Second");
    let id = a["explorationId"].as_str().unwrap();
    let context = json!({"workspace_id":workspace,"exploration_id":id,"stage_id":a["stageId"]});
    let add = |text: String| json!({"kind":"add_text","context":context,"key":Uuid::new_v4(),"title":"Clue","text":text});
    assert_eq!(
        manager
            .handle(serde_json::from_value(add("é".repeat(513))).unwrap())
            .unwrap_err(),
        "input_limit"
    );
    let mut wrong = add("wrong stage".into());
    wrong["context"]["stage_id"] = b["stageId"].clone();
    assert_eq!(
        manager
            .handle(serde_json::from_value(wrong).unwrap())
            .unwrap_err(),
        "not_found"
    );
    for _ in 0..7 {
        command(&mut manager, add("clue".into()));
    }
    assert_eq!(
        manager
            .handle(serde_json::from_value(add("eighth content block".into())).unwrap())
            .unwrap_err(),
        "storage_quota"
    );
    assert_eq!(
        open(&mut manager, &workspace, id)["stage"]["blocks"]
            .as_array()
            .unwrap()
            .len(),
        7
    );
    let note = open(&mut manager, &workspace, id)["stage"]["note"].clone();
    let save = json!({"kind":"save_note","context":context,"key":Uuid::new_v4(),"note_id":note["id"],"expected_revision":1,"text":"x".repeat(2049)});
    assert_eq!(
        manager
            .handle(serde_json::from_value(save).unwrap())
            .unwrap_err(),
        "input_limit"
    );
    assert_eq!(open(&mut manager, &workspace, id)["stage"]["note"], note);
}

#[test]
fn schema_tampering_is_rejected_read_only_and_foreign_keys_protect_immutable_evidence() {
    use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let workspace = create(&mut manager);
    let a = exploration(&mut manager, &workspace, "First");
    let b = exploration(&mut manager, &workspace, "Second");
    let first = open(
        &mut manager,
        &workspace,
        a["explorationId"].as_str().unwrap(),
    );
    drop(manager);
    let directory = root.join(&workspace);
    let database = directory.join("workspace.sqlite3");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let mut db = SqliteConnection::connect_with(&SqliteConnectOptions::new().filename(&database).foreign_keys(true)).await.unwrap();
        let failed = sqlx::query("INSERT INTO blocks(workspace_id,exploration_id,stage_id,id,position,kind,note_id) VALUES(?,?,?,?,0,'note',?)")
            .bind(&workspace).bind(b["explorationId"].as_str().unwrap()).bind(b["stageId"].as_str().unwrap()).bind(Uuid::new_v4().to_string()).bind(first["stage"]["note"]["id"].as_str().unwrap()).execute(&mut db).await.unwrap_err();
        assert!(matches!(failed, sqlx::Error::Database(ref error) if error.is_foreign_key_violation()));
        assert!(sqlx::query("UPDATE note_revisions SET text='overwrite'").execute(&mut db).await.is_err());
        assert!(sqlx::query("DELETE FROM note_revisions").execute(&mut db).await.is_err());
        sqlx::query("CREATE TABLE unrecognized_format(secret TEXT)").execute(&mut db).await.unwrap();
        db.close().await.unwrap();
    });
    let before = files(&directory);
    let recent = fs::read(root.join("recent.json")).unwrap();
    let mut manager = Manager::new(root.clone()).unwrap();
    assert_eq!(
        manager
            .handle(Command::Open {
                workspace_id: Uuid::parse_str(&workspace).unwrap()
            })
            .unwrap_err(),
        "unsupported_store"
    );
    assert_eq!(files(&directory), before);
    assert_eq!(fs::read(root.join("recent.json")).unwrap(), recent);
}

#[test]
fn interrupted_initializations_consume_quota_and_are_never_reset() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    for _ in 0..play_engine::protocol::contract().workspace_limit {
        let path = root.join(format!(".creating-{}", Uuid::new_v4()));
        fs::create_dir(&path).unwrap();
        fs::write(path.join("partial"), b"preserve").unwrap();
    }
    assert_eq!(
        manager
            .handle(Command::Create {
                key: Uuid::new_v4(),
                title: "New".into()
            })
            .unwrap_err(),
        "storage_quota"
    );
    assert_eq!(
        manager.handle(Command::List).unwrap()["workspaces"],
        json!([])
    );
}

#[test]
fn crash_transaction_writer() {
    use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
    let Some(path) = std::env::var_os("PLAY_TEST_CRASH_DATABASE") else {
        return;
    };
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let mut db = SqliteConnection::connect_with(&SqliteConnectOptions::new().filename(path).foreign_keys(true)).await.unwrap();
        sqlx::raw_sql("PRAGMA cache_size=1; PRAGMA cache_spill=1; BEGIN IMMEDIATE; UPDATE notes SET current_revision=99;")
            .execute(&mut db).await.unwrap();
        sqlx::query("INSERT INTO receipts SELECT id,'crash','incomplete','digest',? FROM workspace_meta")
            .bind("x".repeat(12000)).execute(&mut db).await.unwrap();
        // Deliberately bypass connection/transaction destructors to leave a hot journal.
        std::process::exit(41);
    });
}

#[test]
fn a_known_store_recovers_an_interrupted_sqlite_transaction_after_owner_death() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let workspace = create(&mut manager);
    let created = exploration(&mut manager, &workspace, "Crash recovery");
    let id = created["explorationId"].as_str().unwrap();
    let before = open(&mut manager, &workspace, id);
    drop(manager);
    let directory = root.join(&workspace);
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "crash_transaction_writer"])
        .env(
            "PLAY_TEST_CRASH_DATABASE",
            directory.join("workspace.sqlite3"),
        )
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(41));
    assert!(
        directory
            .join("workspace.sqlite3-journal")
            .metadata()
            .unwrap()
            .len()
            > 0
    );
    let mut manager = Manager::new(root).unwrap();
    command(
        &mut manager,
        json!({"kind":"open","workspace_id":workspace}),
    );
    assert_eq!(
        open(&mut manager, &workspace, id)["stage"]["note"],
        before["stage"]["note"]
    );
    assert!(!directory.join("workspace.sqlite3-journal").exists());
}

#[test]
fn missing_and_corrupt_stores_are_preserved_without_recreation() {
    for mode in ["missing", "corrupt"] {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("private");
        let mut manager = Manager::new(root.clone()).unwrap();
        let workspace = create(&mut manager);
        drop(manager);
        let directory = root.join(&workspace);
        let path = directory.join("workspace.sqlite3");
        if mode == "missing" {
            fs::remove_file(&path).unwrap();
        } else {
            fs::write(&path, b"broken database").unwrap();
        }
        let before = files(&directory);
        let mut manager = Manager::new(root).unwrap();
        assert!(
            manager
                .handle(Command::Open {
                    workspace_id: Uuid::parse_str(&workspace).unwrap()
                })
                .is_err()
        );
        assert_eq!(files(&directory), before);
    }
}

#[cfg(unix)]
#[test]
fn linked_private_files_and_unsafe_permissions_cannot_be_opened() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    for mode in ["symlink", "hardlink", "permissions"] {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("private");
        let mut manager = Manager::new(root.clone()).unwrap();
        let workspace = create(&mut manager);
        drop(manager);
        let directory = root.join(&workspace);
        let database = directory.join("workspace.sqlite3");
        let original = fs::read(&database).unwrap();
        let outside = temporary.path().join("source");
        match mode {
            "symlink" => {
                fs::rename(&database, &outside).unwrap();
                symlink(&outside, &database).unwrap();
            }
            "hardlink" => fs::hard_link(&database, &outside).unwrap(),
            _ => fs::set_permissions(&database, fs::Permissions::from_mode(0o644)).unwrap(),
        }
        let mut manager = Manager::new(root).unwrap();
        assert!(
            manager
                .handle(Command::Open {
                    workspace_id: Uuid::parse_str(&workspace).unwrap()
                })
                .is_err()
        );
        assert_eq!(fs::read(&database).unwrap(), original);
        if outside.exists() {
            assert_eq!(fs::read(&outside).unwrap(), original);
        }
    }
}
