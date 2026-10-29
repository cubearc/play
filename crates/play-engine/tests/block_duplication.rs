#[path = "support/native_schema.rs"]
mod native_schema;
use base64::{Engine as _, engine::general_purpose::STANDARD};
use play_engine::workspace::{Context, Manager, RunRequest};
use serde_json::{Value, json};
use uuid::Uuid;

fn command(manager: &mut Manager, value: Value) -> Value {
    manager
        .handle(serde_json::from_value(value).unwrap())
        .unwrap()
}
fn open(manager: &mut Manager, context: &Context) -> Value {
    command(
        manager,
        json!({"kind":"open_exploration","workspace_id":context.workspace_id,"exploration_id":context.exploration_id,"stage_id":context.stage_id}),
    )
}
fn setup(manager: &mut Manager, image: bool) -> (Context, Value) {
    let workspace = command(
        manager,
        json!({"kind":"create","key":Uuid::new_v4(),"title":"Copies"}),
    );
    let created = command(
        manager,
        json!({"kind":"create_exploration","workspace_id":workspace["id"],"key":Uuid::new_v4(),"title":"Clues"}),
    );
    let context:Context=serde_json::from_value(json!({"workspace_id":workspace["id"],"exploration_id":created["explorationId"],"stage_id":created["stageId"]})).unwrap();
    let add = if image {
        json!({"kind":"add_image","context":context,"key":Uuid::new_v4(),"title":"sample.png","caption":"Gur xrl.","png":STANDARD.encode(include_bytes!("../../../tests/fixtures/sample.png"))})
    } else {
        json!({"kind":"add_text","context":context,"key":Uuid::new_v4(),"title":"Clue","text":"Gur xrl."})
    };
    command(manager, add);
    let view = open(manager, &context);
    (context, view)
}
fn copy(context: &Context, view: &Value, index: usize) -> Value {
    let block = &view["stage"]["blocks"][index];
    let reference = |asset: &Value| json!({"asset_id":asset["id"],"revision":asset["revision"]});
    json!({"kind":"duplicate_block","context":context,"key":Uuid::new_v4(),"block_id":block["id"],"expected_revision":view["stage"]["revision"],"source":{"kind":"asset","asset":reference(&block["asset"]),"caption":block.get("caption").map(reference)}})
}
fn edit(manager: &mut Manager, context: &Context, block: &Value, asset: &Value, text: &str) {
    command(
        manager,
        json!({"kind":"revise_text","context":context,"key":Uuid::new_v4(),"block_id":block["id"],"asset_id":asset["id"],"expected_revision":asset["revision"],"text":text}),
    );
}
#[test]
fn text_copy_forks_current_content_with_lineage_independent_edits_and_no_invented_history() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let (context, initial) = setup(&mut manager, false);
    let source = &initial["stage"]["blocks"][0];
    edit(&mut manager, &context, source, &source["asset"], "Svefg.");
    let before = open(&mut manager, &context);
    let source = &before["stage"]["blocks"][0];
    let trail = &before["trails"][0];
    let request:RunRequest=serde_json::from_value(json!({"context":context,"key":Uuid::new_v4(),"trail_id":trail["id"],"pipeline_id":trail["pipeline"]["id"],"pipeline_revision":trail["pipeline"]["revision"],"source_asset_id":trail["source"]["id"],"source_revision":2,"operation":"cipher.rot13","operation_version":1,"retain_output":true})).unwrap();
    let running = manager.prepare_run(&request).unwrap();
    manager.start_attempt(&running.identity).unwrap();
    let intent = copy(&context, &before, 0);
    let result = command(&mut manager, intent.clone());
    assert_eq!(command(&mut manager, intent.clone()), result);
    let after = open(&mut manager, &context);
    let copied = after["stage"]["blocks"][1].clone();
    assert_eq!(copied["id"], result["blockId"]);
    assert_ne!(copied["id"], source["id"]);
    assert_ne!(copied["asset"]["id"], source["asset"]["id"]);
    assert_eq!(copied["asset"]["title"], "Clue (copy)");
    assert_eq!(copied["asset"]["revision"], 1);
    assert_eq!(copied["asset"]["content"], source["asset"]["content"]);
    assert_eq!(
        copied["asset"]["origin"],
        json!({"kind":"asset","assetId":source["asset"]["id"],"revision":2,"title":"Clue"})
    );
    assert_eq!(
        after["stage"]["revision"],
        before["stage"]["revision"].as_i64().unwrap() + 1
    );
    assert_eq!(after["stage"]["blocks"][0], *source);
    assert_eq!(after["stage"]["blocks"][2], before["stage"]["blocks"][1]);
    let new_trail = after["trails"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["source"]["id"] == copied["asset"]["id"])
        .unwrap();
    assert_ne!(new_trail["id"], trail["id"]);
    assert_eq!(new_trail["pipeline"], trail["pipeline"]);
    assert_eq!(
        command(
            &mut manager,
            json!({"kind":"list_runs","context":context,"trail_id":new_trail["id"],"before":null})
        )["runs"],
        json!([])
    );
    edit(&mut manager, &context, &copied, &copied["asset"], "Frpbaq.");
    let edited = open(&mut manager, &context);
    assert_eq!(edited["stage"]["blocks"][0], *source);
    assert_eq!(
        edited["stage"]["blocks"][1]["asset"]["content"]["text"],
        "Frpbaq."
    );
    assert_eq!(
        edited["stage"]["blocks"][1]["asset"]["origin"],
        copied["asset"]["origin"]
    );
    let old_run=manager.finish_run(&running.identity,json!({"status":"completed","output":{"text":cube_core::ciphers::rot13(&running.invocation.input)}})).unwrap();
    assert_eq!(old_run["result"]["outputs"]["output"], "First.");
    assert_eq!(
        old_run["plan"]["inputs"]["input"]["assetId"],
        source["asset"]["id"]
    );
    let nested = command(&mut manager, copy(&context, &edited, 1));
    let nested_view = open(&mut manager, &context);
    let nested_block = &nested_view["stage"]["blocks"][2];
    assert_eq!(nested_block["id"], nested["blockId"]);
    assert_eq!(
        nested_block["asset"]["origin"]["assetId"],
        copied["asset"]["id"]
    );
    assert_eq!(nested_block["asset"]["origin"]["revision"], 2);
    let mut changed = intent;
    changed["source"]["asset"]["revision"] = json!(1);
    assert_eq!(
        manager
            .handle(serde_json::from_value(changed).unwrap())
            .unwrap_err(),
        "idempotency_conflict"
    );
    drop(manager);
    let mut manager = Manager::new(root).unwrap();
    command(
        &mut manager,
        json!({"kind":"open","workspace_id":context.workspace_id}),
    );
    assert_eq!(open(&mut manager, &context), nested_view);
    assert_eq!(
        manager.run_detail(&context, running.identity.id).unwrap(),
        old_run
    );
}
#[test]
fn copying_an_image_keeps_its_caption_grouped_but_forks_both_asset_identities() {
    let tmp = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(tmp.path().join("private")).unwrap();
    let (context, before) = setup(&mut manager, true);
    let original = &before["stage"]["blocks"][0];
    command(&mut manager, copy(&context, &before, 0));
    let after = open(&mut manager, &context);
    let copied = &after["stage"]["blocks"][1];
    assert_eq!(copied["kind"], "asset");
    assert_eq!(after["stage"]["blocks"].as_array().unwrap().len(), 3);
    for field in ["asset", "caption"] {
        assert_ne!(copied[field]["id"], original[field]["id"]);
        assert_eq!(copied[field]["revision"], 1);
        assert_eq!(copied[field]["content"], original[field]["content"]);
        assert_eq!(copied[field]["origin"]["assetId"], original[field]["id"]);
        assert_eq!(
            copied[field]["origin"]["revision"],
            original[field]["revision"]
        );
    }
    let read =
        |id: &Value| json!({"kind":"read_image","context":context,"asset_id":id,"revision":1});
    let original_png = command(&mut manager, read(&original["asset"]["id"]));
    let copied_png = command(&mut manager, read(&copied["asset"]["id"]));
    assert_eq!(original_png, copied_png);
    edit(
        &mut manager,
        &context,
        copied,
        &copied["caption"],
        "Frpbaq.",
    );
    let edited = open(&mut manager, &context);
    assert_eq!(edited["stage"]["blocks"][0], *original);
    assert_eq!(edited["stage"]["blocks"][1]["asset"], copied["asset"]);
    assert_eq!(edited["stage"]["blocks"][1]["caption"]["revision"], 2);
}
#[test]
fn duplicate_rejects_stale_content_order_ownership_reasoning_and_limits_atomically() {
    let tmp = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(tmp.path().join("private")).unwrap();
    let (context, before) = setup(&mut manager, true);
    let intent = copy(&context, &before, 0);
    let source = &before["stage"]["blocks"][0];
    edit(
        &mut manager,
        &context,
        source,
        &source["caption"],
        "Changed caption",
    );
    let current = open(&mut manager, &context);
    assert_eq!(
        manager
            .handle(serde_json::from_value(intent).unwrap())
            .unwrap_err(),
        "state_conflict"
    );
    assert_eq!(open(&mut manager, &context), current);
    let mut intent = copy(&context, &current, 0);
    intent["context"]["exploration_id"] = json!(Uuid::new_v4());
    assert_eq!(
        manager
            .handle(serde_json::from_value(intent).unwrap())
            .unwrap_err(),
        "not_found"
    );
    let mut intent = copy(&context, &current, 0);
    intent["block_id"] = current["stage"]["blocks"][1]["id"].clone();
    assert_eq!(
        manager
            .handle(serde_json::from_value(intent).unwrap())
            .unwrap_err(),
        "invalid_block_type"
    );
    for _ in 0..6 {
        let view = open(&mut manager, &context);
        command(&mut manager, copy(&context, &view, 0));
    }
    let full = open(&mut manager, &context);
    assert_eq!(full["stage"]["blocks"].as_array().unwrap().len(), 8);
    assert_eq!(
        manager
            .handle(serde_json::from_value(copy(&context, &full, 0)).unwrap())
            .unwrap_err(),
        "storage_quota"
    );
    assert_eq!(open(&mut manager, &context), full);
    let mut stale = copy(&context, &full, 0);
    stale["expected_revision"] = current["stage"]["revision"].clone();
    assert_eq!(
        manager
            .handle(serde_json::from_value(stale).unwrap())
            .unwrap_err(),
        "state_conflict"
    );
}

