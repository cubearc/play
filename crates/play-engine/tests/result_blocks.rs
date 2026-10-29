#[path = "support/graph.rs"]
mod graph;
use graph::configuration;
#[path = "support/native_schema.rs"]
mod native_schema;
use play_engine::workspace::{Context, Manager, RunRequest};
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
fn setup(manager: &mut Manager) -> (Context, Value) {
    let workspace = command(
        manager,
        json!({"kind":"create","key":Uuid::new_v4(),"title":"Result studies"}),
    );
    let created = command(
        manager,
        json!({"kind":"create_exploration","workspace_id":workspace["id"],"key":Uuid::new_v4(),"title":"Evidence"}),
    );
    let context:Context=serde_json::from_value(json!({"workspace_id":workspace["id"],"exploration_id":created["explorationId"],"stage_id":created["stageId"]})).unwrap();
    command(
        manager,
        json!({"kind":"add_text","context":context,"key":Uuid::new_v4(),"title":"Clue","text":"Gur xrl."}),
    );
    let result = view(manager, &context);
    (context, result)
}
fn request(context: &Context, trail: &Value, operation: &str, retain: bool) -> RunRequest {
    let version = cube_core::operations::require(operation)
        .unwrap()
        .contract()
        .unwrap()
        .operation_version;
    serde_json::from_value(json!({"context":context,"key":Uuid::new_v4(),"trail_id":trail["id"],"pipeline_id":trail["pipeline"]["id"],"pipeline_revision":trail["pipeline"]["revision"],"source_asset_id":trail["source"]["id"],"source_revision":trail["source"]["revision"],"operation":operation,"operation_version":version,"retain_output":retain})).unwrap()
}
fn complete(
    manager: &mut Manager,
    context: &Context,
    trail: &Value,
    retain: bool,
    output: Value,
) -> Value {
    let operation = if output.get("bytes").is_some() {
        "esolang.brainfuck.execute"
    } else {
        "cipher.rot13"
    };
    let selected = if operation == "esolang.brainfuck.execute" {
        let mut parameters =
            serde_json::to_value(cube_core::esolang::brainfuck::Limits::default()).unwrap();
        parameters["max_output_bytes"] = json!(16384);
        let pipeline = command(
            manager,
            json!({"kind":"create_pipeline","workspace_id":context.workspace_id,"key":Uuid::new_v4(),"title":"Brainfuck","definition":configuration(operation,2,json!(parameters))}),
        );
        command(
            manager,
            json!({"kind":"save_trail","context":context,"key":Uuid::new_v4(),"trail_id":trail["id"],"expected_revision":trail["revision"],"hypothesis":trail["hypothesis"],"pipeline":{"id":pipeline["pipelineId"],"revision":1}}),
        );
        let mut selected = trail.clone();
        selected["pipeline"]["id"] = pipeline["pipelineId"].clone();
        selected["pipeline"]["revision"] = json!(1);
        selected
    } else {
        trail.clone()
    };
    let prepared = manager
        .prepare_run(&request(context, &selected, operation, retain))
        .unwrap();
    manager.start_attempt(&prepared.identity).unwrap();
    manager
        .finish_run(
            &prepared.identity,
            json!({"status":"completed","output":output}),
        )
        .unwrap()
}
fn intent(context: &Context, state: &Value, run: &Value, port: &str) -> Value {
    json!({"kind":"pin_run_output","context":context,"key":Uuid::new_v4(),"expected_revision":state["stage"]["revision"],"run_id":run["id"],"attempt_id":run["attempts"][0]["id"],"port":port})
}
fn source(block: &Value) -> Value {
    json!({"kind":"asset","asset":{"asset_id":block["asset"]["id"],"revision":block["asset"]["revision"]},"caption":null})
}

