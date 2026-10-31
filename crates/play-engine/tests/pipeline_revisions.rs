#[path = "support/graph.rs"]
mod graph;
use graph::configuration;
use play_engine::workspace::{Context, Manager};
use serde_json::{Value, json};
use uuid::Uuid;
fn call(m: &mut Manager, v: Value) -> Value {
    m.handle(serde_json::from_value(v).unwrap()).unwrap()
}
fn setup(m: &mut Manager) -> Context {
    let w = call(
        m,
        json!({"kind":"create","key":Uuid::new_v4(),"title":"Pipelines"}),
    );
    let e = call(
        m,
        json!({"kind":"create_exploration","workspace_id":w["id"],"key":Uuid::new_v4(),"title":"A"}),
    );
    serde_json::from_value(
        json!({"workspace_id":w["id"],"exploration_id":e["explorationId"],"stage_id":e["stageId"]}),
    )
    .unwrap()
}
fn view(m: &mut Manager, c: &Context) -> Value {
    call(
        m,
        json!({"kind":"open_exploration","workspace_id":c.workspace_id,"exploration_id":c.exploration_id,"stage_id":c.stage_id}),
    )
}
fn text(m: &mut Manager, c: &Context, title: &str, text: &str) -> Value {
    call(
        m,
        json!({"kind":"add_text","context":c,"key":Uuid::new_v4(),"title":title,"text":text}),
    );
    view(m, c)["trails"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["source"]["title"] == title)
        .unwrap()
        .clone()
}
fn create(m: &mut Manager, c: &Context) -> Value {
    call(
        m,
        json!({"kind":"create_pipeline","workspace_id":c.workspace_id,"key":Uuid::new_v4(),"title":"Shared ROT13","definition":configuration("cipher.rot13",1,json!({}))}),
    )
}
fn save(m: &mut Manager, c: &Context, t: &Value, p: &Value, h: &str) -> Value {
    call(
        m,
        json!({"kind":"save_trail","bindings":{},"context":c,"key":Uuid::new_v4(),"trail_id":t["id"],"expected_revision":t["revision"],"hypothesis":h,"pipeline":{"id":p["pipelineId"],"revision":p["revision"]}}),
    )
}
fn run(m: &mut Manager, c: &Context, t: &Value) -> Value {
    let p = &t["pipeline"];
    let request=serde_json::from_value(json!({"context":c,"key":Uuid::new_v4(),"trail_id":t["id"],"trail_revision":t["revision"],"target_node_id":t["pipeline"]["outputNodeId"],"pipeline_id":p["id"],"pipeline_revision":p["revision"],"source_asset_id":t["source"]["id"],"source_revision":t["source"]["revision"],"operation":p["outputOperation"],"operation_version":p["operationVersion"],"retain_output":true})).unwrap();
    let prepared = m.prepare_run(&request).unwrap();
    m.start_attempt(&prepared.identity).unwrap();
    let text = cube_core::ciphers::rot13(
        serde_json::to_value(&prepared.invocation.inputs).unwrap()["source"]["text"]
            .as_str()
            .unwrap(),
    );
    m.finish_run(
        &prepared.identity,
        json!({"status":"completed","output":{"text":text}}),
    )
    .unwrap()
}
#[test]
fn trails_share_saved_revisions_without_sharing_hypotheses_or_accepted_evidence() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("private");
    let mut m = Manager::new(root.clone()).unwrap();
    let a = setup(&mut m);
    let first = text(&mut m, &a, "First", "Gur xrl.");
    let e = call(
        &mut m,
        json!({"kind":"create_exploration","workspace_id":a.workspace_id,"key":Uuid::new_v4(),"title":"B"}),
    );
    let b = Context {
        workspace_id: a.workspace_id,
        exploration_id: serde_json::from_value(e["explorationId"].clone()).unwrap(),
        stage_id: serde_json::from_value(e["stageId"].clone()).unwrap(),
    };
    let second = text(&mut m, &b, "Second", "Frpbaq pyhr.");
    let p = create(&mut m, &a);
    save(&mut m, &a, &first, &p, "Hypothesis A");
    save(&mut m, &b, &second, &p, "Hypothesis B");
    let av = view(&mut m, &a);
    let bv = view(&mut m, &b);
    let ta = &av["trails"][0];
    let tb = &bv["trails"][0];
    assert_eq!(ta["pipeline"], tb["pipeline"]);
    assert_ne!(ta["hypothesis"], tb["hypothesis"]);
    assert!(
        !ta["pipeline"]
            .to_string()
            .contains(first["source"]["id"].as_str().unwrap())
    );
    let accepted = run(&mut m, &a, ta);
    assert_eq!(accepted["result"]["outputs"]["output"], "The key.");
    let revision = call(
        &mut m,
        json!({"kind":"revise_pipeline","workspace_id":a.workspace_id,"key":Uuid::new_v4(),"pipeline_id":p["pipelineId"],"expected_revision":1,"definition":configuration("cipher.rot13",1,json!({}))}),
    );
    assert_eq!(revision["revision"], 1, "identical definitions are a no-op");
    let mut limits =
        serde_json::to_value(cube_core::esolang::brainfuck::Limits::default()).unwrap();
    limits["max_output_bytes"] = json!(16384);
    let revision = call(
        &mut m,
        json!({"kind":"revise_pipeline","workspace_id":a.workspace_id,"key":Uuid::new_v4(),"pipeline_id":p["pipelineId"],"expected_revision":1,"definition":configuration("esolang.brainfuck.execute",2,json!(limits))}),
    );
    assert_eq!(revision["revision"], 2);
    assert_eq!(view(&mut m, &a)["trails"][0]["pipeline"]["revision"], 1);
    assert_eq!(view(&mut m, &b)["trails"][0]["pipeline"]["revision"], 1);
    let old = call(
        &mut m,
        json!({"kind":"get_pipeline","workspace_id":a.workspace_id,"pipeline_id":p["pipelineId"],"revision":1}),
    );
    let new = call(
        &mut m,
        json!({"kind":"get_pipeline","workspace_id":a.workspace_id,"pipeline_id":p["pipelineId"],"revision":2}),
    );
    assert_eq!(old["outputNodeId"], new["outputNodeId"]);
    assert_eq!(old["outputOperation"], "cipher.rot13");
    assert_eq!(new["outputOperation"], "esolang.brainfuck.execute");
    assert_eq!(
        m.run_detail(&a, serde_json::from_value(accepted["id"].clone()).unwrap())
            .unwrap(),
        accepted
    );
    let before = view(&mut m, &a);
    drop(m);
    let mut m = Manager::new(root).unwrap();
    call(&mut m, json!({"kind":"open","workspace_id":a.workspace_id}));
    assert_eq!(view(&mut m, &a), before);
}
#[test]
fn pipeline_forks_pin_the_exact_source_and_invalid_edits_change_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let mut m = Manager::new(tmp.path().join("private")).unwrap();
    let c = setup(&mut m);
    let t = text(&mut m, &c, "Clue", "Gur xrl.");
    let p = create(&mut m, &c);
    let fork = call(
        &mut m,
        json!({"kind":"fork_pipeline","workspace_id":c.workspace_id,"key":Uuid::new_v4(),"pipeline_id":p["pipelineId"],"revision":1,"title":"Alternative"}),
    );
    assert_ne!(fork["pipelineId"], p["pipelineId"]);
    let f = call(
        &mut m,
        json!({"kind":"get_pipeline","workspace_id":c.workspace_id,"pipeline_id":fork["pipelineId"],"revision":1}),
    );
    assert_eq!(f["origin"]["pipelineId"], p["pipelineId"]);
    assert_eq!(f["origin"]["revision"], 1);
    let before = view(&mut m, &c);
    for value in [
        json!({"kind":"revise_pipeline","workspace_id":c.workspace_id,"key":Uuid::new_v4(),"pipeline_id":p["pipelineId"],"expected_revision":2,"definition":configuration("cipher.rot13",1,json!({}))}),
        json!({"kind":"create_pipeline","workspace_id":c.workspace_id,"key":Uuid::new_v4(),"title":"Unknown","definition":configuration("shell.execute",1,json!({}))}),
        json!({"kind":"create_pipeline","workspace_id":c.workspace_id,"key":Uuid::new_v4(),"title":"Wrong version","definition":configuration("cipher.rot13",3,json!({}))}),
    ] {
        assert!(m.handle(serde_json::from_value(value).unwrap()).is_err());
        assert_eq!(view(&mut m, &c), before);
    }
    save(&mut m, &c, &t, &p, "First saved hypothesis");
    let stale = json!({"kind":"save_trail","bindings":{},"context":c,"key":Uuid::new_v4(),"trail_id":t["id"],"expected_revision":t["revision"],"hypothesis":"Overwrite","pipeline":{"id":p["pipelineId"],"revision":1}});
    assert_eq!(
        m.handle(serde_json::from_value(stale).unwrap())
            .unwrap_err(),
        "state_conflict"
    );
    assert_eq!(
        view(&mut m, &c)["trails"][0]["hypothesis"],
        "First saved hypothesis"
    );
}

