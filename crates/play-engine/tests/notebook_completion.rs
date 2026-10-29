use play_engine::workspace::{Context, Manager};
use serde_json::{Value, json};
use uuid::Uuid;
fn command(m: &mut Manager, v: Value) -> Value {
    m.handle(serde_json::from_value(v).unwrap()).unwrap()
}
fn setup(m: &mut Manager) -> Context {
    let w = command(
        m,
        json!({"kind":"create","key":Uuid::new_v4(),"title":"Notebook"}),
    );
    let e = command(
        m,
        json!({"kind":"create_exploration","workspace_id":w["id"],"key":Uuid::new_v4(),"title":"Study"}),
    );
    serde_json::from_value(
        json!({"workspace_id":w["id"],"exploration_id":e["explorationId"],"stage_id":e["stageId"]}),
    )
    .unwrap()
}
fn view(m: &mut Manager, c: &Context) -> Value {
    command(
        m,
        json!({"kind":"open_exploration","workspace_id":c.workspace_id,"exploration_id":c.exploration_id,"stage_id":c.stage_id}),
    )
}
fn add(c: &Context, text: &str) -> Value {
    json!({"kind":"add_page","context":c,"key":Uuid::new_v4(),"title":"Clue page","text":text})
}
fn source(b: &Value) -> Value {
    json!({"kind":"asset","asset":{"asset_id":b["asset"]["id"],"revision":b["asset"]["revision"]},"caption":null})
}
#[test]
fn page_revisions_copies_archive_and_restart_preserve_inert_source() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("private");
    let mut m = Manager::new(root.clone()).unwrap();
    let c = setup(&mut m);
    let text = "<script>globalThis.executed=true</script><img src='https://invalid.test/x'><form action='file:///tmp/a'>";
    let intent = add(&c, text);
    let receipt = command(&mut m, intent.clone());
    assert_eq!(command(&mut m, intent), receipt);
    let v = view(&mut m, &c);
    let b = v["stage"]["blocks"][0].clone();
    assert_eq!(b["asset"]["content"], json!({"kind":"page","text":text}));
    assert!(v["trails"][0]["pipeline"].is_null());
    command(
        &mut m,
        json!({"kind":"duplicate_block","context":c,"key":Uuid::new_v4(),"block_id":b["id"],"expected_revision":v["stage"]["revision"],"source":source(&b)}),
    );
    let copy = view(&mut m, &c)["stage"]["blocks"][1].clone();
    assert_ne!(copy["asset"]["id"], b["asset"]["id"]);
    assert_eq!(copy["asset"]["origin"]["revision"], 1);
    command(
        &mut m,
        json!({"kind":"revise_text","context":c,"key":Uuid::new_v4(),"block_id":b["id"],"asset_id":b["asset"]["id"],"expected_revision":1,"text":"<h1>Edited page</h1>"}),
    );
    let v = view(&mut m, &c);
    assert_eq!(v["stage"]["blocks"][0]["asset"]["revision"], 2);
    assert_eq!(v["stage"]["blocks"][1], copy);
    let updated = v["stage"]["blocks"][0].clone();
    let stale = json!({"kind":"archive_block","context":c,"key":Uuid::new_v4(),"block_id":b["id"],"expected_revision":v["stage"]["revision"],"source":source(&b)});
    assert_eq!(
        m.handle(serde_json::from_value(stale).unwrap())
            .unwrap_err(),
        "state_conflict"
    );
    assert_eq!(view(&mut m, &c), v);
    command(
        &mut m,
        json!({"kind":"archive_block","context":c,"key":Uuid::new_v4(),"block_id":b["id"],"expected_revision":v["stage"]["revision"],"source":source(&updated)}),
    );
    let archived = view(&mut m, &c);
    assert_eq!(archived["stage"]["archivedBlocks"][0]["block"], updated);
    command(
        &mut m,
        json!({"kind":"restore_block","context":c,"key":Uuid::new_v4(),"block_id":b["id"],"expected_revision":archived["stage"]["revision"],"source":source(&updated)}),
    );
    let final_view = view(&mut m, &c);
    drop(m);
    let mut m = Manager::new(root).unwrap();
    command(&mut m, json!({"kind":"open","workspace_id":c.workspace_id}));
    assert_eq!(view(&mut m, &c), final_view);
}
#[test]
fn page_bounds_ownership_and_capacity_reject_without_partial_writes() {
    let tmp = tempfile::tempdir().unwrap();
    let mut m = Manager::new(tmp.path().join("private")).unwrap();
    let c = setup(&mut m);
    let other = setup(&mut m);
    command(&mut m, json!({"kind":"open","workspace_id":c.workspace_id}));
    let before = view(&mut m, &c);
    for payload in [
        add(&c, &"é".repeat(513)),
        add(
            &Context {
                stage_id: other.stage_id,
                ..c.clone()
            },
            "foreign",
        ),
    ] {
        assert!(m.handle(serde_json::from_value(payload).unwrap()).is_err());
        assert_eq!(view(&mut m, &c), before);
    }
    for _ in 0..7 {
        command(&mut m, add(&c, "<p>Inert</p>"));
    }
    let full = view(&mut m, &c);
    assert_eq!(
        m.handle(serde_json::from_value(add(&c, "overflow")).unwrap())
            .unwrap_err(),
        "storage_quota"
    );
    assert_eq!(view(&mut m, &c), full);
}
#[test]
fn image_run_uses_the_exact_image_revision_and_rejects_text_operations() {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let tmp = tempfile::tempdir().unwrap();
    let mut m = Manager::new(tmp.path().join("private")).unwrap();
    let c = setup(&mut m);
    let data = include_bytes!("../../../tests/fixtures/sample.png");
    command(
        &mut m,
        json!({"kind":"add_image","context":c,"key":Uuid::new_v4(),"title":"Image","caption":"Gur xrl.","png":STANDARD.encode(data)}),
    );
    let v = view(&mut m, &c);
    let trail = v["trails"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["source"]["content"]["kind"] == "image")
        .unwrap();
    assert!(
        trail["pipeline"].is_object(),
        "images need a typed executable binding"
    );
    let request = json!({"context":c,"key":Uuid::new_v4(),"trail_id":trail["id"],"pipeline_id":trail["pipelineId"],"pipeline_revision":1,"source_asset_id":trail["source"]["id"],"source_revision":1,"operation":"image.pixels.read","operation_version":1,"retain_output":true});
    let prepared = m
        .prepare_run(&serde_json::from_value(request.clone()).unwrap())
        .unwrap();
    assert_eq!(prepared.invocation.input, STANDARD.encode(data));
    let record = m.run_detail(&c, prepared.identity.id).unwrap();
    assert_eq!(
        record["plan"]["inputs"]["input"]["semanticType"],
        "image.samples"
    );
    assert_eq!(record["plan"]["parameters"]["x"], 0);
    assert_eq!(record["plan"]["parameters"]["y"], 0);
    m.start_attempt(&prepared.identity).unwrap();
    let expected = cube_core::image::Image::decode(data)
        .unwrap()
        .pixel(0, 0)
        .unwrap();
    let complete = m
        .finish_run(
            &prepared.identity,
            json!({"status":"completed","output":{"bytes":expected}}),
        )
        .unwrap();
    assert_eq!(
        complete["result"]["outputs"]["output"]["base64"],
        STANDARD.encode(&expected)
    );
    let mut wrong = request;
    wrong["key"] = json!(Uuid::new_v4());
    wrong["operation"] = json!("cipher.rot13");
    assert_eq!(
        m.prepare_run(&serde_json::from_value(wrong).unwrap())
            .err()
            .unwrap(),
        "pipeline_mismatch"
    );
}

