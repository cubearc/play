#[path = "support/native_schema.rs"]
mod native_schema;
use play_engine::workspace::{Context, Manager};
use serde_json::{Value, json};
use uuid::Uuid;
fn command(manager: &mut Manager, value: Value) -> Value {
    manager
        .handle(serde_json::from_value(value).unwrap())
        .unwrap()
}
fn view(manager: &mut Manager, context: &Context) -> Value {
    command(
        manager,
        json!({"kind":"open_exploration","workspace_id":context.workspace_id,"exploration_id":context.exploration_id,"stage_id":context.stage_id}),
    )
}
fn setup(manager: &mut Manager) -> Context {
    let workspace = command(
        manager,
        json!({"kind":"create","key":Uuid::new_v4(),"title":"Note studies"}),
    );
    let created = command(
        manager,
        json!({"kind":"create_exploration","workspace_id":workspace["id"],"key":Uuid::new_v4(),"title":"Local observations"}),
    );
    serde_json::from_value(json!({"workspace_id":workspace["id"],"exploration_id":created["explorationId"],"stage_id":created["stageId"]})).unwrap()
}
fn add(context: &Context, title: &str, text: &str) -> Value {
    json!({"kind":"add_annotation","context":context,"key":Uuid::new_v4(),"title":title,"text":text})
}
fn source(block: &Value) -> Value {
    json!({"kind":"note","note_id":block["note"]["id"],"revision":block["note"]["revision"]})
}
fn action(kind: &str, context: &Context, state: &Value, block: &Value) -> Value {
    json!({"kind":kind,"context":context,"key":Uuid::new_v4(),"block_id":block["id"],"expected_revision":state["stage"]["revision"],"source":source(block)})
}
#[test]
fn annotations_share_note_revisions_but_keep_independent_content_and_copy_lineage() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let context = setup(&mut manager);
    let initial = view(&mut manager, &context);
    let intent = add(&context, "Observation", "First clue is ambiguous.");
    let receipt = command(&mut manager, intent.clone());
    assert_eq!(command(&mut manager, intent), receipt);
    command(
        &mut manager,
        add(&context, "Alternative", "Try the caption first."),
    );
    let state = view(&mut manager, &context);
    assert_eq!(
        state["stage"]["blocks"][2]["note"]["id"],
        initial["stage"]["blocks"][0]["note"]["id"]
    );
    assert_eq!(state["stage"]["blocks"][2]["note"]["kind"], "reasoning");
    let original = state["stage"]["blocks"][0].clone();
    assert_eq!(original["note"]["kind"], "annotation");
    assert_eq!(original["note"]["title"], "Observation");
    assert!(state["trails"].as_array().unwrap().is_empty());
    command(
        &mut manager,
        json!({"kind":"save_note","context":context,"key":Uuid::new_v4(),"note_id":original["note"]["id"],"expected_revision":1,"text":"Updated observation."}),
    );
    let edited = view(&mut manager, &context);
    assert_eq!(edited["stage"]["blocks"][1], state["stage"]["blocks"][1]);
    assert_eq!(edited["stage"]["blocks"][2], state["stage"]["blocks"][2]);
    let note = &edited["stage"]["blocks"][0];
    assert_eq!(note["note"]["revision"], 2);
    command(
        &mut manager,
        action("duplicate_block", &context, &edited, note),
    );
    let copied = view(&mut manager, &context);
    let copy = copied["stage"]["blocks"][1].clone();
    assert_ne!(copy["note"]["id"], note["note"]["id"]);
    assert_eq!(copy["note"]["revision"], 1);
    assert_eq!(copy["note"]["text"], "Updated observation.");
    assert_eq!(
        copy["note"]["origin"],
        json!({"noteId":note["note"]["id"],"revision":2,"title":"Observation"})
    );
    command(
        &mut manager,
        action("archive_block", &context, &copied, note),
    );
    let archived = view(&mut manager, &context);
    assert_eq!(archived["stage"]["archivedBlocks"][0]["block"], *note);
    let edit = json!({"kind":"save_note","context":context,"key":Uuid::new_v4(),"note_id":note["note"]["id"],"expected_revision":2,"text":"no"});
    assert_eq!(
        manager
            .handle(serde_json::from_value(edit).unwrap())
            .unwrap_err(),
        "block_archived"
    );
    command(
        &mut manager,
        action("restore_block", &context, &archived, note),
    );
    let restored = view(&mut manager, &context);
    assert_eq!(restored["stage"]["blocks"][2], *note);
    assert_eq!(
        restored["stage"]["blocks"][1]["note"]["title"],
        "Alternative"
    );
    drop(manager);
    let mut manager = Manager::new(root).unwrap();
    command(
        &mut manager,
        json!({"kind":"open","workspace_id":context.workspace_id}),
    );
    assert_eq!(view(&mut manager, &context), restored);
}
#[test]
fn annotation_snapshots_reject_stale_content_and_reasoning_remains_protected() {
    let tmp = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(tmp.path().join("private")).unwrap();
    let context = setup(&mut manager);
    command(&mut manager, add(&context, "Observation", "Before"));
    let before = view(&mut manager, &context);
    let note = before["stage"]["blocks"][0].clone();
    command(
        &mut manager,
        json!({"kind":"save_note","context":context,"key":Uuid::new_v4(),"note_id":note["note"]["id"],"expected_revision":1,"text":"After"}),
    );
    let after = view(&mut manager, &context);
    for kind in ["duplicate_block", "archive_block"] {
        assert_eq!(
            manager
                .handle(serde_json::from_value(action(kind, &context, &after, &note)).unwrap())
                .unwrap_err(),
            "state_conflict"
        );
        assert_eq!(view(&mut manager, &context), after);
    }
    let reasoning = &after["stage"]["blocks"][1];
    for kind in ["duplicate_block", "archive_block"] {
        assert_eq!(
            manager
                .handle(serde_json::from_value(action(kind, &context, &after, reasoning)).unwrap())
                .unwrap_err(),
            "invalid_block_type"
        );
    }
    let mut foreign = context.clone();
    foreign.exploration_id = Uuid::new_v4();
    assert_eq!(
        manager
            .handle(serde_json::from_value(add(&foreign, "Foreign", "hidden")).unwrap())
            .unwrap_err(),
        "not_found"
    );
    assert_eq!(
        manager
            .handle(serde_json::from_value(add(&context, "Large", &"á".repeat(1025))).unwrap())
            .unwrap_err(),
        "input_limit"
    );
    assert_eq!(view(&mut manager, &context), after);
}
#[test]
fn trailing_annotations_are_not_treated_as_the_protected_reasoning_note() {
    let tmp = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(tmp.path().join("private")).unwrap();
    let context = setup(&mut manager);
    command(&mut manager, add(&context, "First", "Keep order"));
    let initial = view(&mut manager, &context);
    let note = initial["stage"]["blocks"][0].clone();
    let reasoning = initial["stage"]["blocks"][1].clone();
    command(
        &mut manager,
        json!({"kind":"reorder_blocks","context":context,"key":Uuid::new_v4(),"expected_revision":initial["stage"]["revision"],"block_ids":[reasoning["id"],note["id"]]}),
    );
    command(
        &mut manager,
        add(&context, "Last", "Append after an annotation"),
    );
    let state = view(&mut manager, &context);
    assert_eq!(state["stage"]["blocks"][0], reasoning);
    assert_eq!(state["stage"]["blocks"][1], note);
    assert_eq!(state["stage"]["blocks"][2]["note"]["title"], "Last");
}

