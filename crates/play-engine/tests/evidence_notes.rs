use play_engine::workspace::{Context, Manager};
use serde_json::{Value, json};
use uuid::Uuid;
fn call(m: &mut Manager, v: Value) -> Value {
    m.handle(serde_json::from_value(v).unwrap()).unwrap()
}
fn view(m: &mut Manager, c: &Context) -> Value {
    call(
        m,
        json!({"kind":"open_exploration","workspace_id":c.workspace_id,"exploration_id":c.exploration_id,"stage_id":c.stage_id}),
    )
}
fn setup(m: &mut Manager) -> (Context, Value) {
    let w = call(
        m,
        json!({"kind":"create","key":Uuid::new_v4(),"title":"Evidence notes"}),
    );
    let e = call(
        m,
        json!({"kind":"create_exploration","workspace_id":w["id"],"key":Uuid::new_v4(),"title":"Clues"}),
    );
    let c = serde_json::from_value(
        json!({"workspace_id":w["id"],"exploration_id":e["explorationId"],"stage_id":e["stageId"]}),
    )
    .unwrap();
    call(
        m,
        json!({"kind":"add_text","context":c,"key":Uuid::new_v4(),"title":"Clue","text":"Gur xrl."}),
    );
    let v = view(m, &c);
    (c, v)
}
fn annotate(c: &Context, target: Value) -> Value {
    json!({"kind":"annotate_evidence","context":c,"key":Uuid::new_v4(),"title":"Observation","text":"This clue matters.","target":target})
}
fn note<'a>(v: &'a Value, id: &Value) -> &'a Value {
    &v["stage"]["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["id"] == *id)
        .unwrap()["note"]
}
fn inspect(m: &mut Manager, c: &Context, id: &Value) -> Value {
    call(
        m,
        json!({"kind":"inspect_note_target","context":c,"note_id":id}),
    )
}
#[test]
fn asset_annotation_keeps_exact_revision_during_edit_copy_archive_and_restart() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("private");
    let mut m = Manager::new(root.clone()).unwrap();
    let (c, v) = setup(&mut m);
    let b = v["stage"]["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["kind"] == "asset")
        .unwrap();
    let target = json!({"kind":"asset","asset_id":b["asset"]["id"],"revision":1});
    let intent = annotate(&c, target.clone());
    let created = call(&mut m, intent.clone());
    assert_eq!(call(&mut m, intent), created);
    let current = view(&mut m, &c);
    let n = note(&current, &created["blockId"]);
    assert_eq!(n["target"]["reference"], target);
    let note_id = n["id"].clone();
    call(
        &mut m,
        json!({"kind":"revise_text","context":c,"key":Uuid::new_v4(),"block_id":b["id"],"asset_id":b["asset"]["id"],"expected_revision":1,"text":"New working value"}),
    );
    call(
        &mut m,
        json!({"kind":"save_note","context":c,"key":Uuid::new_v4(),"note_id":note_id,"expected_revision":1,"text":"A refined observation"}),
    );
    assert_eq!(
        inspect(&mut m, &c, &note_id)["evidence"]["asset"]["content"]["text"],
        "Gur xrl."
    );
    let current = view(&mut m, &c);
    let copied = call(
        &mut m,
        json!({"kind":"duplicate_block","context":c,"key":Uuid::new_v4(),"block_id":created["blockId"],"expected_revision":current["stage"]["revision"],"source":{"kind":"note","note_id":note_id,"revision":2}}),
    );
    let after = view(&mut m, &c);
    assert_eq!(
        note(&after, &copied["blockId"])["target"],
        note(&after, &created["blockId"])["target"]
    );
    let current_asset = after["stage"]["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|x| x["id"] == b["id"])
        .unwrap();
    call(
        &mut m,
        json!({"kind":"archive_block","context":c,"key":Uuid::new_v4(),"block_id":b["id"],"expected_revision":after["stage"]["revision"],"source":{"kind":"asset","asset":{"asset_id":b["asset"]["id"],"revision":current_asset["asset"]["revision"]},"caption":null}}),
    );
    drop(m);
    let mut m = Manager::new(root).unwrap();
    call(&mut m, json!({"kind":"open","workspace_id":c.workspace_id}));
    assert_eq!(
        inspect(&mut m, &c, &note_id)["evidence"]["asset"]["content"]["text"],
        "Gur xrl."
    );
}

fn request(c: &Context, trail: &Value, retain: bool) -> play_engine::workspace::RunRequest {
    serde_json::from_value(json!({"context":c,"key":Uuid::new_v4(),"trail_id":trail["id"],"trail_revision":trail["revision"],"target_node_id":trail["pipeline"]["outputNodeId"],"pipeline_id":trail["pipeline"]["id"],"pipeline_revision":trail["pipeline"]["revision"],"source_asset_id":trail["source"]["id"],"source_revision":trail["source"]["revision"],"operation":"cipher.rot13","operation_version":1,"retain_output":retain})).unwrap()
}
fn linked(m: &mut Manager, c: &Context, target: Value) -> Value {
    let created = call(m, annotate(c, target));
    let v = view(m, c);
    inspect(m, c, &note(&v, &created["blockId"])["id"])
}
#[test]
fn notes_distinguish_finished_attempts_retained_outputs_and_unavailable_evidence() {
    let temp = tempfile::tempdir().unwrap();
    let mut m = Manager::new(temp.path().join("private")).unwrap();
    let (c, v) = setup(&mut m);
    for (retain, result) in [
        (
            true,
            json!({"status":"completed","output":{"text":"The key."}}),
        ),
        (
            false,
            json!({"status":"completed","output":{"text":"Ephemeral"}}),
        ),
        (true, json!({"status":"failed","code":"invalid_input"})),
    ] {
        let prepared = m
            .prepare_run(&request(&c, &v["trails"][0], retain))
            .unwrap();
        m.start_attempt(&prepared.identity).unwrap();
        let running = call(
            &mut m,
            json!({"kind":"run_detail","context":c,"run_id":prepared.identity.id}),
        );
        let target = json!({"kind":"attempt","run_id":running["id"],"attempt_id":running["attempts"][0]["id"]});
        assert_eq!(
            m.handle(serde_json::from_value(annotate(&c, target.clone())).unwrap())
                .unwrap_err(),
            "attempt_not_finished"
        );
        m.finish_attempt(&prepared.identity, result.clone())
            .unwrap();
        m.finish_run(&prepared.identity, result.clone()).unwrap();
        let detail = linked(&mut m, &c, target.clone());
        assert_eq!(detail["evidence"]["status"], result["status"]);
        assert_eq!(detail["evidence"]["code"], result["code"]);
        let mut output = target.clone();
        output["kind"] = json!("run_output");
        output["port"] = json!("output");
        let detail = linked(&mut m, &c, output.clone());
        if retain && result["status"] == "completed" {
            assert_eq!(detail["evidence"]["value"], "The key.");
        } else {
            assert_eq!(detail["evidence"]["kind"], "unavailable");
            assert_eq!(detail["evidence"]["code"], "result_not_available");
        }
        output["port"] = json!("missing");
        assert_eq!(
            m.handle(serde_json::from_value(annotate(&c, output)).unwrap())
                .unwrap_err(),
            "not_found"
        );
    }
}
#[test]
fn targets_cannot_cross_stages_or_explorations_and_rejected_writes_are_atomic() {
    let temp = tempfile::tempdir().unwrap();
    let mut m = Manager::new(temp.path().join("private")).unwrap();
    let (c, v) = setup(&mut m);
    let block = &v["stage"]["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["kind"] == "asset")
        .unwrap();
    let target = json!({"kind":"block","block_id":block["id"]});
    assert_eq!(
        linked(&mut m, &c, target.clone())["evidence"]["title"],
        "Clue"
    );
    let stage = call(
        &mut m,
        json!({"kind":"create_stage","workspace_id":c.workspace_id,"exploration_id":c.exploration_id,"key":Uuid::new_v4(),"title":"Other stage"}),
    );
    let stage_context:Context=serde_json::from_value(json!({"workspace_id":c.workspace_id,"exploration_id":c.exploration_id,"stage_id":stage["stageId"]})).unwrap();
    let other = call(
        &mut m,
        json!({"kind":"create_exploration","workspace_id":c.workspace_id,"key":Uuid::new_v4(),"title":"Other exploration"}),
    );
    let exploration_context:Context=serde_json::from_value(json!({"workspace_id":c.workspace_id,"exploration_id":other["explorationId"],"stage_id":other["stageId"]})).unwrap();
    for foreign in [stage_context, exploration_context] {
        let before = view(&mut m, &foreign);
        for target in [
            target.clone(),
            json!({"kind":"asset","asset_id":block["asset"]["id"],"revision":1}),
        ] {
            assert_eq!(
                m.handle(serde_json::from_value(annotate(&foreign, target)).unwrap())
                    .unwrap_err(),
                "not_found"
            );
        }
        assert_eq!(view(&mut m, &foreign), before);
    }
    let before = view(&mut m, &c);
    assert_eq!(
        m.handle(
            serde_json::from_value(annotate(
                &c,
                json!({"kind":"asset","asset_id":block["asset"]["id"],"revision":99})
            ))
            .unwrap()
        )
        .unwrap_err(),
        "not_found"
    );
    assert_eq!(view(&mut m, &c), before);
}

#[test]
fn native_upgrade_preserves_notes_and_links_are_immutable() {
    use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("private");
    let mut m = Manager::new(root.clone()).unwrap();
    let (c, v) = setup(&mut m);
    call(
        &mut m,
        json!({"kind":"add_annotation","context":c,"key":Uuid::new_v4(),"title":"Prior note","text":"Already saved"}),
    );
    let before = view(&mut m, &c);
    drop(m);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let path = root
        .join(c.workspace_id.to_string())
        .join("workspace.sqlite3");
    runtime.block_on(async {
        let mut db = SqliteConnection::connect_with(
            &SqliteConnectOptions::new()
                .filename(&path)
                .foreign_keys(false),
        )
        .await
        .unwrap();
        sqlx::query("DROP TABLE note_targets")
            .execute(&mut db)
            .await
            .unwrap();
        let schema = include_str!("../src/workspace/solutions.sql");
        sqlx::raw_sql(&schema[schema.find("CREATE TABLE workspace_meta_next").unwrap()..])
            .execute(&mut db)
            .await
            .unwrap();
        db.close().await.unwrap();
    });
    let mut m = Manager::new(root.clone()).unwrap();
    call(&mut m, json!({"kind":"open","workspace_id":c.workspace_id}));
    assert_eq!(view(&mut m, &c), before);
    linked(
        &mut m,
        &c,
        json!({"kind":"asset","asset_id":v["trails"][0]["source"]["id"],"revision":1}),
    );
    drop(m);
    runtime.block_on(async {
        let mut db = SqliteConnection::connect_with(
            &SqliteConnectOptions::new()
                .filename(&path)
                .foreign_keys(true),
        )
        .await
        .unwrap();
        for statement in [
            "UPDATE note_targets SET asset_revision=2",
            "DELETE FROM note_targets",
            "DELETE FROM asset_revisions",
        ] {
            assert!(sqlx::query(statement).execute(&mut db).await.is_err());
        }
        db.close().await.unwrap();
    });
}
