#[path = "support/graph.rs"]
mod graph;
use graph::configuration;
#[path = "support/native_schema.rs"]
mod native_schema;
use play_engine::workspace::{Command, Context, Manager, RunRequest};
use serde_json::{Value, json};
use uuid::Uuid;
fn command(manager: &mut Manager, value: Value) -> Value {
    manager
        .handle(serde_json::from_value(value).unwrap())
        .unwrap()
}
fn setup(manager: &mut Manager) -> (RunRequest, Value) {
    let workspace = command(
        manager,
        json!({"kind":"create","key":Uuid::new_v4(),"title":"Run notebook"}),
    );
    let created = command(
        manager,
        json!({"kind":"create_exploration","workspace_id":workspace["id"],"key":Uuid::new_v4(),"title":"First"}),
    );
    let context = json!({"workspace_id":workspace["id"],"exploration_id":created["explorationId"],"stage_id":created["stageId"]});
    command(
        manager,
        json!({"kind":"add_text","context":context,"key":Uuid::new_v4(),"title":"Clue","text":"Gur xrl."}),
    );
    let view = command(
        manager,
        json!({"kind":"open_exploration","workspace_id":workspace["id"],"exploration_id":created["explorationId"],"stage_id":created["stageId"]}),
    );
    let trail = &view["trails"][0];
    let request:RunRequest=serde_json::from_value(json!({"context":context,"key":Uuid::new_v4(),"trail_id":trail["id"],"pipeline_id":trail["pipeline"]["id"],"pipeline_revision":1,"source_asset_id":trail["source"]["id"],"source_revision":trail["source"]["revision"],"operation":"cipher.rot13","operation_version":1,"retain_output":true})).unwrap();
    (request, view)
}
#[test]
fn a_run_pins_owned_inputs_contract_and_named_outputs_then_reopens_immutable_evidence() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let (request, view) = setup(&mut manager);
    let prepared = manager.prepare_run(&request).unwrap();
    manager.start_attempt(&prepared.identity).unwrap();
    let completed=manager.finish_run(&prepared.identity,json!({"status":"completed","output":{"text":cube_core::ciphers::rot13(&prepared.invocation.input)}})).unwrap();
    assert_eq!(completed["result"]["outputs"]["output"], "The key.");
    assert_eq!(
        completed["plan"]["pipeline"]["id"],
        request.pipeline_id.to_string()
    );
    assert_eq!(completed["plan"]["contract"]["operation"], "cipher.rot13");
    assert_eq!(
        completed["plan"]["inputs"]["source"]["assetId"],
        view["stage"]["blocks"][0]["asset"]["id"]
    );
    assert_eq!(completed["attempts"].as_array().unwrap().len(), 1);
    assert_eq!(manager.existing_run(&request).unwrap().unwrap(), completed);
    let mut changed = request.clone();
    changed.retain_output = false;
    assert_eq!(
        manager.existing_run(&changed).unwrap_err(),
        "idempotency_conflict"
    );
    assert_eq!(
        manager
            .finish_run(
                &prepared.identity,
                json!({"status":"cancelled","code":"late_cancel"})
            )
            .unwrap_err(),
        "state_conflict"
    );
    drop(manager);
    let mut manager = Manager::new(root).unwrap();
    manager
        .handle(Command::Open {
            workspace_id: request.context.workspace_id,
        })
        .unwrap();
    assert_eq!(
        manager
            .run_detail(&request.context, prepared.identity.id)
            .unwrap(),
        completed
    );
    let created = command(
        &mut manager,
        json!({"kind":"create_exploration","workspace_id":request.context.workspace_id,"key":Uuid::new_v4(),"title":"Second"}),
    );
    let other = Context {
        workspace_id: request.context.workspace_id,
        exploration_id: Uuid::parse_str(created["explorationId"].as_str().unwrap()).unwrap(),
        stage_id: Uuid::parse_str(created["stageId"].as_str().unwrap()).unwrap(),
    };
    assert_eq!(
        manager
            .run_detail(&other, prepared.identity.id)
            .unwrap_err(),
        "not_found"
    );
    assert_eq!(
        manager
            .handle(Command::ListRuns {
                context: other,
                trail_id: request.trail_id,
                before: None
            })
            .unwrap_err(),
        "not_found"
    );
}
#[test]
fn retention_is_explicit_and_a_dead_owner_recovers_only_unfinished_runs() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let (mut request, _) = setup(&mut manager);
    request.retain_output = false;
    let first = manager.prepare_run(&request).unwrap();
    manager.start_attempt(&first.identity).unwrap();
    let done = manager
        .finish_run(
            &first.identity,
            json!({"status":"completed","output":{"text":"The key."}}),
        )
        .unwrap();
    assert_eq!(done["result"]["replayGap"], "output_not_retained");
    assert!(done["result"].get("outputs").is_none());
    request.key = Uuid::new_v4();
    let orphan = manager.prepare_run(&request).unwrap();
    manager.start_attempt(&orphan.identity).unwrap();
    drop(manager);
    let mut manager = Manager::new(root).unwrap();
    manager
        .handle(Command::Open {
            workspace_id: request.context.workspace_id,
        })
        .unwrap();
    let recovered = manager
        .run_detail(&request.context, orphan.identity.id)
        .unwrap();
    assert_eq!(recovered["status"], "interrupted");
    assert_eq!(recovered["attempts"][0]["status"], "interrupted");
    assert_eq!(recovered["result"]["code"], "owner_stopped");
    assert_eq!(
        manager
            .run_detail(&request.context, first.identity.id)
            .unwrap(),
        done
    );
}
#[test]
fn explicitly_saved_pipeline_freezes_a_reusable_definition_and_real_bytes_contract() {
    let tmp = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(tmp.path().join("private")).unwrap();
    let (mut request, _) = setup(&mut manager);
    request.operation = "esolang.brainfuck.execute".into();
    request.operation_version = 2;
    let mut parameters =
        serde_json::to_value(cube_core::esolang::brainfuck::Limits::default()).unwrap();
    parameters["max_output_bytes"] = json!(16384);
    let pipeline = command(
        &mut manager,
        json!({"kind":"create_pipeline","workspace_id":request.context.workspace_id,"key":Uuid::new_v4(),"title":"Brainfuck","definition":configuration(&request.operation,2,json!(parameters))}),
    );
    command(
        &mut manager,
        json!({"kind":"save_trail","context":request.context,"key":Uuid::new_v4(),"trail_id":request.trail_id,"expected_revision":1,"hypothesis":"Read the program output","pipeline":{"id":pipeline["pipelineId"],"revision":1}}),
    );
    request.pipeline_id = serde_json::from_value(pipeline["pipelineId"].clone()).unwrap();
    let prepared = manager.prepare_run(&request).unwrap();
    manager.start_attempt(&prepared.identity).unwrap();
    let run = manager
        .finish_run(
            &prepared.identity,
            json!({"status":"completed","output":{"bytes":[65,0,255]}}),
        )
        .unwrap();
    assert_eq!(run["pipelineId"], request.pipeline_id.to_string());
    assert_eq!(run["plan"]["contract"]["inputs"][0]["name"], "source");
    assert_eq!(
        run["plan"]["pipeline"]["definition"]["nodes"][0]["inputs"]["input"]["value"],
        json!([])
    );
    assert_eq!(run["result"]["outputs"]["output"]["base64"], "QQD/");
    request.key = Uuid::new_v4();
    let second = manager.prepare_run(&request).unwrap();
    let next = manager
        .run_detail(&request.context, second.identity.id)
        .unwrap();
    assert_eq!(next["pipelineId"], run["pipelineId"]);
    assert_eq!(next["attempts"], json!([]));
}

