use base64::{Engine as _, engine::general_purpose::STANDARD};
use play_engine::workspace::{Context, Manager};
use serde_json::{Value, json};
use uuid::Uuid;

fn call(manager: &mut Manager, value: Value) -> Value {
    manager
        .handle(serde_json::from_value(value).expect("closed export command"))
        .unwrap()
}
fn setup(manager: &mut Manager) -> (Context, Value) {
    let workspace = call(
        manager,
        json!({"kind":"create","key":Uuid::new_v4(),"title":"Exports"}),
    );
    let created = call(
        manager,
        json!({"kind":"create_exploration","workspace_id":workspace["id"],"key":Uuid::new_v4(),"title":"Exact evidence"}),
    );
    let context:Context=serde_json::from_value(json!({"workspace_id":workspace["id"],"exploration_id":created["explorationId"],"stage_id":created["stageId"]})).unwrap();
    call(
        manager,
        json!({"kind":"add_text","context":context,"key":Uuid::new_v4(),"title":"Original clue","text":"Gur xrl.\nλ"}),
    );
    let view = call(
        manager,
        json!({"kind":"open_exploration","workspace_id":context.workspace_id,"exploration_id":context.exploration_id,"stage_id":context.stage_id}),
    );
    (context, view)
}
#[test]
fn export_freezes_exact_asset_bytes_and_releases_its_scoped_grant_without_changing_evidence() {
    let root = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(root.path().join("private")).unwrap();
    let (context, before) = setup(&mut manager);
    let asset = &before["trails"][0]["source"];
    let source = json!({"kind":"asset","asset_id":asset["id"],"revision":1});
    let grant = call(
        &mut manager,
        json!({"kind":"prepare_export","context":context,"source":source,"format":"data"}),
    );
    let bytes = "Gur xrl.\nλ".as_bytes();
    assert_eq!(grant["size"], bytes.len());
    assert_eq!(grant["sha256"], cube_core::hashing::sha256(bytes));
    assert_eq!(grant["mime"], "text/plain; charset=utf-8");
    let chunk = call(
        &mut manager,
        json!({"kind":"read_export","context":context,"export_id":grant["exportId"],"offset":0}),
    );
    assert_eq!(
        STANDARD.decode(chunk["base64"].as_str().unwrap()).unwrap(),
        bytes
    );
    assert_eq!(chunk["done"], true);
    assert_eq!(
        call(
            &mut manager,
            json!({"kind":"open_exploration","workspace_id":context.workspace_id,"exploration_id":context.exploration_id,"stage_id":context.stage_id})
        ),
        before
    );
    call(
        &mut manager,
        json!({"kind":"release_export","context":context,"export_id":grant["exportId"]}),
    );
    assert_eq!(manager.handle(serde_json::from_value(json!({"kind":"read_export","context":context,"export_id":grant["exportId"],"offset":0})).unwrap()),Err("not_found"));
}

fn exported(manager: &mut Manager, context: &Context, source: Value, format: &str) -> Vec<u8> {
    let grant = call(
        manager,
        json!({"kind":"prepare_export","context":context,"source":source,"format":format}),
    );
    let mut bytes = Vec::new();
    loop {
        let chunk = call(
            manager,
            json!({"kind":"read_export","context":context,"export_id":grant["exportId"],"offset":bytes.len()}),
        );
        bytes.extend(STANDARD.decode(chunk["base64"].as_str().unwrap()).unwrap());
        if chunk["done"] == true {
            break;
        }
    }
    assert_eq!(bytes.len(), grant["size"].as_u64().unwrap() as usize);
    assert_eq!(cube_core::hashing::sha256(&bytes), grant["sha256"]);
    call(
        manager,
        json!({"kind":"release_export","context":context,"export_id":grant["exportId"]}),
    );
    bytes
}
#[test]
fn walkthrough_exports_only_the_selected_revision_and_does_not_collect_other_private_work() {
    let root = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(root.path().join("private")).unwrap();
    let (context, view) = setup(&mut manager);
    let asset = &view["trails"][0]["source"];
    let source = json!({"kind":"asset","asset_id":asset["id"],"revision":1});
    call(
        &mut manager,
        json!({"kind":"select_candidate","context":context,"key":Uuid::new_v4(),"expected_revision":0,"selection":source}),
    );
    let id = Uuid::new_v4();
    let save = json!({"kind":"save_solution","context":context,"key":Uuid::new_v4(),"solution_id":id,"expected_revision":0,"candidate_revision":1,"notes":[],"title":"Chosen answer","explanation":"First explanation."});
    call(&mut manager, save.clone());
    let selected = json!({"kind":"solution","solution_id":id,"revision":1});
    let first = exported(&mut manager, &context, selected.clone(), "document");
    let mut changed = save;
    changed["key"] = json!(Uuid::new_v4());
    changed["expected_revision"] = json!(1);
    changed["explanation"] = json!("Later private reasoning.");
    call(&mut manager, changed);
    assert_eq!(
        exported(&mut manager, &context, selected.clone(), "document"),
        first
    );
    let doc: Value = serde_json::from_slice(&first).unwrap();
    assert_eq!(doc["format"], "cube-play.walkthrough");
    assert_eq!(doc["solution"]["revision"], 1);
    assert_eq!(doc["solution"]["status"], "private_unverified");
    assert_eq!(doc["solution"]["explanation"], "First explanation.");
    assert_eq!(
        STANDARD
            .decode(doc["solution"]["content"]["base64"].as_str().unwrap())
            .unwrap(),
        "Gur xrl.\nλ".as_bytes()
    );
    assert!(doc["solution"].get("history").is_none());
    assert_eq!(
        manager.handle(
            serde_json::from_value(
                json!({"kind":"prepare_export","context":context,"source":selected,"format":"data"})
            )
            .unwrap()
        ),
        Err("invalid_command")
    );
    let other = call(
        &mut manager,
        json!({"kind":"create_exploration","workspace_id":context.workspace_id,"key":Uuid::new_v4(),"title":"Other"}),
    );
    let foreign = json!({"workspace_id":context.workspace_id,"exploration_id":other["explorationId"],"stage_id":other["stageId"]});
    assert_eq!(
        manager.handle(
            serde_json::from_value(
                json!({"kind":"prepare_export","context":foreign,"source":source,"format":"data"})
            )
            .unwrap()
        ),
        Err("not_found")
    );
}