#[test]
fn annotation_capacity_and_database_guards_preserve_reasoning_and_note_history() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let context = setup(&mut manager);
    for index in 0..7 {
        command(
            &mut manager,
            add(&context, &format!("Note {index}"), "Original"),
        );
    }
    let full = view(&mut manager, &context);
    assert_eq!(
        manager
            .handle(serde_json::from_value(add(&context, "Overflow", "no")).unwrap())
            .unwrap_err(),
        "storage_quota"
    );
    assert_eq!(view(&mut manager, &context), full);
    let note = &full["stage"]["blocks"][0];
    command(&mut manager, action("archive_block", &context, &full, note));
    command(&mut manager, add(&context, "Replacement", "Kept"));
    let replaced = view(&mut manager, &context);
    assert_eq!(
        manager
            .handle(
                serde_json::from_value(action("restore_block", &context, &replaced, note)).unwrap()
            )
            .unwrap_err(),
        "storage_quota"
    );
    assert_eq!(view(&mut manager, &context), replaced);
    let reasoning = replaced["stage"]["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["note"]["kind"] == "reasoning")
        .unwrap()
        .clone();
    drop(manager);
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
            let mut db = SqliteConnection::connect_with(
                &SqliteConnectOptions::new()
                    .filename(
                        root.join(context.workspace_id.to_string())
                            .join("workspace.sqlite3"),
                    )
                    .foreign_keys(true),
            )
            .await
            .unwrap();
            assert!(
                sqlx::query("UPDATE blocks SET archived_revision=1 WHERE id=?")
                    .bind(reasoning["id"].as_str().unwrap())
                    .execute(&mut db)
                    .await
                    .is_err()
            );
            assert!(
                sqlx::query("UPDATE notes SET kind='annotation' WHERE id=?")
                    .bind(reasoning["note"]["id"].as_str().unwrap())
                    .execute(&mut db)
                    .await
                    .is_err()
            );
            assert!(
                sqlx::query("INSERT INTO notes VALUES(?,?,?,?,1,'reasoning','Other',NULL,NULL)")
                    .bind(context.workspace_id.to_string())
                    .bind(context.exploration_id.to_string())
                    .bind(context.stage_id.to_string())
                    .bind(Uuid::new_v4().to_string())
                    .execute(&mut db)
                    .await
                    .is_err()
            );
            assert!(
                sqlx::query("DELETE FROM note_revisions WHERE note_id=?")
                    .bind(note["note"]["id"].as_str().unwrap())
                    .execute(&mut db)
                    .await
                    .is_err()
            );
            assert!(
                sqlx::query("PRAGMA foreign_key_check")
                    .fetch_optional(&mut db)
                    .await
                    .unwrap()
                    .is_none()
            );
            db.close().await.unwrap();
        });
}