fn native_baseline_fixture(root: &std::path::Path, workspace: Uuid) {
    native_schema::snapshot(root, workspace, 1);
}

#[test]
fn recognized_native_baseline_upgrades_atomically_without_reassigning_content() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let (request, before) = setup(&mut manager);
    drop(manager);
    native_baseline_fixture(&root, request.context.workspace_id);
    let marker = root
        .join(request.context.workspace_id.to_string())
        .join("workspace.json");
    let marker_before = std::fs::read(&marker).unwrap();
    let mut manager = Manager::new(root.clone()).unwrap();
    manager
        .handle(Command::Open {
            workspace_id: request.context.workspace_id,
        })
        .unwrap();
    let after = command(
        &mut manager,
        json!({"kind":"open_exploration","workspace_id":request.context.workspace_id,"exploration_id":request.context.exploration_id,"stage_id":request.context.stage_id}),
    );
    assert_eq!(after, before);
    assert_eq!(std::fs::read(marker).unwrap(), marker_before);
    let prepared = manager.prepare_run(&request).unwrap();
    manager.start_attempt(&prepared.identity).unwrap();
    assert_eq!(
        manager
            .finish_run(
                &prepared.identity,
                json!({"status":"completed","output":{"text":"The key."}})
            )
            .unwrap()["status"],
        "completed"
    );
    let bytes = std::fs::read(
        root.join(request.context.workspace_id.to_string())
            .join("workspace.sqlite3"),
    )
    .unwrap();
    assert_eq!(
        u32::from_be_bytes(bytes[60..64].try_into().unwrap()),
        play_engine::protocol::contract().schema_version
    );
}

