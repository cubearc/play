#[path = "support/graph.rs"]
mod graph;
use graph::configuration;
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
    serde_json::from_value(json!({"context":context,"key":Uuid::new_v4(),"trail_id":trail["id"],"trail_revision":trail["revision"],"target_node_id":trail["pipeline"]["outputNodeId"],"pipeline_id":trail["pipeline"]["id"],"pipeline_revision":trail["pipeline"]["revision"],"source_asset_id":trail["source"]["id"],"source_revision":trail["source"]["revision"],"operation":operation,"operation_version":version,"retain_output":retain})).unwrap()
}
fn complete(
    manager: &mut Manager,
    context: &Context,
    trail: &Value,
    retain: bool,
    output: Value,
) -> Value {
    let prepared = manager
        .prepare_run(&request(context, trail, "cipher.rot13", retain))
        .unwrap();
    manager.start_attempt(&prepared.identity).unwrap();
    manager
        .finish_attempt(
            &prepared.identity,
            json!({"status":"completed","output":output.clone()}),
        )
        .unwrap();
    manager
        .finish_run(
            &prepared.identity,
            json!({"status":"completed","output":output}),
        )
        .unwrap()
}
fn select(context: &Context, revision: i64, selection: Value) -> Value {
    json!({"kind":"select_candidate","context":context,"key":Uuid::new_v4(),"expected_revision":revision,"selection":selection})
}
fn candidate(manager: &mut Manager, context: &Context, revision: Option<i64>) -> Value {
    command(
        manager,
        json!({"kind":"get_candidate","context":context,"revision":revision}),
    )["candidate"]
        .clone()
}
#[test]
fn candidate_preserves_asset_revision_after_edit_and_restart_and_is_scoped() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let (context, initial) = setup(&mut manager);
    assert_eq!(candidate(&mut manager, &context, None), Value::Null);
    let block = initial["stage"]["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["kind"] == "asset")
        .unwrap();
    let selection = json!({"kind":"asset","asset_id":block["asset"]["id"],"revision":1});
    let intent = select(&context, 0, selection.clone());
    let receipt = command(&mut manager, intent.clone());
    assert_eq!(command(&mut manager, intent.clone()), receipt);
    let before = candidate(&mut manager, &context, None);
    assert_eq!(before["status"], "private_unverified");
    assert_eq!(before["evidence"]["asset"]["content"]["text"], "Gur xrl.");
    command(
        &mut manager,
        json!({"kind":"revise_text","context":context,"key":Uuid::new_v4(),"block_id":block["id"],"asset_id":block["asset"]["id"],"expected_revision":1,"text":"different"}),
    );
    assert_eq!(candidate(&mut manager, &context, None), before);
    let mut revised = selection.clone();
    revised["revision"] = json!(2);
    let stale = select(&context, 0, revised.clone());
    assert_eq!(
        manager
            .handle(serde_json::from_value(stale).unwrap())
            .unwrap_err(),
        "state_conflict"
    );
    command(&mut manager, select(&context, 1, revised));
    assert_eq!(
        candidate(&mut manager, &context, None)["evidence"]["asset"]["content"]["text"],
        "different"
    );
    assert_eq!(
        candidate(&mut manager, &context, Some(1))["evidence"],
        before["evidence"]
    );
    let other = command(
        &mut manager,
        json!({"kind":"create_exploration","workspace_id":context.workspace_id,"key":Uuid::new_v4(),"title":"Independent"}),
    );
    let separate:Context=serde_json::from_value(json!({"workspace_id":context.workspace_id,"exploration_id":other["explorationId"],"stage_id":other["stageId"]})).unwrap();
    assert_eq!(candidate(&mut manager, &separate, None), Value::Null);
    assert_eq!(
        manager
            .handle(serde_json::from_value(select(&separate, 0, selection)).unwrap())
            .unwrap_err(),
        "not_found"
    );
    drop(manager);
    let mut manager = Manager::new(root).unwrap();
    command(
        &mut manager,
        json!({"kind":"open","workspace_id":context.workspace_id}),
    );
    let restored = candidate(&mut manager, &context, Some(1));
    assert_eq!(restored["id"], receipt["candidateId"]);
    assert_eq!(restored["evidence"], before["evidence"]);
    assert_eq!(restored["history"].as_array().unwrap().len(), 2);
    assert_eq!(command(&mut manager, intent), receipt);
}
#[test]
fn candidate_selects_exact_retained_output_and_rejects_unretained_or_foreign_ports() {
    let tmp = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(tmp.path().join("private")).unwrap();
    let (context, initial) = setup(&mut manager);
    let run = complete(
        &mut manager,
        &context,
        &initial["trails"][0],
        true,
        json!({"text":"The key."}),
    );
    let output = json!({"kind":"run_output","run_id":run["id"],"attempt_id":run["attempts"][0]["id"],"port":"output"});
    command(&mut manager, select(&context, 0, output.clone()));
    let saved = candidate(&mut manager, &context, None);
    assert_eq!(saved["evidence"]["value"], "The key.");
    assert_eq!(saved["evidence"]["semanticType"], "text");
    assert_eq!(saved["evidence"]["origin"]["runId"], run["id"]);
    let next = complete(
        &mut manager,
        &context,
        &initial["trails"][0],
        false,
        json!({"text":"private"}),
    );
    let mut missing = output.clone();
    missing["run_id"] = next["id"].clone();
    missing["attempt_id"] = next["attempts"][0]["id"].clone();
    assert_eq!(
        manager
            .handle(serde_json::from_value(select(&context, 1, missing)).unwrap())
            .unwrap_err(),
        "result_not_available"
    );
    let mut invalid = output;
    invalid["port"] = json!("missing");
    assert_eq!(
        manager
            .handle(serde_json::from_value(select(&context, 1, invalid)).unwrap())
            .unwrap_err(),
        "not_found"
    );
    assert_eq!(candidate(&mut manager, &context, None), saved);
}