#[test]
fn result_blocks_preserve_exact_attempt_origin_and_reuse_text_without_editing_evidence() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let (context, initial) = setup(&mut manager);
    let run = complete(
        &mut manager,
        &context,
        &initial["trails"][0],
        true,
        json!({"text":"The key."}),
    );
    let pin = intent(&context, &initial, &run, "output");
    let receipt = command(&mut manager, pin.clone());
    assert_eq!(command(&mut manager, pin.clone()), receipt);
    let saved = view(&mut manager, &context);
    let block = &saved["stage"]["blocks"][1];
    assert_eq!(
        block["asset"]["content"],
        json!({"kind":"text","text":"The key."})
    );
    let origin = &block["asset"]["origin"];
    assert_eq!(origin["kind"], "run_output");
    assert_eq!(origin["runId"], run["id"]);
    assert_eq!(origin["attemptId"], run["attempts"][0]["id"]);
    assert_eq!(origin["nodeId"], run["attempts"][0]["nodeId"]);
    assert_eq!(origin["port"], "output");
    assert_eq!(origin["semanticType"], "text");
    assert_eq!(origin["trailId"], run["trailId"]);
    let result_trail = saved["trails"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["source"]["id"] == block["asset"]["id"])
        .unwrap();
    let next = manager
        .prepare_run(&request(&context, result_trail, "cipher.rot13", true))
        .unwrap();
    assert_eq!(next.invocation.input, "The key.");
    manager.start_attempt(&next.identity).unwrap();
    manager
        .finish_run(
            &next.identity,
            json!({"status":"completed","output":{"text":"Gur xrl."}}),
        )
        .unwrap();
    let edit = json!({"kind":"revise_text","context":context,"key":Uuid::new_v4(),"block_id":block["id"],"asset_id":block["asset"]["id"],"expected_revision":1,"text":"rewrite"});
    assert_eq!(
        manager
            .handle(serde_json::from_value(edit).unwrap())
            .unwrap_err(),
        "result_read_only"
    );
    command(
        &mut manager,
        json!({"kind":"duplicate_block","context":context,"key":Uuid::new_v4(),"block_id":block["id"],"expected_revision":saved["stage"]["revision"],"source":source(block)}),
    );
    let copied = view(&mut manager, &context);
    let copy = &copied["stage"]["blocks"][2];
    assert_eq!(copy["asset"]["origin"]["kind"], "asset");
    assert_eq!(copy["asset"]["origin"]["assetId"], block["asset"]["id"]);
    command(
        &mut manager,
        json!({"kind":"revise_text","context":context,"key":Uuid::new_v4(),"block_id":copy["id"],"asset_id":copy["asset"]["id"],"expected_revision":1,"text":"An alternative"}),
    );
    let before = view(&mut manager, &context);
    drop(manager);
    let mut manager = Manager::new(root).unwrap();
    command(
        &mut manager,
        json!({"kind":"open","workspace_id":context.workspace_id}),
    );
    assert_eq!(view(&mut manager, &context), before);
    assert_eq!(
        manager
            .run_detail(
                &context,
                Uuid::parse_str(run["id"].as_str().unwrap()).unwrap()
            )
            .unwrap(),
        run
    );
    assert_eq!(command(&mut manager, pin), receipt);
}

#[test]
fn bytes_result_blocks_are_exact_files_and_do_not_coerce_to_text() {
    let tmp = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(tmp.path().join("private")).unwrap();
    let (context, initial) = setup(&mut manager);
    let clue = &initial["stage"]["blocks"][0];
    command(
        &mut manager,
        json!({"kind":"revise_text","context":context,"key":Uuid::new_v4(),"block_id":clue["id"],"asset_id":clue["asset"]["id"],"expected_revision":1,"text":"+."}),
    );
    let initial = view(&mut manager, &context);
    let run = complete(
        &mut manager,
        &context,
        &initial["trails"][0],
        true,
        json!({"bytes":[0,255,128,65]}),
    );
    let port = run["result"]["outputs"]
        .as_object()
        .unwrap()
        .keys()
        .next()
        .unwrap();
    command(&mut manager, intent(&context, &initial, &run, port));
    let saved = view(&mut manager, &context);
    let block = &saved["stage"]["blocks"][1];
    assert_eq!(block["asset"]["origin"]["semanticType"], "bytes");
    assert_eq!(block["asset"]["content"]["kind"], "file");
    assert_eq!(block["asset"]["content"]["byteLength"], 4);
    assert_eq!(
        block["asset"]["content"]["sha256"],
        cube_core::hashing::sha256(&[0, 255, 128, 65])
    );
    assert_eq!(block["asset"]["content"]["preview"]["text"], "00ff8041");
    let trail = saved["trails"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["source"]["id"] == block["asset"]["id"])
        .unwrap();
    assert_eq!(trail["pipeline"], Value::Null);
}

#[test]
fn unavailable_foreign_and_stale_output_intents_never_add_notebook_content() {
    let tmp = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(tmp.path().join("private")).unwrap();
    let (context, initial) = setup(&mut manager);
    let retained = complete(
        &mut manager,
        &context,
        &initial["trails"][0],
        true,
        json!({"text":"The key."}),
    );
    let hidden = complete(
        &mut manager,
        &context,
        &initial["trails"][0],
        false,
        json!({"text":"The key."}),
    );
    let active = manager
        .prepare_run(&request(
            &context,
            &initial["trails"][0],
            "cipher.rot13",
            true,
        ))
        .unwrap();
    manager.start_attempt(&active.identity).unwrap();
    let running = manager.run_detail(&context, active.identity.id).unwrap();
    let valid = intent(&context, &initial, &retained, "output");
    let mut stale = valid.clone();
    stale["expected_revision"] = json!(1);
    let mut foreign = valid.clone();
    foreign["context"]["exploration_id"] = json!(Uuid::new_v4());
    let mut wrong_attempt = valid.clone();
    wrong_attempt["attempt_id"] = json!(Uuid::new_v4());
    for (value, error) in [
        (
            intent(&context, &initial, &hidden, "output"),
            "result_not_available",
        ),
        (
            intent(&context, &initial, &running, "output"),
            "result_not_available",
        ),
        (
            intent(&context, &initial, &retained, "missing"),
            "not_found",
        ),
        (wrong_attempt, "not_found"),
        (foreign, "not_found"),
        (stale, "state_conflict"),
    ] {
        assert_eq!(
            manager
                .handle(serde_json::from_value(value).unwrap())
                .unwrap_err(),
            error
        );
        assert_eq!(view(&mut manager, &context), initial);
    }
    manager
        .finish_run(
            &active.identity,
            json!({"status":"failed","code":"operation_failed"}),
        )
        .unwrap();
    for index in 0..6 {
        command(
            &mut manager,
            json!({"kind":"add_text","context":context,"key":Uuid::new_v4(),"title":format!("Extra {index}"),"text":"kept"}),
        );
    }
    let full = view(&mut manager, &context);
    let pin = intent(&context, &full, &retained, "output");
    assert_eq!(
        manager
            .handle(serde_json::from_value(pin).unwrap())
            .unwrap_err(),
        "storage_quota"
    );
    assert_eq!(view(&mut manager, &context), full);
}

