use base64::{Engine as _, engine::general_purpose::STANDARD};
use play_engine::workspace::{Command, Context, Manager, RunRequest};
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
fn setup(manager: &mut Manager) -> (Context, Value) {
    let workspace = command(
        manager,
        json!({"kind":"create","key":Uuid::new_v4(),"title":"Ordered notebook"}),
    );
    let created = command(
        manager,
        json!({"kind":"create_exploration","workspace_id":workspace["id"],"key":Uuid::new_v4(),"title":"Clues"}),
    );
    let context:Context=serde_json::from_value(json!({"workspace_id":workspace["id"],"exploration_id":created["explorationId"],"stage_id":created["stageId"]})).unwrap();
    command(
        manager,
        json!({"kind":"add_text","context":context,"key":Uuid::new_v4(),"title":"Text clue","text":"Gur xrl."}),
    );
    command(
        manager,
        json!({"kind":"add_image","context":context,"key":Uuid::new_v4(),"title":"sample.png","caption":"The image caption.","png":STANDARD.encode(include_bytes!("../../../tests/fixtures/sample.png"))}),
    );
    let view = open(manager, &context);
    (context, view)
}
fn ids(view: &Value) -> Vec<Value> {
    view["stage"]["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["id"].clone())
        .collect()
}
fn reorder(context: &Context, view: &Value, ids: Vec<Value>) -> Value {
    json!({"kind":"reorder_blocks","context":context,"key":Uuid::new_v4(),"expected_revision":view["stage"]["revision"],"block_ids":ids})
}
#[test]
fn every_block_including_reasoning_reorders_and_reopens_without_rewriting_evidence() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let (context, before) = setup(&mut manager);
    assert_eq!(
        before["stage"]["blocks"].as_array().unwrap().len(),
        3,
        "reasoning belongs to the same ordered sequence"
    );
    assert!(
        before["stage"].get("note").is_none(),
        "there is one canonical Block sequence"
    );
    let blocks = before["stage"]["blocks"].as_array().unwrap();
    assert_eq!(blocks[2]["kind"], "note");
    let trail = &before["trails"][0];
    let request:RunRequest=serde_json::from_value(json!({"context":context,"key":Uuid::new_v4(),"trail_id":trail["id"],"trail_revision":trail["revision"],"target_node_id":trail["pipeline"]["outputNodeId"],"pipeline_id":trail["pipeline"]["id"],"pipeline_revision":1,"source_asset_id":trail["source"]["id"],"source_revision":1,"operation":"cipher.rot13","operation_version":1,"retain_output":true})).unwrap();
    let prepared = manager.prepare_run(&request).unwrap();
    manager.start_attempt(&prepared.identity).unwrap();
    let original = manager
        .finish_run(
            &prepared.identity,
            json!({"status":"completed","output":{"text":"The key."}}),
        )
        .unwrap();
    let order = vec![
        blocks[2]["id"].clone(),
        blocks[1]["id"].clone(),
        blocks[0]["id"].clone(),
    ];
    let mutation = reorder(&context, &before, order.clone());
    let result = command(&mut manager, mutation.clone());
    assert_eq!(
        result["revision"],
        before["stage"]["revision"].as_i64().unwrap() + 1
    );
    assert_eq!(command(&mut manager, mutation.clone()), result);
    let after = open(&mut manager, &context);
    assert_eq!(ids(&after), order);
    assert_eq!(
        after["stage"]["blocks"],
        json!([blocks[2], blocks[1], blocks[0]])
    );
    assert_eq!(after["trails"], before["trails"]);
    assert_eq!(
        manager.run_detail(&context, prepared.identity.id).unwrap(),
        original
    );
    let mut changed = mutation;
    changed["block_ids"] = json!(ids(&before));
    assert_eq!(
        manager
            .handle(serde_json::from_value(changed).unwrap())
            .unwrap_err(),
        "idempotency_conflict"
    );
    drop(manager);
    let mut manager = Manager::new(root).unwrap();
    manager
        .handle(Command::Open {
            workspace_id: context.workspace_id,
        })
        .unwrap();
    assert_eq!(open(&mut manager, &context)["stage"], after["stage"]);
    assert_eq!(
        manager.run_detail(&context, prepared.identity.id).unwrap(),
        original
    );
}
#[test]
fn invalid_or_stale_orders_cannot_drop_duplicate_or_detach_content() {
    let tmp = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(tmp.path().join("private")).unwrap();
    let (context, before) = setup(&mut manager);
    let original = ids(&before);
    for order in [
        vec![],
        vec![original[0].clone()],
        vec![original[0].clone(); 3],
        vec![
            original[0].clone(),
            original[1].clone(),
            json!(Uuid::new_v4()),
        ],
        vec![
            original[0].clone(),
            before["stage"]["blocks"][1]["caption"]["id"].clone(),
            original[2].clone(),
        ],
    ] {
        assert_eq!(
            manager
                .handle(serde_json::from_value(reorder(&context, &before, order)).unwrap())
                .unwrap_err(),
            "invalid_order"
        );
        assert_eq!(open(&mut manager, &context)["stage"], before["stage"]);
    }
    let mut changed = original.clone();
    changed.swap(0, 1);
    command(&mut manager, reorder(&context, &before, changed.clone()));
    assert_eq!(
        manager
            .handle(serde_json::from_value(reorder(&context, &before, original)).unwrap())
            .unwrap_err(),
        "state_conflict"
    );
    let after = open(&mut manager, &context);
    let noop = command(&mut manager, reorder(&context, &after, changed));
    assert_eq!(noop["revision"], after["stage"]["revision"]);
    let other = command(
        &mut manager,
        json!({"kind":"create_exploration","workspace_id":context.workspace_id,"key":Uuid::new_v4(),"title":"Independent"}),
    );
    let mut mixed = reorder(&context, &after, ids(&after));
    mixed["context"]["exploration_id"] = other["explorationId"].clone();
    assert_eq!(
        manager
            .handle(serde_json::from_value(mixed).unwrap())
            .unwrap_err(),
        "not_found"
    );
    assert_eq!(open(&mut manager, &context)["stage"], after["stage"]);
}
#[test]
fn adding_content_preserves_manual_order_and_invalidates_an_earlier_order_snapshot() {
    let tmp = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(tmp.path().join("private")).unwrap();
    let (context, before) = setup(&mut manager);
    let all = ids(&before);
    assert_eq!(all.len(), 3);
    command(
        &mut manager,
        reorder(
            &context,
            &before,
            vec![all[2].clone(), all[0].clone(), all[1].clone()],
        ),
    );
    let moved = open(&mut manager, &context);
    let added = command(
        &mut manager,
        json!({"kind":"add_text","context":context,"key":Uuid::new_v4(),"title":"New clue","text":"Later content"}),
    );
    let after = open(&mut manager, &context);
    assert_eq!(
        ids(&after),
        vec![
            all[2].clone(),
            all[0].clone(),
            all[1].clone(),
            added["blockId"].clone()
        ]
    );
    assert_eq!(
        after["stage"]["revision"],
        moved["stage"]["revision"].as_i64().unwrap() + 1
    );
    assert_eq!(
        manager
            .handle(serde_json::from_value(reorder(&context, &moved, ids(&moved))).unwrap())
            .unwrap_err(),
        "state_conflict"
    );
    let reasoning = &after["stage"]["blocks"][0];
    command(
        &mut manager,
        json!({"kind":"save_note","context":context,"key":Uuid::new_v4(),"note_id":reasoning["note"]["id"],"expected_revision":1,"text":"Saved reasoning"}),
    );
    let noted = open(&mut manager, &context);
    assert_eq!(
        noted["stage"]["revision"], after["stage"]["revision"],
        "content edits do not change structural order revisions"
    );
    let mut order = ids(&noted);
    order.swap(0, 1);
    command(&mut manager, reorder(&context, &noted, order));
    assert_eq!(
        open(&mut manager, &context)["stage"]["blocks"][1]["note"]["text"],
        "Saved reasoning"
    );
}
