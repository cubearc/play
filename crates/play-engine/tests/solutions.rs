use play_engine::workspace::{Context, Manager};
use serde_json::{Value, json};
use uuid::Uuid;

fn command(manager: &mut Manager, value: Value) -> Value {
    manager
        .handle(serde_json::from_value(value).unwrap())
        .unwrap()
}
fn setup(manager: &mut Manager) -> (Context, Value) {
    let workspace = command(
        manager,
        json!({"kind":"create","key":Uuid::new_v4(),"title":"Walkthroughs"}),
    );
    let created = command(
        manager,
        json!({"kind":"create_exploration","workspace_id":workspace["id"],"key":Uuid::new_v4(),"title":"Clues"}),
    );
    let context = serde_json::from_value(json!({"workspace_id":workspace["id"],"exploration_id":created["explorationId"],"stage_id":created["stageId"]})).unwrap();
    command(
        manager,
        json!({"kind":"add_text","context":context,"key":Uuid::new_v4(),"title":"Clue","text":"Gur xrl."}),
    );
    let view = command(
        manager,
        json!({"kind":"open_exploration","workspace_id":workspace["id"],"exploration_id":created["explorationId"],"stage_id":created["stageId"]}),
    );
    let asset = &view["trails"][0]["source"];
    command(
        manager,
        json!({"kind":"select_candidate","context":context,"key":Uuid::new_v4(),"expected_revision":0,"selection":{"kind":"asset","asset_id":asset["id"],"revision":1}}),
    );
    (context, view)
}
fn save(context: &Context, id: Uuid, expected: i64) -> Value {
    json!({"kind":"save_solution","context":context,"key":Uuid::new_v4(),"solution_id":id,"expected_revision":expected,"candidate_revision":1,"notes":[],"title":"The hidden key","explanation":"The selected clue supports this answer."})
}
fn read(manager: &mut Manager, context: &Context, id: Uuid, revision: Option<i64>) -> Value {
    command(
        manager,
        json!({"kind":"get_solution","context":context,"solution_id":id,"revision":revision}),
    )
}
#[test]
fn solution_freezes_explanation_selection_and_history_across_edits_and_restart() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let (context, view) = setup(&mut manager);
    let id = Uuid::new_v4();
    let intent = save(&context, id, 0);
    let receipt = command(&mut manager, intent.clone());
    assert_eq!(command(&mut manager, intent.clone()), receipt);
    let before = read(&mut manager, &context, id, None);
    assert_eq!(before["status"], "private_unverified");
    assert_eq!(before["snapshot"]["selection"]["revision"], 1);
    assert_eq!(before["evidence"]["asset"]["content"]["text"], "Gur xrl.");
    let block = view["stage"]["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["kind"] == "asset")
        .unwrap();
    command(
        &mut manager,
        json!({"kind":"revise_text","context":context,"key":Uuid::new_v4(),"block_id":block["id"],"asset_id":block["asset"]["id"],"expected_revision":1,"text":"Changed source"}),
    );
    let mut revised = save(&context, id, 1);
    revised["explanation"] = json!("A clearer explanation.");
    command(&mut manager, revised);
    assert_eq!(
        read(&mut manager, &context, id, Some(1))["snapshot"],
        before["snapshot"]
    );
    assert_eq!(
        read(&mut manager, &context, id, Some(1))["explanation"],
        before["explanation"]
    );
    let other = command(
        &mut manager,
        json!({"kind":"create_exploration","workspace_id":context.workspace_id,"key":Uuid::new_v4(),"title":"Other"}),
    );
    let foreign:Context=serde_json::from_value(json!({"workspace_id":context.workspace_id,"exploration_id":other["explorationId"],"stage_id":other["stageId"]})).unwrap();
    assert_eq!(manager.handle(serde_json::from_value(json!({"kind":"get_solution","context":foreign,"solution_id":id,"revision":null})).unwrap()).unwrap_err(),"not_found");
    assert_eq!(
        command(
            &mut manager,
            json!({"kind":"list_solutions","context":foreign})
        )["solutions"],
        json!([])
    );
    drop(manager);
    let mut manager = Manager::new(root).unwrap();
    command(
        &mut manager,
        json!({"kind":"open","workspace_id":context.workspace_id}),
    );
    let restored = read(&mut manager, &context, id, Some(1));
    assert_eq!(restored["evidence"], before["evidence"]);
    assert_eq!(restored["history"].as_array().unwrap().len(), 2);
    assert_eq!(command(&mut manager, intent), receipt);
    assert_eq!(
        manager
            .handle(serde_json::from_value(save(&context, id, 0)).unwrap())
            .unwrap_err(),
        "state_conflict"
    );
}

