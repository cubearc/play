use base64::{Engine as _, engine::general_purpose::STANDARD};
use play_engine::workspace::{Command, Manager};
use serde_json::{Value, json};
use uuid::Uuid;

fn command(manager: &mut Manager, value: Value) -> Value {
    manager
        .handle(serde_json::from_value(value).unwrap())
        .unwrap()
}

#[test]
fn canonical_package_import_creates_an_editable_notebook_atomically() {
    let temporary = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(temporary.path().join("private")).unwrap();
    let workspace = command(
        &mut manager,
        json!({"kind":"create","key":Uuid::new_v4(),"title":"Packages"}),
    );
    let package = json!({"format":"cubearc.play.challenge","version":1,"id":Uuid::new_v4(),"revision":1,"title":"Signals","origin":null,"stages":[{"id":Uuid::new_v4(),"title":"First clue","blocks":[{"id":Uuid::new_v4(),"title":"Caption clue","content":{"kind":"text","text":"Uryyb"}}]}]});
    let request = json!({"kind":"import_challenge","workspace_id":workspace["id"],"key":Uuid::new_v4(),"package":STANDARD.encode(serde_json::to_vec(&package).unwrap())});
    let parsed = serde_json::from_value::<Command>(request.clone());
    assert!(
        parsed.is_ok(),
        "canonical package command must be supported: {parsed:?}"
    );
    let imported = manager.handle(parsed.unwrap()).unwrap();
    assert_eq!(command(&mut manager, request), imported);
    let view = command(
        &mut manager,
        json!({"kind":"open_exploration","workspace_id":workspace["id"],"exploration_id":imported["explorationId"],"stage_id":null}),
    );
    assert_eq!(view["title"], "Signals");
    assert_eq!(view["stage"]["title"], "First clue");
    assert_eq!(
        view["stage"]["blocks"][0]["asset"]["content"]["text"],
        "Uryyb"
    );
    assert_eq!(view["origin"]["packageId"], package["id"]);
}

fn package() -> Value {
    json!({"format":"cubearc.play.challenge","version":1,"id":Uuid::new_v4(),"revision":1,"title":"Signals","origin":{"service_id":Uuid::new_v4(),"challenge_id":Uuid::new_v4(),"edition_id":Uuid::new_v4()},"stages":[{"id":Uuid::new_v4(),"title":"First clue","blocks":[{"id":Uuid::new_v4(),"title":"Text","content":{"kind":"text","text":"Uryyb"}}]}]})
}
fn request(workspace: &Value, package: &Value) -> Value {
    json!({"kind":"import_challenge","workspace_id":workspace["id"],"key":Uuid::new_v4(),"package":STANDARD.encode(serde_json::to_vec(package).unwrap())})
}
fn open(manager: &mut Manager, workspace: &Value, imported: &Value) -> Value {
    command(
        manager,
        json!({"kind":"open_exploration","workspace_id":workspace["id"],"exploration_id":imported["explorationId"],"stage_id":imported["stageId"]}),
    )
}
#[test]
fn grouped_assets_pages_notes_and_provenance_survive_edits_and_restart() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("private");
    let mut manager = Manager::new(root.clone()).unwrap();
    let workspace = command(
        &mut manager,
        json!({"kind":"create","key":Uuid::new_v4(),"title":"Packages"}),
    );
    let mut p = package();
    let image = cube_core::image::Image::from_bytes(
        1,
        1,
        cube_core::image::ColorModel::Rgba,
        &[65, 66, 67, 255],
        &[],
        &[],
    )
    .unwrap()
    .encode_png()
    .unwrap();
    p["stages"][0]["blocks"] = json!([
        {"id":Uuid::new_v4(),"title":"Signal","content":{"kind":"image","base64":STANDARD.encode(&image),"sha256":cube_core::hashing::sha256(&image),"caption":"Uryyb"}},
        {"id":Uuid::new_v4(),"title":"Page","content":{"kind":"page","text":"<script>throw Error('inert')</script>"}},
        {"id":Uuid::new_v4(),"title":"Bytes","content":{"kind":"file","base64":"AAEC","sha256":cube_core::hashing::sha256(&[0,1,2])}},
        {"id":Uuid::new_v4(),"title":"Observation","content":{"kind":"annotation","text":"Inspect the pixels."}}
    ]);
    p["stages"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":Uuid::new_v4(),"title":"Second clue","blocks":[]}));
    let imported = command(&mut manager, request(&workspace, &p));
    let view = open(&mut manager, &workspace, &imported);
    assert_eq!(view["stages"].as_array().unwrap().len(), 2);
    let blocks = &view["stage"]["blocks"];
    assert_eq!(blocks.as_array().unwrap().len(), 5);
    assert_eq!(blocks[0]["asset"]["content"]["kind"], "image");
    assert_eq!(blocks[0]["caption"]["content"]["text"], "Uryyb");
    assert_ne!(blocks[0]["caption"]["id"], blocks[0]["asset"]["id"]);
    assert_eq!(
        blocks[1]["asset"]["content"]["text"],
        p["stages"][0]["blocks"][1]["content"]["text"]
    );
    assert_eq!(blocks[2]["asset"]["content"]["byteLength"], 3);
    assert_eq!(blocks[3]["note"]["text"], "Inspect the pixels.");
    assert_eq!(view["origin"]["remote"], p["origin"]);
    assert_eq!(
        view["origin"]["stages"][0]["sourceId"],
        p["stages"][0]["id"]
    );
    assert_eq!(
        view["origin"]["stages"][0]["blocks"][0]["blockId"],
        blocks[0]["id"]
    );
    let context = json!({"workspace_id":workspace["id"],"exploration_id":imported["explorationId"],"stage_id":imported["stageId"]});
    command(
        &mut manager,
        json!({"kind":"revise_text","context":context,"key":Uuid::new_v4(),"block_id":blocks[0]["id"],"asset_id":blocks[0]["caption"]["id"],"expected_revision":1,"text":"Changed caption"}),
    );
    let separate = command(&mut manager, request(&workspace, &p));
    assert_ne!(separate, imported);
    assert_eq!(
        open(&mut manager, &workspace, &separate)["stage"]["blocks"][0]["caption"]["content"]["text"],
        "Uryyb"
    );
    drop(manager);
    let mut manager = Manager::new(root).unwrap();
    command(
        &mut manager,
        json!({"kind":"open","workspace_id":workspace["id"]}),
    );
    let restored = open(&mut manager, &workspace, &imported);
    assert_eq!(restored["origin"], view["origin"]);
    assert_eq!(
        restored["stage"]["blocks"][0]["caption"]["content"]["text"],
        "Changed caption"
    );
}