#[test]
fn repeated_copies_have_readable_distinct_titles_with_bounded_unicode() {
    let tmp = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(tmp.path().join("private")).unwrap();
    let (context, initial) = setup(&mut manager, false);
    let title = "é".repeat(60);
    command(
        &mut manager,
        json!({"kind":"add_text","context":context,"key":Uuid::new_v4(),"title":title,"text":"Clue"}),
    );
    for _ in 0..3 {
        let view = open(&mut manager, &context);
        command(&mut manager, copy(&context, &view, 0));
    }
    let view = open(&mut manager, &context);
    let titles: Vec<_> = view["stage"]["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|b| b["asset"]["title"].as_str())
        .collect();
    assert!(titles.contains(&"Clue (copy)"));
    assert!(titles.contains(&"Clue (copy 2)"));
    assert!(titles.contains(&"Clue (copy 3)"));
    let index = view["stage"]["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .position(|b| b["asset"]["title"] == title)
        .unwrap();
    command(&mut manager, copy(&context, &view, index));
    let copied = open(&mut manager, &context)["stage"]["blocks"][index + 1].clone();
    assert!(copied["asset"]["title"].as_str().unwrap().len() <= 120);
    assert_eq!(copied["asset"]["origin"]["title"], title);
    assert_eq!(initial["stage"]["blocks"][0], view["stage"]["blocks"][0]);
}

fn with_database(
    root: &std::path::Path,
    workspace: Uuid,
    action: impl AsyncFnOnce(&mut sqlx::SqliteConnection),
) {
    use sqlx::{Connection, sqlite::SqliteConnectOptions};
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let mut connection = sqlx::SqliteConnection::connect_with(
            &SqliteConnectOptions::new()
                .filename(root.join(workspace.to_string()).join("workspace.sqlite3"))
                .foreign_keys(false),
        )
        .await
        .unwrap();
        action(&mut connection).await;
        connection.close().await.unwrap();
    });
}
#[test]
fn native_schema_two_and_interrupted_copies_upgrade_without_reassigning_evidence() {
    for interrupted in [false, true] {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("private");
        let mut manager = Manager::new(root.clone()).unwrap();
        let (context, before) = setup(&mut manager, false);
        let trail = &before["trails"][0];
        let request:RunRequest=serde_json::from_value(json!({"context":context,"key":Uuid::new_v4(),"trail_id":trail["id"],"pipeline_id":trail["pipeline"]["id"],"pipeline_revision":1,"source_asset_id":trail["source"]["id"],"source_revision":1,"operation":"cipher.rot13","operation_version":1,"retain_output":true})).unwrap();
        let prepared = manager.prepare_run(&request).unwrap();
        manager.start_attempt(&prepared.identity).unwrap();
        let run = manager
            .finish_run(
                &prepared.identity,
                json!({"status":"completed","output":{"text":"The key."}}),
            )
            .unwrap();
        drop(manager);
        native_schema::snapshot(&root, context.workspace_id, 2);
        let directory = root.join(context.workspace_id.to_string());
        let identity = std::fs::read(directory.join("workspace.json")).unwrap();
        let temporary = Uuid::new_v4();
        let private = directory.join(format!(".upgrade-{temporary}.sqlite3"));
        if interrupted {
            std::fs::write(&private, b"unpublished interrupted copy").unwrap();
            let marker = directory.join("upgrade.json");
            std::fs::write(&marker,serde_json::to_vec(&json!({"product":"cubearc.play.workspace.upgrade","workspace_id":context.workspace_id,"generation":2,"from_schema":2,"to_schema":3,"temporary":temporary})).unwrap()).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                for path in [&private, &marker] {
                    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600)).unwrap();
                }
            }
        }
        let mut manager = Manager::new(root.clone()).unwrap();
        command(
            &mut manager,
            json!({"kind":"open","workspace_id":context.workspace_id}),
        );
        assert_eq!(open(&mut manager, &context), before);
        assert_eq!(
            manager.run_detail(&context, prepared.identity.id).unwrap(),
            run
        );
        assert_eq!(
            std::fs::read(directory.join("workspace.json")).unwrap(),
            identity
        );
        assert!(!private.exists());
        assert!(!directory.join("upgrade.json").exists());
        command(&mut manager, copy(&context, &before, 0));
        let after = open(&mut manager, &context);
        let copied_asset = after["stage"]["blocks"][1]["asset"]["id"]
            .as_str()
            .unwrap()
            .to_owned();
        drop(manager);
        with_database(&root, context.workspace_id, async |connection| {
            let version: i64 = sqlx::query_scalar("PRAGMA user_version")
                .fetch_one(&mut *connection)
                .await
                .unwrap();
            assert_eq!(
                version,
                i64::from(play_engine::protocol::contract().schema_version)
            );
            sqlx::query("PRAGMA foreign_keys=ON")
                .execute(&mut *connection)
                .await
                .unwrap();
            assert!(
                sqlx::query("UPDATE asset_origins SET source_revision=2 WHERE asset_id=?")
                    .bind(&copied_asset)
                    .execute(&mut *connection)
                    .await
                    .unwrap_err()
                    .to_string()
                    .contains("immutable_origin")
            );
            assert!(
                sqlx::query("DELETE FROM asset_origins WHERE asset_id=?")
                    .bind(&copied_asset)
                    .execute(&mut *connection)
                    .await
                    .unwrap_err()
                    .to_string()
                    .contains("immutable_origin")
            );
            assert!(
                sqlx::query("INSERT INTO asset_origins VALUES(?,?,?,?,1)")
                    .bind(context.workspace_id.to_string())
                    .bind(context.exploration_id.to_string())
                    .bind(
                        before["stage"]["blocks"][0]["asset"]["id"]
                            .as_str()
                            .unwrap()
                    )
                    .bind(Uuid::new_v4().to_string())
                    .execute(&mut *connection)
                    .await
                    .is_err()
            );
        });
    }
}
#[test]
fn image_copy_rolls_back_its_first_asset_when_the_caption_exceeds_retained_byte_quota() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let (context, before) = setup(&mut manager, true);
    drop(manager);
    let png = include_bytes!("../../../tests/fixtures/sample.png").len();
    let caption = "Gur xrl.".len();
    let note_bytes = 4 * 1024 * 1024 - (png + caption) - (png + caption / 2);
    let chunks = note_bytes.div_ceil(2048);
    let note = before["stage"]["blocks"][1]["note"]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    with_database(&root, context.workspace_id, async |connection| {
        sqlx::query("WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i+1 FROM n WHERE i<?) INSERT INTO note_revisions SELECT ?,?,?,i+1,substr(?,1,min(2048,?-(i-1)*2048)) FROM n")
   .bind(chunks as i64).bind(context.workspace_id.to_string()).bind(context.exploration_id.to_string()).bind(&note).bind("x".repeat(2048)).bind(note_bytes as i64).execute(&mut *connection).await.unwrap();
        sqlx::query("UPDATE notes SET current_revision=? WHERE id=?")
            .bind(chunks as i64 + 1)
            .bind(note)
            .execute(connection)
            .await
            .unwrap();
    });
    let mut manager = Manager::new(root.clone()).unwrap();
    command(
        &mut manager,
        json!({"kind":"open","workspace_id":context.workspace_id}),
    );
    let before = open(&mut manager, &context);
    assert_eq!(
        manager
            .handle(serde_json::from_value(copy(&context, &before, 0)).unwrap())
            .unwrap_err(),
        "storage_quota"
    );
    assert_eq!(open(&mut manager, &context), before);
    drop(manager);
    with_database(&root, context.workspace_id, async |connection| {
        let counts: (i64, i64) = sqlx::query_as(
            "SELECT (SELECT count(*) FROM assets),(SELECT count(*) FROM asset_origins)",
        )
        .fetch_one(connection)
        .await
        .unwrap();
        assert_eq!(counts, (2, 0));
    });
}