#[test]
fn solution_keeps_only_selected_dependencies_and_reports_missing_evidence() {
    use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let (context, view) = setup(&mut manager);
    let ids = [Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4()];
    let node = |id: Uuid, input: Value| json!({"id":id,"operation":"cipher.rot13","operationVersion":1,"parameters":{},"position":{"x":80,"y":80},"inputs":{"input":input}});
    let definition = json!({"formatVersion":4,"inputs":[{"name":"source","semanticType":"text"}],"nodes":[node(ids[0],json!({"kind":"input","name":"source"})),node(ids[1],json!({"kind":"output","nodeId":ids[0],"port":"output"})),node(ids[2],json!({"kind":"literal","value":"Alternative branch"}))],"output":ids[1]});
    let pipeline = command(
        &mut manager,
        json!({"kind":"create_pipeline","workspace_id":context.workspace_id,"key":Uuid::new_v4(),"title":"Alternatives","definition":definition}),
    );
    let trail = &view["trails"][0];
    command(
        &mut manager,
        json!({"kind":"save_trail","context":context,"key":Uuid::new_v4(),"trail_id":trail["id"],"expected_revision":trail["revision"],"hypothesis":"Two rotations recover the clue","pipeline":{"id":pipeline["pipelineId"],"revision":1},"bindings":{}}),
    );
    let request=serde_json::from_value(json!({"context":context,"key":Uuid::new_v4(),"trail_id":trail["id"],"trail_revision":2,"target_node_id":ids[1],"pipeline_id":pipeline["pipelineId"],"pipeline_revision":1,"source_asset_id":trail["source"]["id"],"source_revision":1,"operation":"cipher.rot13","operation_version":1,"retain_output":true})).unwrap();
    let prepared = manager.prepare_run(&request).unwrap();
    let identity = prepared.identity.clone();
    let output = play_engine::worker::execute_pipeline(prepared.invocation, |event| {
        let mut attempt = identity.clone();
        attempt.node_id = serde_json::from_value(event["nodeId"].clone()).unwrap();
        match event["type"].as_str().unwrap() {
            "node_started" => manager.start_attempt(&attempt),
            "node_finished" => manager.finish_attempt(&attempt, event),
            _ => unreachable!(),
        }
    })
    .unwrap();
    let run = manager
        .finish_run(&identity, json!({"status":"completed","output":output}))
        .unwrap();
    let attempt = run["attempts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["nodeId"] == ids[1].to_string())
        .unwrap();
    command(
        &mut manager,
        json!({"kind":"select_candidate","context":context,"key":Uuid::new_v4(),"expected_revision":1,"selection":{"kind":"run_output","run_id":run["id"],"attempt_id":attempt["id"],"port":"output"}}),
    );
    let id = Uuid::new_v4();
    let mut intent = save(&context, id, 0);
    intent["candidate_revision"] = json!(2);
    command(&mut manager, intent);
    let before = read(&mut manager, &context, id, None);
    let path = &before["snapshot"]["path"];
    assert_eq!(path["nodes"].as_array().unwrap().len(), 2);
    assert!(!path.to_string().contains(&ids[2].to_string()));
    assert!(!path.to_string().contains("Alternative branch"));
    assert_eq!(path["inputs"]["source"]["revision"], 1);
    assert_eq!(
        path["trail"]["hypothesis"],
        "Two rotations recover the clue"
    );
    assert_eq!(before["gaps"], json!([]));
    assert_eq!(before["evidence"]["value"], "Gur xrl.");
    let mut changed = definition.clone();
    changed["nodes"][0]["inputs"]["input"] =
        json!({"kind":"literal","value":"A working alternative"});
    command(
        &mut manager,
        json!({"kind":"revise_pipeline","workspace_id":context.workspace_id,"key":Uuid::new_v4(),"pipeline_id":pipeline["pipelineId"],"expected_revision":1,"definition":changed}),
    );
    command(
        &mut manager,
        json!({"kind":"save_trail","context":context,"key":Uuid::new_v4(),"trail_id":trail["id"],"expected_revision":2,"hypothesis":"Changed working hypothesis","pipeline":{"id":pipeline["pipelineId"],"revision":2},"bindings":{}}),
    );
    assert_eq!(read(&mut manager, &context, id, None), before);
    // Choosing an intermediate output must exclude downstream nodes in that same Run.
    let first = run["attempts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["nodeId"] == ids[0].to_string())
        .unwrap();
    command(
        &mut manager,
        json!({"kind":"select_candidate","context":context,"key":Uuid::new_v4(),"expected_revision":2,"selection":{"kind":"run_output","run_id":run["id"],"attempt_id":first["id"],"port":"output"}}),
    );
    let second = Uuid::new_v4();
    let mut intent = save(&context, second, 0);
    intent["candidate_revision"] = json!(3);
    command(&mut manager, intent);
    assert_eq!(
        read(&mut manager, &context, second, None)["snapshot"]["path"]["nodes"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
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
                    .foreign_keys(true),
            )
            .await
            .unwrap();
            for statement in [
                "UPDATE solution_revisions SET explanation='Changed'",
                "DELETE FROM solution_revisions",
                "DELETE FROM solutions",
                "DELETE FROM candidate_revisions",
            ] {
                assert!(sqlx::query(statement).execute(&mut db).await.is_err());
            }
            sqlx::query("DROP TRIGGER immutable_node_result_delete")
                .execute(&mut db)
                .await
                .unwrap();
            sqlx::query("DELETE FROM node_results WHERE attempt_id=?")
                .bind(first["id"].as_str().unwrap())
                .execute(&mut db)
                .await
                .unwrap();
            let trigger = include_str!("../src/workspace/graphs.sql")
                .lines()
                .find(|l| l.starts_with("CREATE TRIGGER immutable_node_result_delete "))
                .unwrap();
            sqlx::raw_sql(trigger).execute(&mut db).await.unwrap();
            db.close().await.unwrap();
        });
    let mut manager = Manager::new(root).unwrap();
    command(
        &mut manager,
        json!({"kind":"open","workspace_id":context.workspace_id}),
    );
    let after = read(&mut manager, &context, id, None);
    assert_eq!(after["snapshot"], before["snapshot"]);
    assert_eq!(after["evidence"], before["evidence"]);
    assert_eq!(after["gaps"][0]["code"], "result_not_available");
    assert_eq!(after["status"], "private_unverified");
}

#[test]
fn solution_limits_and_invalid_input_leave_history_unchanged() {
    let temp = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(temp.path().join("private")).unwrap();
    let (context, _) = setup(&mut manager);
    let id = Uuid::new_v4();
    for (field, value, code) in [
        ("title", json!(""), "invalid_title"),
        ("explanation", json!(" "), "invalid_command"),
        ("explanation", json!("é".repeat(1025)), "input_limit"),
        ("candidate_revision", json!(0), "invalid_command"),
        ("candidate_revision", json!(2), "not_found"),
    ] {
        let mut intent = save(&context, id, 0);
        intent[field] = value;
        assert_eq!(
            manager
                .handle(serde_json::from_value(intent).unwrap())
                .unwrap_err(),
            code
        );
    }
    for revision in 0..play_engine::protocol::contract().solution_revision_limit {
        command(&mut manager, save(&context, id, revision as i64));
    }
    let before = read(&mut manager, &context, id, None);
    assert_eq!(
        manager
            .handle(
                serde_json::from_value(save(&context, id, before["revision"].as_i64().unwrap()))
                    .unwrap()
            )
            .unwrap_err(),
        "solution_revision_limit"
    );
    assert_eq!(read(&mut manager, &context, id, None), before);
    for _ in 1..play_engine::protocol::contract().solution_limit {
        command(&mut manager, save(&context, Uuid::new_v4(), 0));
    }
    assert_eq!(
        manager
            .handle(serde_json::from_value(save(&context, Uuid::new_v4(), 0)).unwrap())
            .unwrap_err(),
        "solution_limit"
    );
}

#[test]
fn schema_fourteen_upgrade_preserves_candidates_and_adds_solution_storage() {
    use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let (context, _) = setup(&mut manager);
    let get = json!({"kind":"get_candidate","context":context,"revision":null});
    let before = command(&mut manager, get.clone());
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
            sqlx::raw_sql(
                "DROP TABLE note_targets; DROP TABLE solution_revisions; DROP TABLE solutions;",
            )
            .execute(&mut db)
            .await
            .unwrap();
            let schema = include_str!("../src/workspace/candidates.sql");
            sqlx::raw_sql(&schema[schema.find("CREATE TABLE workspace_meta_next").unwrap()..])
                .execute(&mut db)
                .await
                .unwrap();
            db.close().await.unwrap();
        });
    let mut manager = Manager::new(root).unwrap();
    command(
        &mut manager,
        json!({"kind":"open","workspace_id":context.workspace_id}),
    );
    assert_eq!(command(&mut manager, get), before);
    let id = Uuid::new_v4();
    command(&mut manager, save(&context, id, 0));
    assert_eq!(
        read(&mut manager, &context, id, None)["evidence"],
        before["candidate"]["evidence"]
    );
}

