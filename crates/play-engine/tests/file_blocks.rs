#[path = "support/native_schema.rs"]
mod native_schema;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use play_engine::workspace::{Context, Manager};
use serde_json::{Value, json};
use uuid::Uuid;

fn command(manager: &mut Manager, value: Value) -> Value {
    manager
        .handle(serde_json::from_value(value).unwrap())
        .unwrap()
}
fn setup(manager: &mut Manager) -> Context {
    let workspace = command(
        manager,
        json!({"kind":"create","key":Uuid::new_v4(),"title":"File studies"}),
    );
    let created = command(
        manager,
        json!({"kind":"create_exploration","workspace_id":workspace["id"],"key":Uuid::new_v4(),"title":"Evidence"}),
    );
    serde_json::from_value(json!({"workspace_id":workspace["id"],"exploration_id":created["explorationId"],"stage_id":created["stageId"]})).unwrap()
}
fn view(manager: &mut Manager, context: &Context) -> Value {
    command(
        manager,
        json!({"kind":"open_exploration","workspace_id":context.workspace_id,"exploration_id":context.exploration_id,"stage_id":context.stage_id}),
    )
}
fn import(context: &Context, name: &str, bytes: &[u8]) -> Value {
    json!({"kind":"add_file","context":context,"key":Uuid::new_v4(),"title":name,"base64":STANDARD.encode(bytes)})
}
fn source(block: &Value) -> Value {
    json!({"kind":"asset","asset":{"asset_id":block["asset"]["id"],"revision":block["asset"]["revision"]},"caption":null})
}
#[test]
fn imported_files_preserve_exact_bytes_identity_lineage_and_scoped_lifecycle() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let context = setup(&mut manager);
    let data = b"<script>globalThis.compromised=true</script>\nGur xrl.";
    let intent = import(&context, "clue.html", data);
    let receipt = command(&mut manager, intent.clone());
    assert_eq!(command(&mut manager, intent.clone()), receipt);
    let initial = view(&mut manager, &context);
    let block = initial["stage"]["blocks"][0].clone();
    assert_eq!(
        block["asset"]["content"],
        json!({"kind":"file","byteLength":data.len(),"sha256":cube_core::hashing::sha256(data),"preview":{"format":"utf8","text":String::from_utf8(data.to_vec()).unwrap(),"truncated":false}})
    );
    assert_eq!(initial["trails"][0]["pipeline"], Value::Null);
    assert_eq!(initial["stage"]["blocks"][1]["kind"], "note");
    command(
        &mut manager,
        json!({"kind":"duplicate_block","context":context,"key":Uuid::new_v4(),"block_id":block["id"],"expected_revision":initial["stage"]["revision"],"source":source(&block)}),
    );
    let copied = view(&mut manager, &context);
    let copy = copied["stage"]["blocks"][1].clone();
    assert_ne!(copy["asset"]["id"], block["asset"]["id"]);
    assert_eq!(copy["asset"]["content"], block["asset"]["content"]);
    assert_eq!(copy["asset"]["origin"]["assetId"], block["asset"]["id"]);
    command(
        &mut manager,
        json!({"kind":"archive_block","context":context,"key":Uuid::new_v4(),"block_id":block["id"],"expected_revision":copied["stage"]["revision"],"source":source(&block)}),
    );
    let archived = view(&mut manager, &context);
    assert_eq!(
        archived["stage"]["archivedBlocks"][0]["block"]["asset"],
        block["asset"]
    );
    command(
        &mut manager,
        json!({"kind":"restore_block","context":context,"key":Uuid::new_v4(),"block_id":block["id"],"expected_revision":archived["stage"]["revision"],"source":source(&block)}),
    );
    let restored = view(&mut manager, &context);
    assert_eq!(restored["stage"]["blocks"][1], block);
    let edit = json!({"kind":"revise_text","context":context,"key":Uuid::new_v4(),"block_id":block["id"],"asset_id":block["asset"]["id"],"expected_revision":1,"text":"replacement"});
    assert_eq!(
        manager
            .handle(serde_json::from_value(edit).unwrap())
            .unwrap_err(),
        "invalid_asset_type"
    );
    drop(manager);
    let database = root
        .join(context.workspace_id.to_string())
        .join("workspace.sqlite3");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
        let mut db = SqliteConnection::connect_with(
            &SqliteConnectOptions::new()
                .filename(database)
                .read_only(true),
        )
        .await
        .unwrap();
        let stored: Vec<u8> =
            sqlx::query_scalar("SELECT data FROM asset_revisions WHERE asset_id=? AND revision=1")
                .bind(block["asset"]["id"].as_str().unwrap())
                .fetch_one(&mut db)
                .await
                .unwrap();
        assert_eq!(stored, data);
        assert!(
            sqlx::query("PRAGMA foreign_key_check")
                .fetch_optional(&mut db)
                .await
                .unwrap()
                .is_none()
        );
        db.close().await.unwrap();
    });
    let mut manager = Manager::new(root).unwrap();
    command(
        &mut manager,
        json!({"kind":"open","workspace_id":context.workspace_id}),
    );
    assert_eq!(view(&mut manager, &context), restored);
    assert_eq!(command(&mut manager, intent), receipt);
}

