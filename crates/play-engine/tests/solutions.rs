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
    json!({"kind":"save_solution","context":context,"key":Uuid::new_v4(),"solution_id":id,"expected_revision":expected,"candidate_revision":1,"title":"The hidden key","explanation":"The selected clue supports this answer."})
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