#[test]
fn unsupported_solution_snapshots_are_refused_without_compatibility_readers() {
    use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let (context, _) = setup(&mut manager);
    let id = Uuid::new_v4();
    command(&mut manager, save(&context, id, 0));
    drop(manager);
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let mut db = SqliteConnection::connect_with(
                &SqliteConnectOptions::new().filename(
                    root.join(context.workspace_id.to_string())
                        .join("workspace.sqlite3"),
                ),
            )
            .await
            .unwrap();
            sqlx::query("DROP TRIGGER solution_revisions_immutable_update")
                .execute(&mut db)
                .await
                .unwrap();
            sqlx::query(
                "UPDATE solution_revisions SET snapshot=json_set(snapshot,'$.formatVersion',0)",
            )
            .execute(&mut db)
            .await
            .unwrap();
            let trigger = include_str!("../src/workspace/solutions.sql")
                .lines()
                .find(|l| l.starts_with("CREATE TRIGGER solution_revisions_immutable_update "))
                .unwrap();
            sqlx::raw_sql(trigger).execute(&mut db).await.unwrap();
            db.close().await.unwrap();
        });
    let mut manager = Manager::new(root).unwrap();
    command(
        &mut manager,
        json!({"kind":"open","workspace_id":context.workspace_id}),
    );
    assert_eq!(manager.handle(serde_json::from_value(json!({"kind":"get_solution","context":context,"solution_id":id,"revision":null})).unwrap()).unwrap_err(),"unsupported_solution");
}