#[test]
fn file_previews_are_bounded_unicode_safe_and_binary_exact() {
    let tmp = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(tmp.path().join("private")).unwrap();
    let context = setup(&mut manager);
    let text = "á".repeat(1024);
    command(&mut manager, import(&context, "long.txt", text.as_bytes()));
    command(
        &mut manager,
        import(&context, "data.bin", &[0, 255, 1, 2, 128]),
    );
    command(&mut manager, import(&context, "empty.dat", &[]));
    let boundary = format!("{}💡tail", "a".repeat(255));
    command(
        &mut manager,
        import(&context, "boundary.txt", boundary.as_bytes()),
    );
    let result = view(&mut manager, &context);
    let blocks = result["stage"]["blocks"].as_array().unwrap();
    let long = &blocks[0]["asset"]["content"];
    assert_eq!(long["byteLength"], text.len());
    assert_eq!(long["preview"]["format"], "utf8");
    assert_eq!(long["preview"]["truncated"], true);
    assert_eq!(long["preview"]["text"].as_str().unwrap().len(), 256);
    assert!(text.starts_with(long["preview"]["text"].as_str().unwrap()));
    assert_eq!(
        blocks[1]["asset"]["content"]["preview"],
        json!({"format":"hex","text":"00ff010280","truncated":false})
    );
    assert_eq!(
        blocks[2]["asset"]["content"]["preview"],
        json!({"format":"utf8","text":"","truncated":false})
    );
    assert_eq!(
        blocks[3]["asset"]["content"]["preview"],
        json!({"format":"utf8","text":"a".repeat(255),"truncated":true})
    );
}

#[test]
fn file_import_refuses_overflow_invalid_encoding_and_foreign_context_without_mutation() {
    let tmp = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(tmp.path().join("private")).unwrap();
    let context = setup(&mut manager);
    let before = view(&mut manager, &context);
    let maximum = play_engine::protocol::contract().file_bytes;
    let mut bad = import(&context, "bad.bin", b"valid");
    bad["base64"] = json!("====");
    let mut foreign = context.clone();
    foreign.exploration_id = Uuid::new_v4();
    for (intent, code) in [
        (bad, "invalid_file"),
        (
            import(&context, "large.bin", &vec![0; maximum + 1]),
            "file_limit",
        ),
        (import(&foreign, "foreign.bin", b"private"), "not_found"),
    ] {
        assert_eq!(
            manager
                .handle(serde_json::from_value(intent).unwrap())
                .unwrap_err(),
            code
        );
        assert_eq!(view(&mut manager, &context), before);
    }
    let maximum_data = vec![255; maximum];
    let intent = import(&context, "maximum.bin", &maximum_data);
    command(&mut manager, intent.clone());
    let accepted = view(&mut manager, &context);
    assert_eq!(
        accepted["stage"]["blocks"][0]["asset"]["content"]["byteLength"],
        maximum
    );
    assert_eq!(
        accepted["stage"]["blocks"][0]["asset"]["content"]["preview"]["truncated"],
        true
    );
    let mut retry = intent;
    retry["base64"] = json!(STANDARD.encode(b"changed"));
    assert_eq!(
        manager
            .handle(serde_json::from_value(retry).unwrap())
            .unwrap_err(),
        "idempotency_conflict"
    );
    assert_eq!(view(&mut manager, &context), accepted);
    for index in 0..6 {
        command(&mut manager, import(&context, &format!("{index}.bin"), &[]));
    }
    let full = view(&mut manager, &context);
    assert_eq!(
        manager
            .handle(serde_json::from_value(import(&context, "overflow.bin", b"no slot")).unwrap())
            .unwrap_err(),
        "storage_quota"
    );
    assert_eq!(view(&mut manager, &context), full);
}

#[test]
fn schema_four_upgrade_preserves_archived_text_lineage_and_runs_before_import() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let context = setup(&mut manager);
    command(
        &mut manager,
        json!({"kind":"add_text","context":context,"key":Uuid::new_v4(),"title":"Clue","text":"Gur xrl."}),
    );
    let initial = view(&mut manager, &context);
    let block = &initial["stage"]["blocks"][0];
    let trail = &initial["trails"][0];
    let request=serde_json::from_value(json!({"context":context,"key":Uuid::new_v4(),"trail_id":trail["id"],"pipeline_id":trail["pipeline"]["id"],"pipeline_revision":1,"source_asset_id":block["asset"]["id"],"source_revision":1,"operation":"cipher.rot13","operation_version":1,"retain_output":true})).unwrap();
    let run = manager.prepare_run(&request).unwrap();
    manager.start_attempt(&run.identity).unwrap();
    let completed = manager
        .finish_run(
            &run.identity,
            json!({"status":"completed","output":{"text":"The key."}}),
        )
        .unwrap();
    command(
        &mut manager,
        json!({"kind":"duplicate_block","context":context,"key":Uuid::new_v4(),"block_id":block["id"],"expected_revision":initial["stage"]["revision"],"source":source(block)}),
    );
    let copied = view(&mut manager, &context);
    command(
        &mut manager,
        json!({"kind":"archive_block","context":context,"key":Uuid::new_v4(),"block_id":block["id"],"expected_revision":copied["stage"]["revision"],"source":source(block)}),
    );
    let before = view(&mut manager, &context);
    drop(manager);
    native_schema::snapshot(&root, context.workspace_id, 4);
    let mut manager = Manager::new(root).unwrap();
    command(
        &mut manager,
        json!({"kind":"open","workspace_id":context.workspace_id}),
    );
    assert_eq!(view(&mut manager, &context), before);
    assert_eq!(
        manager.run_detail(&context, run.identity.id).unwrap(),
        completed
    );
    command(&mut manager, import(&context, "new.dat", b"kept"));
    let after = view(&mut manager, &context);
    assert_eq!(
        after["stage"]["archivedBlocks"],
        before["stage"]["archivedBlocks"]
    );
    assert_eq!(after["stage"]["blocks"][0], before["stage"]["blocks"][0]);
    assert_eq!(
        after["stage"]["blocks"][1]["asset"]["content"]["kind"],
        "file"
    );
}