#[test]
fn pagination_and_retention_quota_preserve_all_completed_evidence() {
    let tmp = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(tmp.path().join("private")).unwrap();
    let (mut request, _) = setup(&mut manager);
    for _ in 0..play_engine::protocol::contract().exploration_run_limit {
        request.key = Uuid::new_v4();
        let prepared = manager.prepare_run(&request).unwrap();
        manager.start_attempt(&prepared.identity).unwrap();
        manager
            .finish_run(
                &prepared.identity,
                json!({"status":"completed","output":{"text":"The key."}}),
            )
            .unwrap();
    }
    request.key = Uuid::new_v4();
    assert!(matches!(
        manager.prepare_run(&request),
        Err("storage_quota")
    ));
    let mut cursor = None;
    let mut ids = std::collections::HashSet::new();
    loop {
        let page = manager
            .handle(Command::ListRuns {
                context: request.context.clone(),
                trail_id: request.trail_id,
                before: cursor,
            })
            .unwrap();
        assert!(page["runs"].as_array().unwrap().len() <= 8);
        for run in page["runs"].as_array().unwrap() {
            assert!(ids.insert(run["id"].as_str().unwrap().to_string()));
            assert_eq!(run["status"], "completed");
        }
        cursor = page["before"].as_i64();
        if cursor.is_none() {
            break;
        }
    }
    assert_eq!(
        ids.len(),
        play_engine::protocol::contract().exploration_run_limit
    );
}