fn execute(manager: &mut Manager, context: &Context, trail: &Value) -> Value {
    let request = serde_json::from_value(json!({"context":context,"key":Uuid::new_v4(),"trail_id":trail["id"],"trail_revision":trail["revision"],"target_node_id":trail["pipeline"]["outputNodeId"],"pipeline_id":trail["pipeline"]["id"],"pipeline_revision":trail["pipeline"]["revision"],"source_asset_id":trail["source"]["id"],"source_revision":trail["source"]["revision"],"operation":trail["pipeline"]["outputOperation"],"operation_version":trail["pipeline"]["operationVersion"],"retain_output":true})).unwrap();
    let prepared = manager.prepare_run(&request).unwrap();
    let identity = prepared.identity.clone();
    let output = play_engine::worker::execute_pipeline(prepared.invocation, |event| {
        let mut attempt = identity.clone();
        attempt.node_id = serde_json::from_value(event["nodeId"].clone()).unwrap();
        match event["type"].as_str().unwrap() {
            "node_started" => manager.start_attempt(&attempt),
            "node_finished" => manager.finish_attempt(&attempt, event),
            _ => unreachable!(),
        }
    })
    .unwrap();
    manager
        .finish_run(&identity, json!({"status":"completed","output":output}))
        .unwrap()
}
fn open(manager: &mut Manager, context: &Context) -> Value {
    command(
        manager,
        json!({"kind":"open_exploration","workspace_id":context.workspace_id,"exploration_id":context.exploration_id,"stage_id":context.stage_id}),
    )
}
#[test]
fn solution_follows_promoted_outputs_across_runs_without_unrelated_attempts() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let (context, view) = setup(&mut manager);
    let first = execute(&mut manager, &context, &view["trails"][0]);
    let unrelated = execute(&mut manager, &context, &view["trails"][0]);
    let pinned = command(
        &mut manager,
        json!({"kind":"pin_run_output","context":context,"key":Uuid::new_v4(),"expected_revision":view["stage"]["revision"],"run_id":first["id"],"attempt_id":first["attempts"][0]["id"],"port":"output"}),
    );
    let view = open(&mut manager, &context);
    let block = view["stage"]["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["id"] == pinned["blockId"])
        .unwrap();
    let trail = view["trails"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["source"]["id"] == block["asset"]["id"])
        .unwrap();
    let second = execute(&mut manager, &context, trail);
    command(
        &mut manager,
        json!({"kind":"select_candidate","context":context,"key":Uuid::new_v4(),"expected_revision":1,"selection":{"kind":"run_output","run_id":second["id"],"attempt_id":second["attempts"][0]["id"],"port":"output"}}),
    );
    let id = Uuid::new_v4();
    let mut intent = save(&context, id, 0);
    intent["candidate_revision"] = json!(2);
    command(&mut manager, intent);
    let before = read(&mut manager, &context, id, None);
    let lineage = &before["snapshot"]["lineage"];
    assert_eq!(lineage["paths"][0]["path"]["runId"], first["id"]);
    assert_eq!(lineage["paths"].as_array().unwrap().len(), 1);
    assert_eq!(lineage["assets"].as_array().unwrap().len(), 2);
    assert!(
        !before["snapshot"]
            .to_string()
            .contains(unrelated["id"].as_str().unwrap())
    );
    assert_eq!(before["evidence"]["value"], "Gur xrl.");
    assert_eq!(before["gaps"], json!([]));
    drop(manager);
    let mut manager = Manager::new(root.clone()).unwrap();
    command(
        &mut manager,
        json!({"kind":"open","workspace_id":context.workspace_id}),
    );
    assert_eq!(read(&mut manager, &context, id, None), before);
    drop(manager);
    remove_result(
        &root,
        &context,
        first["attempts"][0]["id"].as_str().unwrap(),
    );
    let mut manager = Manager::new(root.clone()).unwrap();
    command(
        &mut manager,
        json!({"kind":"open","workspace_id":context.workspace_id}),
    );
    let after = read(&mut manager, &context, id, None);
    assert_eq!(after["snapshot"], before["snapshot"]);
    assert_eq!(after["evidence"], before["evidence"]);
    assert_eq!(after["gaps"].as_array().unwrap().len(), 1);
    assert_eq!(after["gaps"][0]["code"], "result_not_available");
    assert!(
        after["gaps"][0]["reference"]
            .as_str()
            .unwrap()
            .contains(first["attempts"][0]["id"].as_str().unwrap())
    );
    let mut revised = save(&context, id, 1);
    revised["candidate_revision"] = json!(2);
    assert_eq!(
        manager
            .handle(serde_json::from_value(revised).unwrap())
            .unwrap_err(),
        "result_not_available"
    );
    assert_eq!(read(&mut manager, &context, id, None), after);
}

