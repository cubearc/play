use play_engine::workspace::{Command, Manager};
use serde_json::{Value, json};
use uuid::Uuid;
fn call(m: &mut Manager, mut v: Value) -> Value {
    if v.get("key").is_none()
        && !matches!(
            v["kind"].as_str(),
            Some("open" | "open_exploration" | "get_candidate" | "get_solution" | "list_solutions")
        )
    {
        v["key"] = json!(Uuid::new_v4());
    }
    m.handle(serde_json::from_value::<Command>(v).unwrap())
        .unwrap()
}
#[test]
fn stage_order_progress_archive_and_conflicts_are_scoped_and_survive_restart() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("private");
    let mut m = Manager::new(root.clone()).unwrap();
    let w = call(&mut m, json!({"kind":"create","title":"Stages"}))["id"].clone();
    let a = call(
        &mut m,
        json!({"kind":"create_exploration","workspace_id":w,"title":"A"}),
    );
    let b = call(
        &mut m,
        json!({"kind":"create_exploration","workspace_id":w,"title":"B"}),
    );
    let c = json!({"workspace_id":w,"exploration_id":a["explorationId"],"stage_id":a["stageId"]});
    let edit = json!({"kind":"edit_stage","context":c,"key":Uuid::new_v4(),"expected_revision":1,"title":"Decode image","progress":"completed","archived":false});
    assert_eq!(call(&mut m, edit.clone())["revision"], 2);
    assert_eq!(
        call(&mut m, edit.clone())["revision"],
        2,
        "retry is idempotent"
    );
    let mut stale = edit.clone();
    stale["key"] = json!(Uuid::new_v4());
    assert_eq!(
        m.handle(serde_json::from_value(stale).unwrap()),
        Err("state_conflict")
    );
    let second = call(
        &mut m,
        json!({"kind":"create_stage","workspace_id":w,"exploration_id":a["explorationId"],"title":"Compare"}),
    );
    let open = json!({"kind":"open_exploration","workspace_id":w,"exploration_id":a["explorationId"],"stage_id":a["stageId"]});
    let view = call(&mut m, open.clone());
    call(
        &mut m,
        json!({"kind":"reorder_stages","workspace_id":w,"exploration_id":a["explorationId"],"expected_revision":view["revision"],"stage_ids":[second["stageId"],a["stageId"]]}),
    );
    let mut archive = edit;
    archive["expected_revision"] = json!(2);
    archive["key"] = json!(Uuid::new_v4());
    archive["archived"] = json!(true);
    call(&mut m, archive);
    let v = call(&mut m, open.clone());
    assert_eq!(v["stage"]["progress"], "completed");
    assert_eq!(v["stage"]["archived"], true);
    assert_eq!(v["stages"][0]["id"], second["stageId"]);
    assert_eq!(v["archivedStages"][0]["id"], a["stageId"]);
    let add = json!({"kind":"add_text","context":c,"key":Uuid::new_v4(),"title":"Blocked","text":"private"});
    assert_eq!(
        m.handle(serde_json::from_value(add).unwrap()),
        Err("stage_archived")
    );
    let foreign = json!({"kind":"edit_stage","context":{"workspace_id":w,"exploration_id":b["explorationId"],"stage_id":a["stageId"]},"key":Uuid::new_v4(),"expected_revision":3,"title":"Wrong","progress":"not_started","archived":false});
    assert_eq!(
        m.handle(serde_json::from_value(foreign).unwrap()),
        Err("not_found")
    );
    drop(m);
    let mut m = Manager::new(root).unwrap();
    call(&mut m, json!({"kind":"open","workspace_id":w}));
    assert_eq!(call(&mut m, open)["stage"]["progress"], "completed");
    let bv = call(
        &mut m,
        json!({"kind":"open_exploration","workspace_id":w,"exploration_id":b["explorationId"],"stage_id":null}),
    );
    assert_eq!(bv["stage"]["progress"], "not_started");
    assert_eq!(bv["stage"]["archived"], false);
}