#[test]
fn invalid_packages_leave_no_partial_content_or_selection_and_retry_is_safe() {
    let temporary = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(temporary.path().join("private")).unwrap();
    let workspace = command(
        &mut manager,
        json!({"kind":"create","key":Uuid::new_v4(),"title":"Packages"}),
    );
    let mut variants = Vec::new();
    let p = package();
    let mut bad = p.clone();
    bad["version"] = json!(2);
    variants.push(bad);
    variants.push(
        json!({"format_version":1,"instructions":"old","steps":[],"hints":[],"artifacts":[]}),
    );
    let mut bad = p.clone();
    bad["stages"][0]["blocks"][0]["id"] = bad["stages"][0]["id"].clone();
    variants.push(bad);
    let mut bad = p.clone();
    bad["origin"]["service_id"] = json!(Uuid::nil());
    variants.push(bad);
    let mut bad = p.clone();
    bad["stages"][0]["blocks"][0]["content"]["path"] = json!("../../private.sqlite3");
    variants.push(bad);
    let mut bad = p.clone();
    bad["pipeline"] = json!({"execute":true});
    variants.push(bad);
    let mut bad = p.clone();
    bad["stages"] = json!([]);
    variants.push(bad);
    let mut bad = p.clone();
    bad["stages"][0]["blocks"][0]["content"]["text"] = json!("é".repeat(513));
    variants.push(bad);
    // The first block has already been inserted when the corrupt second block fails.
    let mut bad = p.clone();
    bad["stages"][0]["blocks"].as_array_mut().unwrap().push(json!({"id":Uuid::new_v4(),"title":"Corrupt file","content":{"kind":"file","base64":"AAEC","sha256":"wrong"}}));
    variants.push(bad);
    for bad in variants {
        assert!(
            manager
                .handle(serde_json::from_value(request(&workspace, &bad)).unwrap())
                .is_err()
        );
        assert_eq!(
            command(
                &mut manager,
                json!({"kind":"view","workspace_id":workspace["id"]})
            ),
            workspace
        );
    }
    let mut large = request(&workspace, &p);
    large["package"] = json!(STANDARD.encode(vec![b' '; 131073]));
    assert_eq!(
        manager
            .handle(serde_json::from_value(large).unwrap())
            .unwrap_err(),
        "package_limit"
    );
    let mut duplicate = request(&workspace, &p);
    duplicate["package"] =
        json!(STANDARD.encode(b"{\"format\":\"cubearc.play.challenge\",\"format\":\"other\"}"));
    assert_eq!(
        manager
            .handle(serde_json::from_value(duplicate).unwrap())
            .unwrap_err(),
        "invalid_package"
    );
    let good = request(&workspace, &p);
    let first = command(&mut manager, good.clone());
    assert_eq!(first, command(&mut manager, good.clone()));
    let mut changed = good;
    changed["package"] = request(&workspace, &package())["package"].clone();
    assert_eq!(
        manager
            .handle(serde_json::from_value(changed).unwrap())
            .unwrap_err(),
        "idempotency_conflict"
    );
}