#[test]
fn candidate_history_is_bounded_without_replacing_prior_selections() {
    let tmp = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(tmp.path().join("private")).unwrap();
    let (context, initial) = setup(&mut manager);
    let asset = &initial["trails"][0]["source"];
    let selection = json!({"kind":"asset","asset_id":asset["id"],"revision":1});
    for expected in 0..play_engine::protocol::contract().candidate_revision_limit {
        command(
            &mut manager,
            select(&context, expected as i64, selection.clone()),
        );
    }
    let before = candidate(&mut manager, &context, None);
    let intent = select(&context, before["revision"].as_i64().unwrap(), selection);
    assert_eq!(
        manager
            .handle(serde_json::from_value(intent).unwrap())
            .unwrap_err(),
        "candidate_revision_limit"
    );
    assert_eq!(candidate(&mut manager, &context, None), before);
    for revision in [0, 33] {
        let result = manager.handle(
            serde_json::from_value(
                json!({"kind":"get_candidate","context":context,"revision":revision}),
            )
            .unwrap(),
        );
        assert_eq!(
            result.unwrap_err(),
            if revision == 0 {
                "invalid_command"
            } else {
                "not_found"
            }
        );
    }
}

#[test]
fn database_protects_candidate_revisions_and_missing_output_is_an_explicit_gap() {
    use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
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
    let selected = json!({"kind":"run_output","run_id":run["id"],"attempt_id":run["attempts"][0]["id"],"port":"output"});
    command(&mut manager, select(&context, 0, selected.clone()));
    drop(manager);
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let path = root
                .join(context.workspace_id.to_string())
                .join("workspace.sqlite3");
            let mut db = SqliteConnection::connect_with(
                &SqliteConnectOptions::new()
                    .filename(path)
                    .foreign_keys(true),
            )
            .await
            .unwrap();
            for statement in [
                "UPDATE candidate_revisions SET port='changed'",
                "DELETE FROM candidate_revisions",
                "DELETE FROM candidates",
                "DELETE FROM pipeline_runs",
                "DELETE FROM node_results",
            ] {
                assert!(
                    sqlx::query(statement).execute(&mut db).await.is_err(),
                    "{statement}"
                );
            }
            // Simulate unavailable output while restoring the exact native schema.
            sqlx::query("DROP TRIGGER immutable_node_result_delete")
                .execute(&mut db)
                .await
                .unwrap();
            sqlx::query("DELETE FROM node_results")
                .execute(&mut db)
                .await
                .unwrap();
            let trigger = include_str!("../src/workspace/graphs.sql")
                .lines()
                .find(|line| line.starts_with("CREATE TRIGGER immutable_node_result_delete "))
                .unwrap();
            sqlx::raw_sql(trigger).execute(&mut db).await.unwrap();
            db.close().await.unwrap();
        });
    let mut manager = Manager::new(root).unwrap();
    command(
        &mut manager,
        json!({"kind":"open","workspace_id":context.workspace_id}),
    );
    let saved = candidate(&mut manager, &context, None);
    assert_eq!(saved["selection"], selected);
    assert_eq!(
        saved["evidence"],
        json!({"kind":"unavailable","code":"result_not_available"})
    );
    assert_eq!(saved["status"], "private_unverified");
}

