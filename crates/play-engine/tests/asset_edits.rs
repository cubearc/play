use base64::{Engine as _, engine::general_purpose::STANDARD};
use play_engine::workspace::{Command, Context, Manager, RunRequest};
use serde_json::{Value, json};
use uuid::Uuid;

fn command(manager: &mut Manager, value: Value) -> Value {
    manager
        .handle(serde_json::from_value(value).unwrap())
        .unwrap()
}
fn setup(manager: &mut Manager, image: bool) -> (Context, Value) {
    let workspace = command(
        manager,
        json!({"kind":"create","key":Uuid::new_v4(),"title":"Editable notebook"}),
    );
    let created = command(
        manager,
        json!({"kind":"create_exploration","workspace_id":workspace["id"],"key":Uuid::new_v4(),"title":"Clues"}),
    );
    let context: Context = serde_json::from_value(json!({"workspace_id":workspace["id"],"exploration_id":created["explorationId"],"stage_id":created["stageId"]})).unwrap();
    let add = if image {
        json!({"kind":"add_image","context":context,"key":Uuid::new_v4(),"title":"sample.png","caption":"Gur xrl.","png":STANDARD.encode(include_bytes!("../../../tests/fixtures/sample.png"))})
    } else {
        json!({"kind":"add_text","context":context,"key":Uuid::new_v4(),"title":"Clue","text":"Gur xrl."})
    };
    command(manager, add);
    let view = open(manager, &context);
    (context, view)
}
fn open(manager: &mut Manager, context: &Context) -> Value {
    command(
        manager,
        json!({"kind":"open_exploration","workspace_id":context.workspace_id,"exploration_id":context.exploration_id,"stage_id":context.stage_id}),
    )
}
fn edit(context: &Context, block: &Value, asset: &Value, text: &str) -> Value {
    json!({"kind":"revise_text","context":context,"key":Uuid::new_v4(),"block_id":block["id"],"asset_id":asset["id"],"expected_revision":asset["revision"],"text":text})
}
fn run_request(context: &Context, trail: &Value) -> RunRequest {
    serde_json::from_value(json!({"context":context,"key":Uuid::new_v4(),"trail_id":trail["id"],"pipeline_id":trail["pipeline"]["id"],"pipeline_revision":trail["pipeline"]["revision"],"source_asset_id":trail["source"]["id"],"source_revision":trail["source"]["revision"],"operation":"cipher.rot13","operation_version":1,"retain_output":true})).unwrap()
}
#[test]
fn editing_text_preserves_accepted_input_and_reopens_new_content_with_old_evidence() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let (context, before) = setup(&mut manager, false);
    let block = &before["stage"]["blocks"][0];
    let asset = &block["asset"];
    let request = run_request(&context, &before["trails"][0]);
    let accepted = manager.prepare_run(&request).unwrap();
    manager.start_attempt(&accepted.identity).unwrap();
    let mutation = edit(&context, block, asset, "Frpbaq pyhr.");
    let saved = command(&mut manager, mutation.clone());
    assert_eq!(saved["assetId"], asset["id"]);
    assert_eq!(saved["revision"], 2);
    assert_eq!(command(&mut manager, mutation.clone()), saved);
    let after = open(&mut manager, &context);
    assert_eq!(after["stage"]["blocks"][0]["id"], block["id"]);
    assert_eq!(after["stage"]["blocks"][0]["asset"]["id"], asset["id"]);
    assert_eq!(after["stage"]["blocks"][0]["asset"]["revision"], 2);
    assert_eq!(
        after["stage"]["blocks"][0]["asset"]["content"]["text"],
        "Frpbaq pyhr."
    );
    assert_eq!(after["trails"][0]["id"], before["trails"][0]["id"]);
    assert_eq!(after["trails"][0]["source"]["revision"], 2);
    assert_eq!(
        after["trails"][0]["pipeline"],
        before["trails"][0]["pipeline"]
    );
    assert_eq!(accepted.invocation.input, "Gur xrl.");
    let old = manager.finish_run(&accepted.identity, json!({"status":"completed","output":{"text":cube_core::ciphers::rot13(&accepted.invocation.input)}})).unwrap();
    assert_eq!(old["plan"]["inputs"]["input"]["revision"], 1);
    assert_eq!(old["result"]["outputs"]["output"], "The key.");
    assert_eq!(manager.existing_run(&request).unwrap().unwrap(), old);
    let mut stale = request.clone();
    stale.key = Uuid::new_v4();
    assert_eq!(manager.prepare_run(&stale).err(), Some("state_conflict"));
    let mut wrong_source = run_request(&context, &after["trails"][0]);
    wrong_source.source_asset_id = Uuid::new_v4();
    assert_eq!(
        manager.prepare_run(&wrong_source).err(),
        Some("state_conflict")
    );
    let next = run_request(&context, &after["trails"][0]);
    let current = manager.prepare_run(&next).unwrap();
    assert_eq!(current.invocation.input, "Frpbaq pyhr.");
    manager.start_attempt(&current.identity).unwrap();
    let new = manager.finish_run(&current.identity, json!({"status":"completed","output":{"text":cube_core::ciphers::rot13(&current.invocation.input)}})).unwrap();
    assert_eq!(new["result"]["outputs"]["output"], "Second clue.");
    assert_eq!(new["plan"]["inputs"]["input"]["revision"], 2);
    let mut different = mutation;
    different["text"] = json!("different retry");
    assert_eq!(
        manager
            .handle(serde_json::from_value(different).unwrap())
            .unwrap_err(),
        "idempotency_conflict"
    );
    drop(manager);
    let mut reopened = Manager::new(root).unwrap();
    reopened
        .handle(Command::Open {
            workspace_id: context.workspace_id,
        })
        .unwrap();
    assert_eq!(
        open(&mut reopened, &context)["stage"]["blocks"],
        after["stage"]["blocks"]
    );
    assert_eq!(
        reopened.run_detail(&context, accepted.identity.id).unwrap(),
        old
    );
    assert_eq!(
        reopened.run_detail(&context, current.identity.id).unwrap(),
        new
    );
}
#[test]
fn caption_edits_keep_the_figure_image_and_references_intact() {
    let tmp = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(tmp.path().join("private")).unwrap();
    let (context, before) = setup(&mut manager, true);
    let block = &before["stage"]["blocks"][0];
    let read =
        json!({"kind":"read_image","context":context,"asset_id":block["asset"]["id"],"revision":1});
    let png = command(&mut manager, read.clone());
    let caption = &block["caption"];
    command(&mut manager, edit(&context, block, caption, ""));
    let after = open(&mut manager, &context);
    let edited = &after["stage"]["blocks"][0];
    assert_eq!(after["stage"]["blocks"].as_array().unwrap().len(), 2);
    assert_eq!(edited["id"], block["id"]);
    assert_eq!(edited["asset"], block["asset"]);
    assert_eq!(edited["caption"]["id"], caption["id"]);
    assert_eq!(edited["caption"]["revision"], 2);
    assert_eq!(edited["caption"]["content"]["text"], "");
    assert_eq!(command(&mut manager, read), png);
    let image_edit = edit(&context, block, &block["asset"], "not image bytes");
    assert_eq!(
        manager
            .handle(serde_json::from_value(image_edit).unwrap())
            .unwrap_err(),
        "invalid_asset_type"
    );
}
#[test]
fn stale_foreign_and_oversized_edits_leave_current_content_unchanged() {
    let tmp = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(tmp.path().join("private")).unwrap();
    let (context, before) = setup(&mut manager, false);
    let block = &before["stage"]["blocks"][0];
    let asset = &block["asset"];
    command(&mut manager, edit(&context, block, asset, "Frpbaq."));
    let after = open(&mut manager, &context);
    assert_eq!(
        manager
            .handle(serde_json::from_value(edit(&context, block, asset, "stale")).unwrap())
            .unwrap_err(),
        "state_conflict"
    );
    let current = &after["stage"]["blocks"][0]["asset"];
    let unchanged = command(&mut manager, edit(&context, block, current, "Frpbaq."));
    assert_eq!(
        unchanged["revision"], 2,
        "equal content does not append a revision"
    );
    for revision in [0, -1, i64::MAX] {
        let mut invalid = edit(&context, block, current, "invalid revision");
        invalid["expected_revision"] = json!(revision);
        assert_eq!(
            manager
                .handle(serde_json::from_value(invalid).unwrap())
                .unwrap_err(),
            "input_limit"
        );
    }
    let oversized = edit(&context, block, current, &"é".repeat(513));
    assert_eq!(
        manager
            .handle(serde_json::from_value(oversized).unwrap())
            .unwrap_err(),
        "input_limit"
    );
    let other = command(
        &mut manager,
        json!({"kind":"create_exploration","workspace_id":context.workspace_id,"key":Uuid::new_v4(),"title":"Other"}),
    );
    let mut foreign = edit(&context, block, current, "wrong owner");
    foreign["context"]["exploration_id"] = other["explorationId"].clone();
    foreign["context"]["stage_id"] = other["stageId"].clone();
    assert_eq!(
        manager
            .handle(serde_json::from_value(foreign).unwrap())
            .unwrap_err(),
        "not_found"
    );
    let mut unknown = edit(&context, block, current, "wrong block");
    unknown["block_id"] = json!(Uuid::new_v4());
    assert_eq!(
        manager
            .handle(serde_json::from_value(unknown).unwrap())
            .unwrap_err(),
        "not_found"
    );
    assert_eq!(
        open(&mut manager, &context)["stage"]["blocks"],
        after["stage"]["blocks"]
    );
}