#[test]
fn solution_records_copy_ancestry_without_claiming_edited_bytes_are_run_outputs() {
    let temp = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(temp.path().join("private")).unwrap();
    let (context, view) = setup(&mut manager);
    let original = view["stage"]["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["kind"] == "asset")
        .unwrap();
    let copy = command(
        &mut manager,
        json!({"kind":"duplicate_block","context":context,"key":Uuid::new_v4(),"expected_revision":view["stage"]["revision"],"block_id":original["id"],"source":{"kind":"asset","asset":{"asset_id":original["asset"]["id"],"revision":1},"caption":null}}),
    );
    let view = open(&mut manager, &context);
    let copied = view["stage"]["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["id"] == copy["blockId"])
        .unwrap();
    command(
        &mut manager,
        json!({"kind":"revise_text","context":context,"key":Uuid::new_v4(),"block_id":copied["id"],"asset_id":copied["asset"]["id"],"expected_revision":1,"text":"A manually edited interpretation"}),
    );
    command(
        &mut manager,
        json!({"kind":"select_candidate","context":context,"key":Uuid::new_v4(),"expected_revision":1,"selection":{"kind":"asset","asset_id":copied["asset"]["id"],"revision":2}}),
    );
    let id = Uuid::new_v4();
    let mut intent = save(&context, id, 0);
    intent["candidate_revision"] = json!(2);
    command(&mut manager, intent);
    let saved = read(&mut manager, &context, id, None);
    assert_eq!(
        saved["evidence"]["asset"]["content"]["text"],
        "A manually edited interpretation"
    );
    assert_eq!(saved["snapshot"]["path"], Value::Null);
    let lineage = &saved["snapshot"]["lineage"];
    assert_eq!(lineage["paths"], json!([]));
    assert_eq!(lineage["assets"].as_array().unwrap().len(), 2);
    assert_eq!(lineage["assets"][0]["revision"], 2);
    assert_eq!(lineage["assets"][0]["editedSinceCopy"], true);
    assert_eq!(lineage["assets"][1]["assetId"], original["asset"]["id"]);
    assert_eq!(lineage["assets"][1]["revision"], 1);
    assert_eq!(lineage["assets"][1]["editedSinceCopy"], false);
}

fn remove_result(root: &std::path::Path, context: &Context, attempt: &str) {
    use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let mut db = SqliteConnection::connect_with(
                &SqliteConnectOptions::new().filename(
                    root.join(context.workspace_id.to_string())
                        .join("workspace.sqlite3"),
                ),
            )
            .await
            .unwrap();
            sqlx::query("DROP TRIGGER immutable_node_result_delete")
                .execute(&mut db)
                .await
                .unwrap();
            sqlx::query("DELETE FROM node_results WHERE attempt_id=?")
                .bind(attempt)
                .execute(&mut db)
                .await
                .unwrap();
            let trigger = include_str!("../src/workspace/graphs.sql")
                .lines()
                .find(|l| l.starts_with("CREATE TRIGGER immutable_node_result_delete "))
                .unwrap();
            sqlx::raw_sql(trigger).execute(&mut db).await.unwrap();
            db.close().await.unwrap();
        });
}