#[test]
fn typed_numeric_candidate_uses_core_contract_without_text_conversion() {
    let tmp = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(tmp.path().join("private")).unwrap();
    let (context, initial) = setup(&mut manager);
    let block = initial["stage"]["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["kind"] == "asset")
        .unwrap();
    command(
        &mut manager,
        json!({"kind":"revise_text","context":context,"key":Uuid::new_v4(),"block_id":block["id"],"asset_id":block["asset"]["id"],"expected_revision":1,"text":"42"}),
    );
    let current = view(&mut manager, &context);
    let trail = &current["trails"][0];
    let pipeline = command(
        &mut manager,
        json!({"kind":"create_pipeline","workspace_id":context.workspace_id,"key":Uuid::new_v4(),"title":"Read an integer","definition":configuration("text.integer",1,json!({}))}),
    );
    command(
        &mut manager,
        json!({"kind":"save_trail","context":context,"key":Uuid::new_v4(),"trail_id":trail["id"],"expected_revision":trail["revision"],"hypothesis":"Interpret the clue as an exact integer","pipeline":{"id":pipeline["pipelineId"],"revision":1},"bindings":{}}),
    );
    let updated = view(&mut manager, &context);
    let prepared = manager
        .prepare_run(&request(
            &context,
            &updated["trails"][0],
            "text.integer",
            true,
        ))
        .unwrap();
    manager.start_attempt(&prepared.identity).unwrap();
    let output = play_engine::worker::execute_pipeline(prepared.invocation, |_| Ok(())).unwrap();
    manager
        .finish_attempt(
            &prepared.identity,
            json!({"status":"completed","output":output}),
        )
        .unwrap();
    let run = manager
        .finish_run(
            &prepared.identity,
            json!({"status":"completed","output":output}),
        )
        .unwrap();
    command(
        &mut manager,
        select(
            &context,
            0,
            json!({"kind":"run_output","run_id":run["id"],"attempt_id":run["attempts"][0]["id"],"port":"output"}),
        ),
    );
    let saved = candidate(&mut manager, &context, None);
    assert_eq!(saved["evidence"]["semanticType"], "json");
    assert_eq!(saved["evidence"]["value"], 42);
}

#[test]
fn native_schema_thirteen_upgrades_without_changing_notebook_or_run_evidence() {
    use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
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
    let before = view(&mut manager, &context);
    drop(manager);
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let mut db = SqliteConnection::connect_with(
                &SqliteConnectOptions::new()
                    .filename(
                        root.join(context.workspace_id.to_string())
                            .join("workspace.sqlite3"),
                    )
                    .foreign_keys(false),
            )
            .await
            .unwrap();
            sqlx::raw_sql("DROP TABLE note_targets; DROP TABLE solution_revisions; DROP TABLE solutions; DROP TABLE candidate_revisions; DROP TABLE candidates;")
                .execute(&mut db)
                .await
                .unwrap();
            let schema = include_str!("../src/workspace/challenges.sql");
            let previous = &schema[schema.find("CREATE TABLE workspace_meta_next").unwrap()..];
            sqlx::raw_sql(previous).execute(&mut db).await.unwrap();
            db.close().await.unwrap();
        });
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
    assert_eq!(candidate(&mut manager, &context, None), Value::Null);
    command(
        &mut manager,
        select(
            &context,
            0,
            json!({"kind":"run_output","run_id":run["id"],"attempt_id":run["attempts"][0]["id"],"port":"output"}),
        ),
    );
    assert_eq!(
        candidate(&mut manager, &context, None)["evidence"]["value"],
        "The key."
    );
}