#[test]
fn package_budgets_and_workspace_authority_are_enforced_at_the_boundary() {
    let temporary = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(temporary.path().join("private")).unwrap();
    let workspace = command(
        &mut manager,
        json!({"kind":"create","key":Uuid::new_v4(),"title":"Limits"}),
    );
    let mut p = package();
    let mut variants = Vec::new();
    let mut bad = p.clone();
    bad["revision"] = json!(0);
    variants.push(bad);
    let mut bad = p.clone();
    bad["id"] = json!(Uuid::nil());
    variants.push(bad);
    let mut bad = p.clone();
    bad["title"] = json!(" ");
    variants.push(bad);
    let mut bad = p.clone();
    bad["stages"][0]["blocks"][0]["content"] = json!({"kind":"video","base64":""});
    variants.push(bad);
    for content in [
        json!({"kind":"file","base64":"!","sha256":"x"}),
        json!({"kind":"image","base64":"AAEC","sha256":cube_core::hashing::sha256(&[0,1,2]),"caption":""}),
        json!({"kind":"page","text":"x".repeat(1025)}),
        json!({"kind":"annotation","text":"x".repeat(2049)}),
    ] {
        let mut bad = p.clone();
        bad["stages"][0]["blocks"][0]["content"] = content;
        variants.push(bad);
    }
    while p["stages"][0]["blocks"].as_array().unwrap().len() < 7 {
        p["stages"][0]["blocks"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":Uuid::new_v4(),"title":"Text","content":{"kind":"text","text":""}}));
    }
    while p["stages"].as_array().unwrap().len() < 16 {
        p["stages"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id":Uuid::new_v4(),"title":"Stage","blocks":[]}));
    }
    let mut bad = p.clone();
    bad["stages"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":Uuid::new_v4(),"title":"Too many","blocks":[]}));
    variants.push(bad);
    let mut bad = p.clone();
    bad["stages"][0]["blocks"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":Uuid::new_v4(),"title":"Too many","content":{"kind":"text","text":""}}));
    variants.push(bad);
    for bad in variants {
        assert!(
            manager
                .handle(serde_json::from_value(request(&workspace, &bad)).unwrap())
                .is_err()
        );
        assert_eq!(
            command(
                &mut manager,
                json!({"kind":"view","workspace_id":workspace["id"]})
            ),
            workspace
        );
    }
    let mut foreign = request(&workspace, &p);
    foreign["workspace_id"] = json!(Uuid::new_v4());
    assert_eq!(
        manager
            .handle(serde_json::from_value(foreign).unwrap())
            .unwrap_err(),
        "workspace_mismatch"
    );
    for encoded in ["!".to_string(), STANDARD.encode([0xff]), "A".repeat(174765)] {
        let mut bad = request(&workspace, &p);
        bad["package"] = json!(encoded);
        assert!(
            manager
                .handle(serde_json::from_value(bad).unwrap())
                .is_err()
        );
    }
    let mut bytes = serde_json::to_vec(&p).unwrap();
    bytes.resize(131072, b' ');
    let mut at_limit = request(&workspace, &p);
    at_limit["package"] = json!(STANDARD.encode(&bytes));
    let imported = command(&mut manager, at_limit);
    let view = open(&mut manager, &workspace, &imported);
    assert_eq!(view["stages"].as_array().unwrap().len(), 16);
    assert_eq!(view["stage"]["blocks"].as_array().unwrap().len(), 8);
    assert_eq!(view["origin"]["sha256"], cube_core::hashing::sha256(&bytes));
}