#[test]
fn solution_deduplicates_an_asset_shared_by_multiple_pipeline_inputs() {
    let temp = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(temp.path().join("private")).unwrap();
    let (context, view) = setup(&mut manager);
    let node = Uuid::new_v4();
    let definition = json!({"formatVersion":4,"inputs":[{"name":"source","semanticType":"text"},{"name":"other","semanticType":"text"}],"nodes":[{"id":node,"operation":"text.concat","operationVersion":1,"parameters":{},"position":{"x":80,"y":80},"inputs":{"left":{"kind":"input","name":"source"},"right":{"kind":"input","name":"other"}}}],"output":node});
    let pipeline = command(
        &mut manager,
        json!({"kind":"create_pipeline","workspace_id":context.workspace_id,"key":Uuid::new_v4(),"title":"Shared clue","definition":definition}),
    );
    let trail = &view["trails"][0];
    command(
        &mut manager,
        json!({"kind":"save_trail","context":context,"key":Uuid::new_v4(),"trail_id":trail["id"],"expected_revision":1,"hypothesis":"Use the same evidence twice","pipeline":{"id":pipeline["pipelineId"],"revision":1},"bindings":{"other":{"asset_id":trail["source"]["id"],"revision":1}}}),
    );
    let view = open(&mut manager, &context);
    let run = execute(&mut manager, &context, &view["trails"][0]);
    command(
        &mut manager,
        json!({"kind":"select_candidate","context":context,"key":Uuid::new_v4(),"expected_revision":1,"selection":{"kind":"run_output","run_id":run["id"],"attempt_id":run["attempts"][0]["id"],"port":"output"}}),
    );
    let id = Uuid::new_v4();
    let mut intent = save(&context, id, 0);
    intent["candidate_revision"] = json!(2);
    command(&mut manager, intent);
    let saved = read(&mut manager, &context, id, None);
    assert_eq!(
        saved["snapshot"]["path"]["inputs"]
            .as_object()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        saved["snapshot"]["lineage"]["assets"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(saved["evidence"]["value"], "Gur xrl.Gur xrl.");
    assert_eq!(saved["gaps"], json!([]));
}

#[test]
fn solution_freezes_only_chosen_note_revisions_and_their_exact_targets() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let (context, view) = setup(&mut manager);
    let asset = &view["trails"][0]["source"];
    let linked = command(
        &mut manager,
        json!({"kind":"annotate_evidence","context":context,"key":Uuid::new_v4(),"title":"Decoding clue","text":"The original letters matter.","target":{"kind":"asset","asset_id":asset["id"],"revision":1}}),
    );
    command(
        &mut manager,
        json!({"kind":"add_annotation","context":context,"key":Uuid::new_v4(),"title":"Unrelated branch","text":"This alternative is not part of the explanation."}),
    );
    let view = open(&mut manager, &context);
    let block = view["stage"]["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["id"] == linked["blockId"])
        .unwrap();
    let selected = json!({"note_id":block["note"]["id"],"revision":1});
    let id = Uuid::new_v4();
    let mut intent = save(&context, id, 0);
    intent["notes"] = json!([selected]);
    let receipt = command(&mut manager, intent.clone());
    assert_eq!(command(&mut manager, intent), receipt);
    let before = read(&mut manager, &context, id, None);
    assert_eq!(before["snapshot"]["notes"].as_array().unwrap().len(), 1);
    assert_eq!(
        before["snapshot"]["notes"][0]["text"],
        "The original letters matter."
    );
    assert_eq!(
        before["snapshot"]["notes"][0]["target"]["reference"]["revision"],
        1
    );
    assert!(!before["snapshot"].to_string().contains("Unrelated branch"));
    command(
        &mut manager,
        json!({"kind":"save_note","context":context,"key":Uuid::new_v4(),"note_id":block["note"]["id"],"expected_revision":1,"text":"A later interpretation."}),
    );
    assert_eq!(read(&mut manager, &context, id, None), before);
    let mut revise = save(&context, id, 1);
    revise["notes"] = json!([selected]);
    command(&mut manager, revise);
    assert_eq!(
        read(&mut manager, &context, id, Some(1))["snapshot"],
        before["snapshot"]
    );
    drop(manager);
    let mut manager = Manager::new(root).unwrap();
    command(
        &mut manager,
        json!({"kind":"open","workspace_id":context.workspace_id}),
    );
    assert_eq!(
        read(&mut manager, &context, id, Some(1))["snapshot"],
        before["snapshot"]
    );
    for notes in [
        json!([selected, selected]),
        json!([{"note_id":Uuid::new_v4(),"revision":1}]),
        json!([{"note_id":block["note"]["id"],"revision":0}]),
    ] {
        let mut invalid = save(&context, Uuid::new_v4(), 0);
        invalid["notes"] = notes;
        assert!(
            manager
                .handle(serde_json::from_value(invalid).unwrap())
                .is_err()
        );
    }
}

#[test]
fn solution_notes_keep_missing_targets_as_gaps_and_refuse_foreign_notes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let (context, view) = setup(&mut manager);
    let run = execute(&mut manager, &context, &view["trails"][0]);
    let added = command(
        &mut manager,
        json!({"kind":"annotate_evidence","context":context,"key":Uuid::new_v4(),"title":"Output observation","text":"This retained output supports the explanation.","target":{"kind":"run_output","run_id":run["id"],"attempt_id":run["attempts"][0]["id"],"port":"output"}}),
    );
    let view = open(&mut manager, &context);
    let note = &view["stage"]["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["id"] == added["blockId"])
        .unwrap()["note"];
    let id = Uuid::new_v4();
    let mut intent = save(&context, id, 0);
    intent["notes"] = json!([{"note_id":note["id"],"revision":1}]);
    command(&mut manager, intent.clone());
    let before = read(&mut manager, &context, id, None);
    let other = command(
        &mut manager,
        json!({"kind":"create_exploration","workspace_id":context.workspace_id,"key":Uuid::new_v4(),"title":"Independent"}),
    );
    let foreign = command(
        &mut manager,
        json!({"kind":"open_exploration","workspace_id":context.workspace_id,"exploration_id":other["explorationId"],"stage_id":other["stageId"]}),
    );
    let foreign_note = &foreign["stage"]["blocks"][0]["note"]["id"];
    let mut invalid = save(&context, Uuid::new_v4(), 0);
    invalid["notes"] = json!([{"note_id":foreign_note,"revision":1}]);
    assert_eq!(
        manager
            .handle(serde_json::from_value(invalid).unwrap())
            .unwrap_err(),
        "not_found"
    );
    drop(manager);
    remove_result(&root, &context, run["attempts"][0]["id"].as_str().unwrap());
    let mut manager = Manager::new(root).unwrap();
    command(
        &mut manager,
        json!({"kind":"open","workspace_id":context.workspace_id}),
    );
    let after = read(&mut manager, &context, id, None);
    assert_eq!(after["snapshot"], before["snapshot"]);
    assert_eq!(after["evidence"], before["evidence"]);
    assert_eq!(after["gaps"].as_array().unwrap().len(), 1);
    assert_eq!(after["gaps"][0]["code"], "result_not_available");
    assert!(
        after["gaps"][0]["reference"]
            .as_str()
            .unwrap()
            .starts_with("note-target:")
    );
    intent["expected_revision"] = json!(1);
    intent["key"] = json!(Uuid::new_v4());
    command(&mut manager, intent);
    assert_eq!(
        read(&mut manager, &context, id, None)["gaps"],
        after["gaps"]
    );
}

#[test]
fn two_imports_of_one_challenge_keep_candidate_solution_and_note_histories_separate() {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let workspace = command(
        &mut manager,
        json!({"kind":"create","key":Uuid::new_v4(),"title":"Independent solvers"}),
    );
    let package = json!({"format":"cubearc.play.challenge","version":1,"id":Uuid::new_v4(),"revision":1,"title":"One challenge","origin":null,"stages":[{"id":Uuid::new_v4(),"title":"The clue","blocks":[{"id":Uuid::new_v4(),"title":"Clue","content":{"kind":"text","text":"Gur xrl."}}]}]});
    let encoded = STANDARD.encode(serde_json::to_vec(&package).unwrap());
    let mut histories = Vec::new();
    for text in ["First explanation", "Independent alternative"] {
        let imported = command(
            &mut manager,
            json!({"kind":"import_challenge","workspace_id":workspace["id"],"key":Uuid::new_v4(),"package":encoded}),
        );
        let context:Context = serde_json::from_value(json!({"workspace_id":workspace["id"],"exploration_id":imported["explorationId"],"stage_id":imported["stageId"]})).unwrap();
        let view = open(&mut manager, &context);
        assert_eq!(view["origin"]["packageId"], package["id"]);
        let asset = &view["trails"][0]["source"];
        let note = &view["stage"]["blocks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["note"]["kind"] == "reasoning")
            .unwrap()["note"];
        command(
            &mut manager,
            json!({"kind":"save_note","context":context,"key":Uuid::new_v4(),"note_id":note["id"],"expected_revision":1,"text":text}),
        );
        command(
            &mut manager,
            json!({"kind":"select_candidate","context":context,"key":Uuid::new_v4(),"expected_revision":0,"selection":{"kind":"asset","asset_id":asset["id"],"revision":1}}),
        );
        let id = Uuid::new_v4();
        let mut intent = save(&context, id, 0);
        intent["notes"] = json!([{"note_id":note["id"],"revision":2}]);
        command(&mut manager, intent);
        let value = read(&mut manager, &context, id, None);
        assert_eq!(value["snapshot"]["notes"][0]["text"], text);
        histories.push((context, id, value));
    }
    assert_ne!(histories[0].0.exploration_id, histories[1].0.exploration_id);
    assert_ne!(
        histories[0].2["snapshot"]["notes"][0]["id"],
        histories[1].2["snapshot"]["notes"][0]["id"]
    );
    assert_eq!(manager.handle(serde_json::from_value(json!({"kind":"get_solution","context":histories[0].0,"solution_id":histories[1].1,"revision":null})).unwrap()).unwrap_err(), "not_found");
    drop(manager);
    let mut manager = Manager::new(root).unwrap();
    command(
        &mut manager,
        json!({"kind":"open","workspace_id":workspace["id"]}),
    );
    for (context, id, value) in histories {
        assert_eq!(read(&mut manager, &context, id, None), value);
        assert_eq!(
            command(
                &mut manager,
                json!({"kind":"list_solutions","context":context})
            )["solutions"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }
}