#[test]
fn result_identity_is_unique_across_fresh_intents_and_archive_never_reapplies_a_retry() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let (context, initial) = setup(&mut manager);
    let run = complete(
        &mut manager,
        &context,
        &initial["trails"][0],
        true,
        json!({"text":"The key."}),
    );
    let first = intent(&context, &initial, &run, "output");
    let receipt = command(&mut manager, first.clone());
    let saved = view(&mut manager, &context);
    let block = saved["stage"]["blocks"][1].clone();
    let again = command(&mut manager, intent(&context, &saved, &run, "output"));
    assert_eq!(again["blockId"], receipt["blockId"]);
    assert_eq!(view(&mut manager, &context), saved);
    command(
        &mut manager,
        json!({"kind":"archive_block","context":context,"key":Uuid::new_v4(),"block_id":block["id"],"expected_revision":saved["stage"]["revision"],"source":source(&block)}),
    );
    let archived = view(&mut manager, &context);
    assert_eq!(command(&mut manager, first), receipt);
    command(&mut manager, intent(&context, &archived, &run, "output"));
    assert_eq!(view(&mut manager, &context), archived);
    drop(manager);
    let database = root
        .join(context.workspace_id.to_string())
        .join("workspace.sqlite3");
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
            let mut db = SqliteConnection::connect_with(
                &SqliteConnectOptions::new()
                    .filename(database)
                    .foreign_keys(true),
            )
            .await
            .unwrap();
            let asset = block["asset"]["id"].as_str().unwrap();
            assert!(
                sqlx::query("INSERT INTO asset_revisions VALUES(?,?,?,2,?,?)")
                    .bind(context.workspace_id.to_string())
                    .bind(context.exploration_id.to_string())
                    .bind(asset)
                    .bind(b"replacement".as_slice())
                    .bind(cube_core::hashing::sha256(b"replacement"))
                    .execute(&mut db)
                    .await
                    .is_err()
            );
            for sql in [
                "UPDATE result_origins SET port='changed' WHERE asset_id=?",
                "DELETE FROM result_origins WHERE asset_id=?",
            ] {
                assert!(sqlx::query(sql).bind(asset).execute(&mut db).await.is_err());
            }
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
fn native_schema_five_upgrade_preserves_file_archive_and_run_before_saving_output() {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let (context, initial) = setup(&mut manager);
    let run = complete(
        &mut manager,
        &context,
        &initial["trails"][0],
        true,
        json!({"text":"The key."}),
    );
    command(
        &mut manager,
        json!({"kind":"add_file","context":context,"key":Uuid::new_v4(),"title":"data.bin","base64":STANDARD.encode([0,255,128])}),
    );
    let added = view(&mut manager, &context);
    let file = &added["stage"]["blocks"][1];
    command(
        &mut manager,
        json!({"kind":"archive_block","context":context,"key":Uuid::new_v4(),"block_id":file["id"],"expected_revision":added["stage"]["revision"],"source":source(file)}),
    );
    let before = view(&mut manager, &context);
    drop(manager);
    native_schema::snapshot(&root, context.workspace_id, 5);
    let mut manager = Manager::new(root).unwrap();
    command(
        &mut manager,
        json!({"kind":"open","workspace_id":context.workspace_id}),
    );
    assert_eq!(view(&mut manager, &context), before);
    assert_eq!(
        manager
            .run_detail(
                &context,
                Uuid::parse_str(run["id"].as_str().unwrap()).unwrap()
            )
            .unwrap(),
        run
    );
    command(&mut manager, intent(&context, &before, &run, "output"));
    let after = view(&mut manager, &context);
    assert_eq!(
        after["stage"]["archivedBlocks"],
        before["stage"]["archivedBlocks"]
    );
    assert_eq!(
        after["stage"]["blocks"][1]["asset"]["content"]["text"],
        "The key."
    );
}