#[test]
fn retained_run_exports_exact_core_output_while_private_and_foreign_results_are_refused() {
    let root = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(root.path().join("private")).unwrap();
    let (context, view) = setup(&mut manager);
    let trail = &view["trails"][0];
    let mut request:play_engine::workspace::RunRequest=serde_json::from_value(json!({"context":context,"key":Uuid::new_v4(),"trail_id":trail["id"],"trail_revision":trail["revision"],"target_node_id":trail["pipeline"]["outputNodeId"],"pipeline_id":trail["pipeline"]["id"],"pipeline_revision":1,"source_asset_id":trail["source"]["id"],"source_revision":1,"operation":"cipher.rot13","operation_version":1,"retain_output":true})).unwrap();
    for retain in [true, false] {
        request.key = Uuid::new_v4();
        request.retain_output = retain;
        let prepared = manager.prepare_run(&request).unwrap();
        manager.start_attempt(&prepared.identity).unwrap();
        let text = cube_core::ciphers::rot13("Gur xrl.\nλ");
        manager
            .finish_attempt(
                &prepared.identity,
                json!({"status":"completed","output":{"text":text}}),
            )
            .unwrap();
        let run = manager
            .finish_run(
                &prepared.identity,
                json!({"status":"completed","output":{"text":text}}),
            )
            .unwrap();
        assert_eq!(run["status"], "completed", "{run}");
        let source = json!({"kind":"run_output","run_id":run["id"],"attempt_id":run["attempts"][0]["id"],"port":"output"});
        if retain {
            assert_eq!(
                exported(&mut manager, &context, source.clone(), "data"),
                "The key.\nλ".as_bytes()
            );
            let doc: Value =
                serde_json::from_slice(&exported(&mut manager, &context, source, "document"))
                    .unwrap();
            assert_eq!(doc["evidence"]["origin"]["runId"], run["id"]);
        } else {
            assert!(manager.handle(serde_json::from_value(json!({"kind":"prepare_export","context":context,"source":source,"format":"data"})).unwrap()).is_err());
        }
        assert_eq!(
            manager.run_detail(&context, prepared.identity.id).unwrap(),
            run
        );
    }
}

#[test]
fn binary_image_and_inert_page_assets_export_without_preview_loss() {
    let root = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(root.path().join("private")).unwrap();
    let (context, _) = setup(&mut manager);
    let png = include_bytes!("../../../tests/fixtures/sample.png");
    let binary = [0, 255, 128, 10];
    let page = "<script>never execute</script>";
    for command in [
        json!({"kind":"add_image","context":context,"key":Uuid::new_v4(),"title":"Figure","caption":"Related caption","png":STANDARD.encode(png)}),
        json!({"kind":"add_file","context":context,"key":Uuid::new_v4(),"title":"Binary clue","base64":STANDARD.encode(binary)}),
        json!({"kind":"add_page","context":context,"key":Uuid::new_v4(),"title":"Page clue","text":page}),
    ] {
        call(&mut manager, command);
    }
    let view = call(
        &mut manager,
        json!({"kind":"open_exploration","workspace_id":context.workspace_id,"exploration_id":context.exploration_id,"stage_id":context.stage_id}),
    );
    for trail in view["trails"].as_array().unwrap() {
        let asset = &trail["source"];
        let expected = match asset["content"]["kind"].as_str().unwrap() {
            "image" => png.to_vec(),
            "file" => binary.to_vec(),
            "page" => page.as_bytes().to_vec(),
            _ => asset["content"]["text"]
                .as_str()
                .unwrap()
                .as_bytes()
                .to_vec(),
        };
        assert_eq!(
            exported(
                &mut manager,
                &context,
                json!({"kind":"asset","asset_id":asset["id"],"revision":asset["revision"]}),
                "data"
            ),
            expected
        );
    }
}