#[test]
fn sqlite_guards_frozen_run_and_node_evidence_even_during_execution() {
    use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let (request, _) = setup(&mut manager);
    let prepared = manager.prepare_run(&request).unwrap();
    manager.start_attempt(&prepared.identity).unwrap();
    let path = root
        .join(request.context.workspace_id.to_string())
        .join("workspace.sqlite3");
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let mut db = SqliteConnection::connect_with(
            &SqliteConnectOptions::new()
                .filename(&path)
                .foreign_keys(true),
        )
        .await
        .unwrap();
        assert!(
            sqlx::query("UPDATE pipeline_runs SET plan='{}'")
                .execute(&mut db)
                .await
                .is_err()
        );
        assert!(
            sqlx::query("UPDATE node_attempts SET node_id='other'")
                .execute(&mut db)
                .await
                .is_err()
        );
        assert!(
            sqlx::query("DELETE FROM run_inputs")
                .execute(&mut db)
                .await
                .is_err()
        );
        db.close().await.unwrap();
    });
    let completed = manager
        .finish_run(
            &prepared.identity,
            json!({"status":"completed","output":{"text":"The key."}}),
        )
        .unwrap();
    drop(manager);
    runtime.block_on(async {
        let mut db = SqliteConnection::connect_with(
            &SqliteConnectOptions::new()
                .filename(&path)
                .foreign_keys(true),
        )
        .await
        .unwrap();
        for sql in [
            "UPDATE pipeline_runs SET result='{}'",
            "DELETE FROM pipeline_runs",
            "UPDATE node_attempts SET status='failed'",
            "DELETE FROM node_attempts",
        ] {
            assert!(sqlx::query(sql).execute(&mut db).await.is_err());
        }
        db.close().await.unwrap();
    });
    let mut manager = Manager::new(root).unwrap();
    manager
        .handle(Command::Open {
            workspace_id: request.context.workspace_id,
        })
        .unwrap();
    assert_eq!(
        manager
            .run_detail(&request.context, prepared.identity.id)
            .unwrap(),
        completed
    );
}

fn private_file(path: &std::path::Path, bytes: &[u8]) {
    use std::io::Write;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path).unwrap().write_all(bytes).unwrap();
}
#[test]
fn an_interrupted_native_upgrade_discards_only_its_marked_unpublished_copy() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let (request, before) = setup(&mut manager);
    drop(manager);
    native_baseline_fixture(&root, request.context.workspace_id);
    let directory = root.join(request.context.workspace_id.to_string());
    let temporary = Uuid::new_v4();
    let copy = directory.join(format!(".upgrade-{temporary}.sqlite3"));
    let marker = directory.join("upgrade.json");
    private_file(&copy, b"interrupted private upgrade");
    private_file(
        &directory.join(format!(".upgrade-{temporary}.sqlite3-journal")),
        b"unpublished journal",
    );
    private_file(&marker,serde_json::to_string(&json!({"product":"cubearc.play.workspace.upgrade","workspace_id":request.context.workspace_id,"generation":2,"from_schema":1,"to_schema":2,"temporary":temporary})).unwrap().as_bytes());
    let unrelated = directory.join("unrelated-private-evidence");
    private_file(&unrelated, b"preserve");
    let mut manager = Manager::new(root).unwrap();
    manager
        .handle(Command::Open {
            workspace_id: request.context.workspace_id,
        })
        .unwrap();
    assert!(!copy.exists());
    assert!(!marker.exists());
    assert_eq!(std::fs::read(unrelated).unwrap(), b"preserve");
    assert_eq!(
        command(
            &mut manager,
            json!({"kind":"open_exploration","workspace_id":request.context.workspace_id,"exploration_id":request.context.exploration_id,"stage_id":request.context.stage_id})
        ),
        before
    );
}
#[test]
fn an_unrecognized_upgrade_marker_cannot_authorize_mutation_or_cleanup() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let (request, _) = setup(&mut manager);
    drop(manager);
    native_baseline_fixture(&root, request.context.workspace_id);
    let directory = root.join(request.context.workspace_id.to_string());
    let marker = directory.join("upgrade.json");
    private_file(
        &marker,
        b"{\"product\":\"unknown\",\"temporary\":\"../../source\"}",
    );
    let database = directory.join("workspace.sqlite3");
    let before = std::fs::read(&database).unwrap();
    let original = std::fs::read(&marker).unwrap();
    let mut manager = Manager::new(root).unwrap();
    assert_eq!(
        manager
            .handle(Command::Open {
                workspace_id: request.context.workspace_id
            })
            .unwrap_err(),
        "unsupported_store"
    );
    assert_eq!(std::fs::read(database).unwrap(), before);
    assert_eq!(std::fs::read(marker).unwrap(), original);
}