#[path = "support/native_schema.rs"]
mod native_schema;
#[test]
fn schema_seven_preserves_notes_and_assets_while_binding_prior_image_trails() {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("private");
    let mut m = Manager::new(root.clone()).unwrap();
    let c = setup(&mut m);
    command(
        &mut m,
        json!({"kind":"add_annotation","context":c,"key":Uuid::new_v4(),"title":"Observation","text":"Keep this Note"}),
    );
    command(
        &mut m,
        json!({"kind":"add_image","context":c,"key":Uuid::new_v4(),"title":"Prior image","caption":"Gur xrl.","png":STANDARD.encode(include_bytes!("../../../tests/fixtures/sample.png"))}),
    );
    let before = view(&mut m, &c);
    drop(m);
    native_schema::snapshot(&root, c.workspace_id, 7);
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
 use sqlx::{Connection,SqliteConnection,sqlite::SqliteConnectOptions};
 let mut db=SqliteConnection::connect_with(&SqliteConnectOptions::new().filename(root.join(c.workspace_id.to_string()).join("workspace.sqlite3"))).await.unwrap();
 sqlx::query("UPDATE trails SET pipeline_id=NULL,pipeline_revision=NULL WHERE source_id IN (SELECT id FROM assets WHERE kind='image')").execute(&mut db).await.unwrap();db.close().await.unwrap();
 });
    let mut m = Manager::new(root).unwrap();
    command(&mut m, json!({"kind":"open","workspace_id":c.workspace_id}));
    let after = view(&mut m, &c);
    assert_eq!(after["stage"], before["stage"]);
    let image = after["trails"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["source"]["content"]["kind"] == "image")
        .unwrap();
    assert_eq!(image["pipeline"]["initialOperation"], "image.pixels.read");
    assert_eq!(
        image["source"]["id"],
        before["stage"]["blocks"][1]["asset"]["id"]
    );
    command(&mut m, add(&c, "<h1>Page after upgrade</h1>"));
    assert_eq!(
        view(&mut m, &c)["stage"]["blocks"][2]["asset"]["content"]["kind"],
        "page"
    );
}