#[test]
fn saved_definitions_are_typed_bounded_scoped_and_never_implicitly_replaced_by_run() {
    let tmp = tempfile::tempdir().unwrap();
    let mut m = Manager::new(tmp.path().join("private")).unwrap();
    let c = setup(&mut m);
    let t = text(&mut m, &c, "Clue", "Gur xrl.");
    let before = view(&mut m, &c);
    let bad_parameters = [
        json!({"operation":"cipher.rot13","operation_version":1,"parameters":{"path":"/private"}}),
        json!({"operation":"image.pixels.read","operation_version":1,"parameters":{"x":-1,"y":0,"expand_palette":false}}),
        json!({"operation":"esolang.brainfuck.execute","operation_version":2,"parameters":{"max_instructions":1000001,"max_output_bytes":16384,"max_memory_bytes":8388608}}),
    ];
    for config in bad_parameters {
        let mut command = json!({"kind":"create_pipeline","workspace_id":c.workspace_id,"key":Uuid::new_v4(),"title":"Invalid"});
        command["definition"] = configuration(
            &config["operation"],
            &config["operation_version"],
            config["parameters"].clone(),
        );
        assert!(m.handle(serde_json::from_value(command).unwrap()).is_err());
        assert_eq!(view(&mut m, &c), before);
    }
    let image = call(
        &mut m,
        json!({"kind":"create_pipeline","workspace_id":c.workspace_id,"key":Uuid::new_v4(),"title":"Image pipeline","definition":configuration("image.pixels.read",1,json!({"x":1,"y":0,"expand_palette":false}))}),
    );
    let command = json!({"kind":"save_trail","bindings":{},"context":c,"key":Uuid::new_v4(),"trail_id":t["id"],"expected_revision":t["revision"],"hypothesis":"Wrong type","pipeline":{"id":image["pipelineId"],"revision":1}});
    assert_eq!(
        m.handle(serde_json::from_value(command).unwrap())
            .unwrap_err(),
        "invalid_asset_type"
    );
    let command = json!({"kind":"save_trail","bindings":{},"context":c,"key":Uuid::new_v4(),"trail_id":t["id"],"expected_revision":t["revision"],"hypothesis":"x".repeat(2049),"pipeline":null});
    assert_eq!(
        m.handle(serde_json::from_value(command).unwrap())
            .unwrap_err(),
        "input_limit"
    );
    let p = &t["pipeline"];
    let request=serde_json::from_value(json!({"context":c,"key":Uuid::new_v4(),"trail_id":t["id"],"trail_revision":t["revision"],"target_node_id":t["pipeline"]["outputNodeId"],"pipeline_id":p["id"],"pipeline_revision":p["revision"],"source_asset_id":t["source"]["id"],"source_revision":t["source"]["revision"],"operation":"esolang.brainfuck.execute","operation_version":2,"retain_output":true})).unwrap();
    assert_eq!(m.prepare_run(&request).err().unwrap(), "pipeline_mismatch");
    assert_eq!(view(&mut m, &c)["trails"], before["trails"]);
    let accepted = create(&mut m, &c);
    let mut wrong = c.clone();
    wrong.exploration_id = Uuid::new_v4();
    let command = json!({"kind":"save_trail","bindings":{},"context":wrong,"key":Uuid::new_v4(),"trail_id":t["id"],"expected_revision":t["revision"],"hypothesis":"Wrong context","pipeline":{"id":accepted["pipelineId"],"revision":1}});
    assert_eq!(
        m.handle(serde_json::from_value(command).unwrap())
            .unwrap_err(),
        "not_found"
    );
}

