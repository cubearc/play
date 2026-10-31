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
        json!({"kind":"create","key":Uuid::new_v4(),"title":"Archive studies"}),
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
fn source(block: &Value) -> Value {
    let reference = |asset: &Value| json!({"asset_id":asset["id"],"revision":asset["revision"]});
    json!({"kind":"asset","asset":reference(&block["asset"]),"caption":block.get("caption").map(reference)})
}
fn lifecycle(context: &Context, view: &Value, block: &Value, archived: bool) -> Value {
    json!({"kind":if archived{"archive_block"}else{"restore_block"},"context":context,"key":Uuid::new_v4(),"block_id":block["id"],"expected_revision":view["stage"]["revision"],"source":source(block)})
}
fn request(context: &Context, trail: &Value) -> RunRequest {
    serde_json::from_value(json!({"context":context,"key":Uuid::new_v4(),"trail_id":trail["id"],"trail_revision":trail["revision"],"target_node_id":trail["pipeline"]["outputNodeId"],"pipeline_id":trail["pipeline"]["id"],"pipeline_revision":trail["pipeline"]["revision"],"source_asset_id":trail["source"]["id"],"source_revision":trail["source"]["revision"],"operation":"cipher.rot13","operation_version":1,"retain_output":true})).unwrap()
}
#[test]
fn archive_preserves_admitted_execution_and_lineage_then_restores_original_identities() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let (context, initial) = setup(&mut manager, false);
    let original = initial["stage"]["blocks"][0].clone();
    command(
        &mut manager,
        json!({"kind":"duplicate_block","context":context,"key":Uuid::new_v4(),"block_id":original["id"],"expected_revision":initial["stage"]["revision"],"source":source(&original)}),
    );
    let before = open(&mut manager, &context);
    let original_trail = before["trails"]
        .as_array()
        .unwrap()
        .iter()
        .find(|trail| trail["source"]["id"] == original["asset"]["id"])
        .unwrap();
    let accepted = request(&context, original_trail);
    let running = manager.prepare_run(&accepted).unwrap();
    manager.start_attempt(&running.identity).unwrap();
    let intent = lifecycle(&context, &before, &original, true);
    let archived = command(&mut manager, intent.clone());
    assert_eq!(command(&mut manager, intent.clone()), archived);
    let view = open(&mut manager, &context);
    assert_eq!(view["stage"]["blocks"].as_array().unwrap().len(), 2);
    assert_eq!(view["stage"]["blocks"][0], before["stage"]["blocks"][1]);
    assert_eq!(
        view["stage"]["archivedBlocks"][0]["block"]["id"],
        original["id"]
    );
    assert_eq!(
        view["stage"]["archivedBlocks"][0]["archivedRevision"],
        archived["revision"]
    );
    assert_eq!(
        view["trails"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["id"] == original_trail["id"])
            .unwrap()["archived"],
        true
    );
    let completed = manager
        .finish_run(
            &running.identity,
            json!({"status":"completed","output":{"text":"The key."}}),
        )
        .unwrap();
    assert_eq!(manager.existing_run(&accepted).unwrap().unwrap(), completed);
    let mut new = request(&context, original_trail);
    assert_eq!(manager.prepare_run(&new).err(), Some("block_archived"));
    let copy_trail = before["trails"]
        .as_array()
        .unwrap()
        .iter()
        .find(|trail| trail["source"]["id"] != original["asset"]["id"])
        .unwrap();
    let copy_run = manager.prepare_run(&request(&context, copy_trail)).unwrap();
    manager.start_attempt(&copy_run.identity).unwrap();
    assert_eq!(
        manager
            .finish_run(
                &copy_run.identity,
                json!({"status":"completed","output":{"text":"The key."}})
            )
            .unwrap()["result"]["outputs"]["output"],
        "The key."
    );
    drop(manager);
    let mut manager = Manager::new(root).unwrap();
    command(
        &mut manager,
        json!({"kind":"open","workspace_id":context.workspace_id}),
    );
    assert_eq!(open(&mut manager, &context), view);
    assert_eq!(
        manager.run_detail(&context, running.identity.id).unwrap(),
        completed
    );
    let record = &view["stage"]["archivedBlocks"][0]["block"];
    let restored = command(&mut manager, lifecycle(&context, &view, record, false));
    let after = open(&mut manager, &context);
    assert_eq!(after["stage"]["archivedBlocks"], json!([]));
    assert_eq!(after["stage"]["blocks"][1], original);
    assert_eq!(after["stage"]["blocks"][0], before["stage"]["blocks"][1]);
    assert!(
        after["trails"]
            .as_array()
            .unwrap()
            .iter()
            .all(|t| t["archived"] == false)
    );
    assert_eq!(
        command(&mut manager, intent),
        archived,
        "old receipt cannot archive a restored Block again"
    );
    assert_eq!(open(&mut manager, &context), after);
    assert_eq!(restored["archived"], false);
    new.key = Uuid::new_v4();
    assert_eq!(
        serde_json::to_value(manager.prepare_run(&new).unwrap().invocation.inputs).unwrap()["source"]
            ["text"],
        "Gur xrl."
    );
    assert_eq!(
        manager.run_detail(&context, running.identity.id).unwrap(),
        completed
    );
}
#[test]
fn archived_image_and_caption_remain_readable_but_not_editable_or_copyable() {
    let tmp = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(tmp.path().join("private")).unwrap();
    let (context, before) = setup(&mut manager, true);
    let block = before["stage"]["blocks"][0].clone();
    let read =
        json!({"kind":"read_image","context":context,"asset_id":block["asset"]["id"],"revision":1});
    let png = command(&mut manager, read.clone());
    command(&mut manager, lifecycle(&context, &before, &block, true));
    let view = open(&mut manager, &context);
    assert_eq!(view["stage"]["blocks"].as_array().unwrap().len(), 1);
    assert_eq!(command(&mut manager, read), png);
    assert_eq!(view["trails"].as_array().unwrap().len(), 2);
    assert!(
        view["trails"]
            .as_array()
            .unwrap()
            .iter()
            .all(|t| t["archived"] == true)
    );
    for mutation in [
        json!({"kind":"revise_text","context":context,"key":Uuid::new_v4(),"block_id":block["id"],"asset_id":block["caption"]["id"],"expected_revision":1,"text":"Changed"}),
        json!({"kind":"duplicate_block","context":context,"key":Uuid::new_v4(),"block_id":block["id"],"expected_revision":view["stage"]["revision"],"source":source(&block)}),
    ] {
        assert_eq!(
            manager
                .handle(serde_json::from_value(mutation).unwrap())
                .unwrap_err(),
            "block_archived"
        );
        assert_eq!(open(&mut manager, &context), view);
    }
    command(
        &mut manager,
        lifecycle(
            &context,
            &view,
            &view["stage"]["archivedBlocks"][0]["block"],
            false,
        ),
    );
    let after = open(&mut manager, &context);
    assert_eq!(after["stage"]["blocks"][0], block);
    assert_eq!(after["stage"]["archivedBlocks"], json!([]));
}
#[test]
fn stale_foreign_and_reasoning_lifecycle_intents_never_hide_the_wrong_content() {
    let tmp = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(tmp.path().join("private")).unwrap();
    let (context, before) = setup(&mut manager, false);
    let block = &before["stage"]["blocks"][0];
    let stale = lifecycle(&context, &before, block, true);
    command(
        &mut manager,
        json!({"kind":"revise_text","context":context,"key":Uuid::new_v4(),"block_id":block["id"],"asset_id":block["asset"]["id"],"expected_revision":1,"text":"New revision"}),
    );
    let current = open(&mut manager, &context);
    assert_eq!(
        manager
            .handle(serde_json::from_value(stale).unwrap())
            .unwrap_err(),
        "state_conflict"
    );
    assert_eq!(open(&mut manager, &context), current);
    let mut note = lifecycle(&context, &current, &current["stage"]["blocks"][0], true);
    note["block_id"] = current["stage"]["blocks"][1]["id"].clone();
    assert_eq!(
        manager
            .handle(serde_json::from_value(note).unwrap())
            .unwrap_err(),
        "invalid_block_type"
    );
    let mut other = lifecycle(&context, &current, &current["stage"]["blocks"][0], true);
    other["context"]["exploration_id"] = json!(Uuid::new_v4());
    assert_eq!(
        manager
            .handle(serde_json::from_value(other).unwrap())
            .unwrap_err(),
        "not_found"
    );
    let no_op = command(
        &mut manager,
        lifecycle(&context, &current, &current["stage"]["blocks"][0], false),
    );
    assert_eq!(no_op["revision"], current["stage"]["revision"]);
    command(
        &mut manager,
        lifecycle(&context, &current, &current["stage"]["blocks"][0], true),
    );
    let archived = open(&mut manager, &context);
    let same = command(
        &mut manager,
        lifecycle(
            &context,
            &archived,
            &archived["stage"]["archivedBlocks"][0]["block"],
            true,
        ),
    );
    assert_eq!(same["revision"], archived["stage"]["revision"]);
    assert_eq!(
        manager
            .handle(
                serde_json::from_value(lifecycle(
                    &context,
                    &current,
                    &current["stage"]["blocks"][0],
                    false
                ))
                .unwrap()
            )
            .unwrap_err(),
        "state_conflict"
    );
    assert_eq!(open(&mut manager, &context), archived);
}
#[test]
fn active_and_retained_limits_allow_safe_slot_reuse_and_restore_without_erasing_archives() {
    let tmp = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(tmp.path().join("private")).unwrap();
    let (context, _) = setup(&mut manager, false);
    for index in 0..14 {
        let view = open(&mut manager, &context);
        if view["stage"]["blocks"].as_array().unwrap().len() == 8 {
            let block = view["stage"]["blocks"]
                .as_array()
                .unwrap()
                .iter()
                .find(|b| b["kind"] == "asset")
                .unwrap();
            command(&mut manager, lifecycle(&context, &view, block, true));
        }
        command(
            &mut manager,
            json!({"kind":"add_text","context":context,"key":Uuid::new_v4(),"title":format!("Clue {index}"),"text":"Text"}),
        );
    }
    let full = open(&mut manager, &context);
    assert_eq!(full["stage"]["blocks"].as_array().unwrap().len(), 8);
    assert_eq!(full["stage"]["archivedBlocks"].as_array().unwrap().len(), 8);
    let archived = full["stage"]["archivedBlocks"][0]["block"].clone();
    assert_eq!(
        manager
            .handle(serde_json::from_value(lifecycle(&context, &full, &archived, false)).unwrap())
            .unwrap_err(),
        "storage_quota"
    );
    assert_eq!(open(&mut manager, &context), full);
    command(
        &mut manager,
        lifecycle(&context, &full, &full["stage"]["blocks"][0], true),
    );
    let available = open(&mut manager, &context);
    let add = json!({"kind":"add_text","context":context,"key":Uuid::new_v4(),"title":"Excess","text":"Text"});
    assert_eq!(
        manager
            .handle(serde_json::from_value(add).unwrap())
            .unwrap_err(),
        "retained_block_limit"
    );
    command(
        &mut manager,
        lifecycle(&context, &available, &archived, false),
    );
    let restored = open(&mut manager, &context);
    assert_eq!(restored["stage"]["blocks"].as_array().unwrap().len(), 8);
    assert_eq!(
        restored["stage"]["archivedBlocks"]
            .as_array()
            .unwrap()
            .len(),
        8
    );
    assert_eq!(restored["stage"]["blocks"][6]["id"], archived["id"]);
    let mut invalid = restored["stage"]["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["id"].clone())
        .collect::<Vec<_>>();
    invalid[0] = restored["stage"]["archivedBlocks"][0]["block"]["id"].clone();
    assert_eq!(manager.handle(serde_json::from_value(json!({"kind":"reorder_blocks","context":context,"key":Uuid::new_v4(),"expected_revision":restored["stage"]["revision"],"block_ids":invalid})).unwrap()).unwrap_err(),"invalid_order");
    assert_eq!(open(&mut manager, &context), restored);
}