#[test]
fn duplicate_stage_forks_visible_content_without_copying_answers_or_completion() {
    let temp = tempfile::tempdir().unwrap();
    let mut m = Manager::new(temp.path().join("private")).unwrap();
    let w = call(&mut m, json!({"kind":"create","title":"Copies"}))["id"].clone();
    let a = call(
        &mut m,
        json!({"kind":"create_exploration","workspace_id":w,"title":"Working"}),
    );
    let c = json!({"workspace_id":w,"exploration_id":a["explorationId"],"stage_id":a["stageId"]});
    call(
        &mut m,
        json!({"kind":"add_text","context":c,"title":"Encoded","text":"Uryyb"}),
    );
    let mut v = call(
        &mut m,
        json!({"kind":"open_exploration","workspace_id":w,"exploration_id":a["explorationId"],"stage_id":null}),
    );
    let reasoning = v["stage"]["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["kind"] == "note")
        .unwrap()
        .clone();
    call(
        &mut m,
        json!({"kind":"save_note","context":c,"note_id":reasoning["note"]["id"],"expected_revision":1,"text":"Copy this hypothesis, keep its source revision."}),
    );
    let mut ids = v["stage"]["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|b| b["id"].clone())
        .collect::<Vec<_>>();
    ids.sort_by_key(|id| if *id == reasoning["id"] { 0 } else { 1 });
    call(
        &mut m,
        json!({"kind":"reorder_blocks","context":c,"expected_revision":v["stage"]["revision"],"block_ids":ids}),
    );
    v = call(
        &mut m,
        json!({"kind":"open_exploration","workspace_id":w,"exploration_id":a["explorationId"],"stage_id":a["stageId"]}),
    );
    let source_asset = v["trails"][0]["source"]["id"].clone();
    call(
        &mut m,
        json!({"kind":"select_candidate","context":c,"expected_revision":0,"selection":{"kind":"asset","asset_id":source_asset,"revision":1}}),
    );
    let solution = Uuid::new_v4();
    call(
        &mut m,
        json!({"kind":"save_solution","context":c,"solution_id":solution,"expected_revision":0,"candidate_revision":1,"notes":[],"title":"Source answer","explanation":"Selected source clue."}),
    );
    let edited = call(
        &mut m,
        json!({"kind":"edit_stage","context":c,"expected_revision":v["stage"]["revision"],"title":"Source","progress":"completed","archived":false}),
    );
    v["stage"]["revision"] = edited["revision"].clone();
    let copy = call(
        &mut m,
        json!({"kind":"duplicate_stage","context":c,"expected_revision":v["stage"]["revision"],"title":"Alternative"}),
    );
    let copied = call(
        &mut m,
        json!({"kind":"open_exploration","workspace_id":w,"exploration_id":a["explorationId"],"stage_id":copy["stageId"]}),
    );
    assert_ne!(copied["stage"]["id"], v["stage"]["id"]);
    assert_eq!(
        copied["stage"]["blocks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| b["kind"].clone())
            .collect::<Vec<_>>(),
        v["stage"]["blocks"]
            .as_array()
            .unwrap()
            .iter()
            .map(|b| b["kind"].clone())
            .collect::<Vec<_>>()
    );
    let copied_note = &copied["stage"]["blocks"][0]["note"];
    assert_eq!(
        copied_note["text"],
        "Copy this hypothesis, keep its source revision."
    );
    assert_eq!(copied_note["origin"]["noteId"], reasoning["note"]["id"]);
    assert_eq!(copied_note["origin"]["revision"], 2);
    let asset = |v: &Value| {
        v["stage"]["blocks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|b| b["kind"] == "asset")
            .unwrap()["asset"]
            .clone()
    };
    let original = asset(&v);
    let fork = asset(&copied);
    assert_ne!(original["id"], fork["id"]);
    assert_eq!(original["content"], fork["content"]);
    assert_eq!(fork["origin"]["assetId"], original["id"]);
    assert_eq!(copied["stage"]["progress"], "not_started");
    let cc =
        json!({"workspace_id":w,"exploration_id":a["explorationId"],"stage_id":copy["stageId"]});
    assert!(
        call(
            &mut m,
            json!({"kind":"get_candidate","context":cc,"revision":null})
        ) == json!({"candidate":null})
    );
    assert_eq!(
        call(&mut m, json!({"kind":"list_solutions","context":cc}))["solutions"],
        json!([])
    );
    assert_eq!(
        call(
            &mut m,
            json!({"kind":"get_candidate","context":c,"revision":null})
        )["candidate"]["selection"]["asset_id"],
        source_asset
    );
    call(
        &mut m,
        json!({"kind":"edit_stage","context":c,"expected_revision":v["stage"]["revision"],"title":"Source","progress":"completed","archived":true}),
    );
    assert_eq!(
        call(
            &mut m,
            json!({"kind":"get_solution","context":c,"solution_id":solution,"revision":1})
        )["title"],
        "Source answer"
    );
}

#[test]
fn unnamed_stage_copies_have_unique_utf8_titles_within_the_domain_limit() {
    let root = tempfile::tempdir().unwrap();
    let mut manager = Manager::new(root.path().join("private")).unwrap();
    let w = call(&mut manager, json!({"kind":"create","title":"Names"}))["id"].clone();
    let source = call(
        &mut manager,
        json!({"kind":"create_exploration","workspace_id":w,"title":"A"}),
    );
    let context = json!({"workspace_id":w,"exploration_id":source["explorationId"],"stage_id":source["stageId"]});
    let title = "🧩".repeat(30);
    call(
        &mut manager,
        json!({"kind":"edit_stage","context":context,"expected_revision":1,"title":title,"progress":"not_started","archived":false}),
    );
    let mut titles = Vec::new();
    for _ in 0..2 {
        let copy = call(
            &mut manager,
            json!({"kind":"duplicate_stage","context":context,"expected_revision":2,"title":null}),
        );
        let view = call(
            &mut manager,
            json!({"kind":"open_exploration","workspace_id":w,"exploration_id":source["explorationId"],"stage_id":copy["stageId"]}),
        );
        titles.push(view["stage"]["title"].as_str().unwrap().to_owned());
    }
    assert_ne!(titles[0], titles[1]);
    assert!(titles.iter().all(|s| s.len() <= 120 && s.starts_with('🧩')));
    assert!(titles[0].ends_with(" (copy)"));
    assert!(titles[1].ends_with(" (copy 2)"));
}