#[test]
fn native_schema_six_upgrade_preserves_saved_results_and_reasoning_before_annotation_creation() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let context = setup(&mut manager);
    command(
        &mut manager,
        json!({"kind":"add_text","context":context,"key":Uuid::new_v4(),"title":"Clue","text":"Gur xrl."}),
    );
    let state = view(&mut manager, &context);
    let trail = &state["trails"][0];
    let request=serde_json::from_value(json!({"context":context,"key":Uuid::new_v4(),"trail_id":trail["id"],"trail_revision":trail["revision"],"target_node_id":trail["pipeline"]["outputNodeId"],"pipeline_id":trail["pipeline"]["id"],"pipeline_revision":1,"source_asset_id":trail["source"]["id"],"source_revision":1,"operation":"cipher.rot13","operation_version":1,"retain_output":true})).unwrap();
    let prepared = manager.prepare_run(&request).unwrap();
    manager.start_attempt(&prepared.identity).unwrap();
    let run = manager
        .finish_run(
            &prepared.identity,
            json!({"status":"completed","output":{"text":"The key."}}),
        )
        .unwrap();
    command(
        &mut manager,
        json!({"kind":"pin_run_output","context":context,"key":Uuid::new_v4(),"expected_revision":state["stage"]["revision"],"run_id":run["id"],"attempt_id":run["attempts"][0]["id"],"port":"output"}),
    );
    let before = view(&mut manager, &context);
    drop(manager);
    native_schema::snapshot(&root, context.workspace_id, 6);
    let mut manager = Manager::new(root).unwrap();
    command(
        &mut manager,
        json!({"kind":"open","workspace_id":context.workspace_id}),
    );
    assert_eq!(view(&mut manager, &context), before);
    assert_eq!(
        manager.run_detail(&context, prepared.identity.id).unwrap(),
        run
    );
    command(
        &mut manager,
        add(&context, "Observation", "The earlier result remains exact."),
    );
    let after = view(&mut manager, &context);
    assert_eq!(after["stage"]["blocks"][1], before["stage"]["blocks"][1]);
    assert_eq!(after["stage"]["blocks"][2]["note"]["kind"], "annotation");
}