#[test]
fn revising_a_default_does_not_change_later_asset_defaults_and_limits_preserve_saved_work() {
    let tmp = tempfile::tempdir().unwrap();
    let mut m = Manager::new(tmp.path().join("private")).unwrap();
    let c = setup(&mut m);
    let first = text(&mut m, &c, "Initial", "Gur xrl.");
    let p = &first["pipeline"];
    let mut params =
        serde_json::to_value(cube_core::esolang::brainfuck::Limits::default()).unwrap();
    params["max_output_bytes"] = json!(16384);
    call(
        &mut m,
        json!({"kind":"revise_pipeline","workspace_id":c.workspace_id,"key":Uuid::new_v4(),"pipeline_id":p["id"],"expected_revision":1,"definition":configuration("esolang.brainfuck.execute",2,json!(params))}),
    );
    let second = text(&mut m, &c, "Later", "Frpbaq.");
    assert_eq!(second["pipeline"]["id"], p["id"]);
    assert_eq!(second["pipeline"]["revision"], 1);
    assert_eq!(second["pipeline"]["outputOperation"], "cipher.rot13");
    assert_eq!(
        run(&mut m, &c, &second)["result"]["outputs"]["output"],
        "Second."
    );
    let intent = json!({"kind":"create_pipeline","workspace_id":c.workspace_id,"key":Uuid::new_v4(),"title":"Pixel sampling","definition":configuration("image.pixels.read",1,json!({"x":0,"y":0,"expand_palette":false}))});
    let image = call(&mut m, intent.clone());
    assert_eq!(call(&mut m, intent), image);
    for revision in 1..32 {
        let result = call(
            &mut m,
            json!({"kind":"revise_pipeline","workspace_id":c.workspace_id,"key":Uuid::new_v4(),"pipeline_id":image["pipelineId"],"expected_revision":revision,"definition":configuration("image.pixels.read",1,json!({"x":revision,"y":0,"expand_palette":false}))}),
        );
        assert_eq!(result["revision"], revision + 1);
    }
    let before = view(&mut m, &c);
    let overflow = json!({"kind":"revise_pipeline","workspace_id":c.workspace_id,"key":Uuid::new_v4(),"pipeline_id":image["pipelineId"],"expected_revision":32,"definition":configuration("image.pixels.read",1,json!({"x":32,"y":0,"expand_palette":false}))});
    assert_eq!(
        m.handle(serde_json::from_value(overflow).unwrap())
            .unwrap_err(),
        "storage_quota"
    );
    assert_eq!(view(&mut m, &c), before);
    for _ in 2..32 {
        create(&mut m, &c);
    }
    let before = view(&mut m, &c);
    let overflow = json!({"kind":"create_pipeline","workspace_id":c.workspace_id,"key":Uuid::new_v4(),"title":"One too many","definition":configuration("cipher.rot13",1,json!({}))});
    assert_eq!(
        m.handle(serde_json::from_value(overflow).unwrap())
            .unwrap_err(),
        "storage_quota"
    );
    assert_eq!(view(&mut m, &c), before);
}
