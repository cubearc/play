use play_engine::workspace::{Context, Manager, RunRequest};
use serde_json::{Value, json};
use uuid::Uuid;
fn call(m: &mut Manager, v: Value) -> Value {
    m.handle(serde_json::from_value(v).unwrap()).unwrap()
}
fn context(m: &mut Manager, workspace: Option<Uuid>, title: &str) -> Context {
    let w = workspace.unwrap_or_else(|| {
        serde_json::from_value(
            call(
                m,
                json!({"kind":"create","key":Uuid::new_v4(),"title":"Asset pipelines"}),
            )["id"]
                .clone(),
        )
        .unwrap()
    });
    let e = call(
        m,
        json!({"kind":"create_exploration","workspace_id":w,"key":Uuid::new_v4(),"title":title}),
    );
    serde_json::from_value(
        json!({"workspace_id":w,"exploration_id":e["explorationId"],"stage_id":e["stageId"]}),
    )
    .unwrap()
}
fn view(m: &mut Manager, c: &Context) -> Value {
    call(
        m,
        json!({"kind":"open_exploration","workspace_id":c.workspace_id,"exploration_id":c.exploration_id,"stage_id":c.stage_id}),
    )
}
fn image(m: &mut Manager, c: &Context, caption: &str) -> Value {
    image_from(
        m,
        c,
        caption,
        include_bytes!("../../../tests/fixtures/sample.png"),
    )
}
fn image_from(m: &mut Manager, c: &Context, caption: &str, png: &[u8]) -> Value {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    call(
        m,
        json!({"kind":"add_image","context":c,"key":Uuid::new_v4(),"title":"Puzzle image","caption":caption,"png":STANDARD.encode(png)}),
    );
    view(m, c)
}
fn definition() -> Value {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    json!({"formatVersion":4,"inputs":[{"name":"source","semanticType":"image.samples"},{"name":"caption","semanticType":"text"}],"nodes":[{"id":a,"operation":"image.pixels.read","operationVersion":1,"parameters":{"x":0,"y":0,"expand_palette":false},"position":{"x":80,"y":80},"inputs":{"input":{"kind":"input","name":"source"}}},{"id":b,"operation":"cipher.rot13","operationVersion":1,"parameters":{},"position":{"x":400,"y":80},"inputs":{"input":{"kind":"input","name":"caption"}}}],"output":b})
}

fn numeric_definition(pointer: &str) -> Value {
    let ids: [Uuid; 5] = std::array::from_fn(|_| Uuid::new_v4());
    let input = |name: &str| json!({"kind":"input","name":name});
    let output = |index: usize| json!({"kind":"output","nodeId":ids[index],"port":"output"});
    let nodes=vec![
        ("image.dimensions",json!({"input":input("source")}),json!({})),
        ("json.select",json!({"input":output(0)}),json!({"pointer":pointer})),
        ("number.maximum",json!({"input":output(1)}),json!({})),
        ("text.integer",json!({"input":input("caption")}),json!({})),
        ("number.add",json!({"left":output(2),"right":output(3)}),json!({})),
    ].into_iter().enumerate().map(|(i,(operation,inputs,parameters))|json!({"id":ids[i],"operation":operation,"operationVersion":1,"inputs":inputs,"parameters":parameters,"position":{"x":40+(i%3)*300,"y":40+(i/3)*180}})).collect::<Vec<_>>();
    json!({"formatVersion":4,"inputs":[{"name":"source","semanticType":"image.samples"},{"name":"caption","semanticType":"text"}],"nodes":nodes,"output":ids[4]})
}

#[test]
fn numeric_field_graph_preserves_actual_types_attempts_and_snapshot_after_restart() {
    let root = tempfile::tempdir().unwrap();
    let mut m = Manager::new(root.path().join("private")).unwrap();
    let c = context(&mut m, None, "Numeric facts");
    let v = image_from(
        &mut m,
        &c,
        "7",
        include_bytes!("../../../tests/fixtures/composition.png"),
    );
    let d = numeric_definition("/dimensions");
    let p = pipeline(&mut m, &c, &d);
    let v = bind(&mut m, &c, &v, &p, "Use exact dimensions and score");
    let t = trail(&v, &figure(&v)["asset"]);
    let r = request(&c, t, &d["nodes"][4]);
    let completed = execute(&mut m, &r);
    assert_eq!(completed["result"]["outputs"]["output"], json!(8));
    assert_eq!(completed["attempts"].as_array().unwrap().len(), 5);
    assert!(
        completed["attempts"]
            .as_array()
            .unwrap()
            .iter()
            .all(|a| a["status"] == "completed")
    );
    assert!(completed["plan"].to_string().len() <= 16384);
    let run_id = serde_json::from_value(completed["id"].clone()).unwrap();
    drop(m);
    let mut m = Manager::new(root.path().join("private")).unwrap();
    call(&mut m, json!({"kind":"open","workspace_id":c.workspace_id}));
    assert_eq!(m.run_detail(&c, run_id).unwrap(), completed);
}

#[test]
fn numeric_failures_preserve_completed_values_and_stop_consumers() {
    for (caption, pointer, code, attempt_count) in [
        ("7", "/missing", "missing_field", 2),
        ("9007199254740991", "/dimensions", "numeric_overflow", 5),
        ("7", "/dimensions/0", "contract_mismatch", 2),
    ] {
        let root = tempfile::tempdir().unwrap();
        let mut m = Manager::new(root.path().join("private")).unwrap();
        let c = context(&mut m, None, "Failed numeric path");
        let v = image_from(
            &mut m,
            &c,
            caption,
            include_bytes!("../../../tests/fixtures/composition.png"),
        );
        let d = numeric_definition(pointer);
        let p = pipeline(&mut m, &c, &d);
        let v = bind(&mut m, &c, &v, &p, "Inspect numeric evidence");
        let run = execute_outcome(
            &mut m,
            &request(&c, trail(&v, &figure(&v)["asset"]), &d["nodes"][4]),
        );
        assert_eq!(run["result"], json!({"status":"failed","code":code}));
        let attempts = run["attempts"].as_array().unwrap();
        assert_eq!(attempts.len(), attempt_count);
        assert_eq!(
            attempts[0]["result"]["outputs"]["output"],
            json!({"dimensions":[1,1]})
        );
        let id = serde_json::from_value(run["id"].clone()).unwrap();
        drop(m);
        let mut m = Manager::new(root.path().join("private")).unwrap();
        call(&mut m, json!({"kind":"open","workspace_id":c.workspace_id}));
        assert_eq!(m.run_detail(&c, id).unwrap(), run);
    }
}
fn pipeline(m: &mut Manager, c: &Context, d: &Value) -> Value {
    call(
        m,
        json!({"kind":"create_pipeline","workspace_id":c.workspace_id,"key":Uuid::new_v4(),"title":"Image and caption","definition":d}),
    )
}
fn figure(v: &Value) -> &Value {
    v["stage"]["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["kind"] == "asset" && b["asset"]["content"]["kind"] == "image")
        .unwrap()
}
fn trail<'a>(v: &'a Value, asset: &Value) -> &'a Value {
    v["trails"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["source"]["id"] == asset["id"])
        .unwrap()
}
fn bind(m: &mut Manager, c: &Context, v: &Value, p: &Value, h: &str) -> Value {
    let f = figure(v);
    let t = trail(v, &f["asset"]);
    call(
        m,
        json!({"kind":"save_trail","context":c,"key":Uuid::new_v4(),"trail_id":t["id"],"expected_revision":t["revision"],"hypothesis":h,"pipeline":{"id":p["pipelineId"],"revision":p["revision"]},"bindings":{"caption":{"asset_id":f["caption"]["id"],"revision":f["caption"]["revision"]}}}),
    );
    view(m, c)
}
fn request(c: &Context, t: &Value, target: &Value) -> RunRequest {
    serde_json::from_value(json!({"context":c,"key":Uuid::new_v4(),"trail_id":t["id"],"trail_revision":t["revision"],"pipeline_id":t["pipeline"]["id"],"pipeline_revision":t["pipeline"]["revision"],"source_asset_id":t["source"]["id"],"source_revision":t["source"]["revision"],"target_node_id":target["id"],"operation":target["operation"],"operation_version":target["operationVersion"],"retain_output":true})).unwrap()
}
fn execute(m: &mut Manager, r: &RunRequest) -> Value {
    let result = execute_outcome(m, r);
    assert_eq!(result["status"], "completed");
    result
}
fn execute_outcome(m: &mut Manager, r: &RunRequest) -> Value {
    let p = m.prepare_run(r).unwrap();
    let id = p.identity.clone();
    let output = play_engine::worker::execute_pipeline(p.invocation, |event| {
        let mut n = id.clone();
        n.node_id = serde_json::from_value(event["nodeId"].clone()).unwrap();
        if event["type"] == "node_started" {
            m.start_attempt(&n)
        } else {
            m.finish_attempt(&n, event)
        }
    });
    let result = match output {
        Ok(output) => json!({"status":"completed","output":output}),
        Err(code) => json!({"status":"failed","code":code}),
    };
    m.finish_run(&id, result).unwrap()
}

#[test]
fn decoder_failure_is_durable_without_losing_completed_producer_evidence() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("private");
    let mut m = Manager::new(root.clone()).unwrap();
    let c = context(&mut m, None, "Keep the failed path");
    let v = image(&mut m, &c, "Unused caption");
    let mut d = definition();
    let a = d["nodes"][0]["id"].clone();
    let b = d["nodes"][1]["id"].clone();
    let end = Uuid::new_v4();
    d["nodes"] = json!([
        {"id":a,"operation":"bytes.concat","operationVersion":1,"parameters":{},"position":{"x":40,"y":40},"inputs":{"left":{"kind":"literal","value":{"base64":"/wA="}},"right":{"kind":"literal","value":{"base64":"gA=="}}}},
        {"id":b,"operation":"text.utf8.decode","operationVersion":1,"parameters":{},"position":{"x":340,"y":40},"inputs":{"input":{"kind":"output","nodeId":a,"port":"output"}}},
        {"id":end,"operation":"text.concat","operationVersion":1,"parameters":{},"position":{"x":640,"y":40},"inputs":{"left":{"kind":"output","nodeId":b,"port":"output"},"right":{"kind":"literal","value":"not reached"}}}
    ]);
    d["output"] = json!(end);
    let p = pipeline(&mut m, &c, &d);
    let v = bind(&mut m, &c, &v, &p, "Inspect the bytes");
    let t = trail(&v, &figure(&v)["asset"]);
    let run = execute_outcome(&mut m, &request(&c, t, &d["nodes"][2]));
    assert_eq!(
        run["result"],
        json!({"status":"failed","code":"invalid_encoding"})
    );
    let attempts = run["attempts"].as_array().unwrap();
    assert_eq!(attempts.len(), 2);
    assert_eq!(attempts[0]["status"], "completed");
    assert_eq!(attempts[1]["status"], "failed");
    assert_eq!(attempts[0]["result"]["outputs"]["output"]["base64"], "/wCA");
    drop(m);
    let mut m = Manager::new(root).unwrap();
    call(&mut m, json!({"kind":"open","workspace_id":c.workspace_id}));
    assert_eq!(
        m.run_detail(&c, serde_json::from_value(run["id"].clone()).unwrap())
            .unwrap(),
        run
    );
}
#[test]
fn two_explorations_bind_the_same_graph_to_distinct_image_caption_revisions_and_targets() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("private");
    let mut m = Manager::new(root.clone()).unwrap();
    let a = context(&mut m, None, "A");
    let av = image(&mut m, &a, "Gur xrl.");
    let b = context(&mut m, Some(a.workspace_id), "B");
    let bv = image(&mut m, &b, "Frpbaq pyhr.");
    let d = definition();
    let p = pipeline(&mut m, &a, &d);
    let av = bind(&mut m, &a, &av, &p, "Follow the image");
    let bv = bind(&mut m, &b, &bv, &p, "Follow the caption");
    let at = trail(&av, &figure(&av)["asset"]);
    let bt = trail(&bv, &figure(&bv)["asset"]);
    assert_eq!(at["pipeline"], bt["pipeline"]);
    assert_ne!(at["hypothesis"], bt["hypothesis"]);
    assert_ne!(
        at["bindings"]["caption"]["id"],
        bt["bindings"]["caption"]["id"]
    );
    assert!(!d.to_string().contains(at["source"]["id"].as_str().unwrap()));
    let ar = execute(&mut m, &request(&a, at, &d["nodes"][1]));
    let br = execute(&mut m, &request(&b, bt, &d["nodes"][1]));
    let pixels = execute(&mut m, &request(&a, at, &d["nodes"][0]));
    assert_eq!(ar["result"]["outputs"]["output"], "The key.");
    assert_eq!(br["result"]["outputs"]["output"], "Second clue.");
    assert_eq!(pixels["result"]["outputs"]["output"]["base64"], "FFAy");
    assert_eq!(ar["plan"]["inputs"]["source"]["used"], false);
    assert_eq!(ar["plan"]["inputs"]["caption"]["used"], true);
    assert_eq!(pixels["plan"]["inputs"]["caption"]["used"], false);
    assert_eq!(ar["plan"]["pipeline"]["definition"], d);
    assert_eq!(ar["attempts"].as_array().unwrap().len(), 1);
    drop(m);
    let mut m = Manager::new(root).unwrap();
    call(&mut m, json!({"kind":"open","workspace_id":a.workspace_id}));
    assert_eq!(
        m.run_detail(&a, serde_json::from_value(ar["id"].clone()).unwrap())
            .unwrap(),
        ar
    );
    assert_eq!(
        m.run_detail(&b, serde_json::from_value(br["id"].clone()).unwrap())
            .unwrap(),
        br
    );
}

#[test]
fn image_bytes_and_caption_join_in_explicit_order_and_preserve_every_actual_attempt() {
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("private");
    let mut m = Manager::new(root.clone()).unwrap();
    let c = context(&mut m, None, "Join the clues");
    let v = image_from(
        &mut m,
        &c,
        "xrl.",
        include_bytes!("../../../tests/fixtures/composition.png"),
    );
    let mut d = definition();
    let pixel = d["nodes"][0]["id"].clone();
    let caption = d["nodes"][1]["id"].clone();
    let decode = Uuid::new_v4();
    let rotate = Uuid::new_v4();
    let join = Uuid::new_v4();
    d["nodes"].as_array_mut().unwrap().extend([
        json!({"id":decode,"operation":"text.utf8.decode","operationVersion":1,"parameters":{},"position":{"x":300,"y":80},"inputs":{"input":{"kind":"output","nodeId":pixel,"port":"output"}}}),
        json!({"id":rotate,"operation":"cipher.rot13","operationVersion":1,"parameters":{},"position":{"x":600,"y":80},"inputs":{"input":{"kind":"output","nodeId":decode,"port":"output"}}}),
        json!({"id":join,"operation":"text.concat","operationVersion":1,"parameters":{"separator":" "},"position":{"x":900,"y":160},"inputs":{"left":{"kind":"output","nodeId":rotate,"port":"output"},"right":{"kind":"output","nodeId":caption,"port":"output"}}}),
    ]);
    d["output"] = json!(join);
    let p = pipeline(&mut m, &c, &d);
    let v = bind(&mut m, &c, &v, &p, "The image supplies the first word");
    let t = trail(&v, &figure(&v)["asset"]);
    let run = execute(&mut m, &request(&c, t, &d["nodes"][4]));
    assert_eq!(run["result"]["outputs"]["output"], "The key.");
    assert_eq!(run["attempts"].as_array().unwrap().len(), 5);
    assert_eq!(run["plan"]["inputs"]["source"]["used"], true);
    assert_eq!(run["plan"]["inputs"]["caption"]["used"], true);
    assert_eq!(run["plan"]["pipeline"]["definition"], d);
    assert_eq!(run["plan"]["formatVersion"], 3);
    assert!(run["plan"].to_string().len() <= 16384);
    assert_eq!(run["plan"]["contracts"].as_object().unwrap().len(), 3);
    for node in run["plan"]["nodes"].as_array().unwrap() {
        assert!(node.get("contract").is_none());
        let contract = if node["contractDigest"] == run["plan"]["contractDigest"] {
            &run["plan"]["contract"]
        } else {
            &run["plan"]["contracts"][node["contractDigest"].as_str().unwrap()]
        };
        assert_eq!(contract["operation"], node["operation"]["operation"]);
        let installed =
            cube_core::operations::require(node["operation"]["operation"].as_str().unwrap())
                .unwrap()
                .contract()
                .unwrap();
        assert_eq!(
            node["contractDigest"],
            cube_core::hashing::sha256(&serde_json::to_vec(&installed).unwrap())
        );
        assert_eq!(*contract, serde_json::to_value(installed).unwrap());
    }
    let attempts = run["attempts"].as_array().unwrap();
    assert!(attempts.iter().all(|a| a["status"] == "completed"));
    drop(m);
    let mut m = Manager::new(root).unwrap();
    call(&mut m, json!({"kind":"open","workspace_id":c.workspace_id}));
    assert_eq!(
        m.run_detail(&c, serde_json::from_value(run["id"].clone()).unwrap())
            .unwrap(),
        run
    );
}
fn save(c: &Context, t: &Value, p: &Value, bindings: Value) -> Value {
    json!({"kind":"save_trail","context":c,"key":Uuid::new_v4(),"trail_id":t["id"],"expected_revision":t["revision"],"hypothesis":"Inspect both inputs","pipeline":{"id":p["pipelineId"],"revision":p["revision"]},"bindings":bindings})
}
fn ref_of(asset: &Value) -> Value {
    json!({"asset_id":asset["id"],"revision":asset["revision"]})
}
fn duplicate(m: &mut Manager, c: &Context, v: &Value) -> Value {
    let f = figure(v);
    call(
        m,
        json!({"kind":"duplicate_block","context":c,"key":Uuid::new_v4(),"block_id":f["id"],"expected_revision":v["stage"]["revision"],"source":{"kind":"asset","asset":ref_of(&f["asset"]),"caption":ref_of(&f["caption"])}}),
    );
    view(m, c)
}
#[test]
fn pinning_rebinding_and_copying_keep_exact_revision_identity() {
    let tmp = tempfile::tempdir().unwrap();
    let mut m = Manager::new(tmp.path().join("private")).unwrap();
    let c = context(&mut m, None, "Pinned caption");
    let v = image(&mut m, &c, "Gur xrl.");
    let d = definition();
    let p = pipeline(&mut m, &c, &d);
    let v = bind(&mut m, &c, &v, &p, "Keep original");
    let t = trail(&v, &figure(&v)["asset"]);
    let old_request = request(&c, t, &d["nodes"][1]);
    let old = execute(&mut m, &old_request);
    let copied = duplicate(&mut m, &c, &v);
    let copied_figure = copied["stage"]["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["asset"]["origin"]["assetId"] == figure(&v)["asset"]["id"])
        .unwrap();
    let ct = trail(&copied, &copied_figure["asset"]);
    assert_eq!(
        ct["bindings"]["caption"]["id"],
        copied_figure["caption"]["id"]
    );
    assert_eq!(ct["bindings"]["caption"]["revision"], 1);
    let f = figure(&v);
    call(
        &mut m,
        json!({"kind":"revise_text","context":c,"key":Uuid::new_v4(),"block_id":f["id"],"asset_id":f["caption"]["id"],"expected_revision":1,"text":"Frpbaq pyhr."}),
    );
    let edited = view(&mut m, &c);
    let t = trail(&edited, &f["asset"]);
    assert_eq!(t["bindings"]["caption"]["revision"], 1);
    assert_eq!(
        execute(&mut m, &request(&c, t, &d["nodes"][1]))["result"]["outputs"]["output"],
        "The key."
    );
    let copied_old = duplicate(&mut m, &c, &edited);
    let ct = copied_old["trails"]
        .as_array()
        .unwrap()
        .iter()
        .rev()
        .find(|t| t["source"]["content"]["kind"] == "image")
        .unwrap();
    assert_eq!(ct["bindings"]["caption"]["id"], f["caption"]["id"]);
    assert_eq!(
        ct["bindings"]["caption"]["revision"], 1,
        "copying r2 must not replace a pin to r1"
    );
    let stale = request(&c, t, &d["nodes"][1]);
    let latest = figure(&edited);
    call(
        &mut m,
        save(&c, t, &p, json!({"caption":ref_of(&latest["caption"])})),
    );
    assert_eq!(m.prepare_run(&stale).err(), Some("state_conflict"));
    assert_eq!(m.existing_run(&old_request).unwrap().unwrap(), old);
    let bound = view(&mut m, &c);
    let t = trail(&bound, &f["asset"]);
    assert_eq!(
        execute(&mut m, &request(&c, t, &d["nodes"][1]))["result"]["outputs"]["output"],
        "Second clue."
    );
}
#[test]
fn scoped_bindings_and_targets_are_validated_before_any_run_is_queued() {
    let tmp = tempfile::tempdir().unwrap();
    let mut m = Manager::new(tmp.path().join("private")).unwrap();
    let a = context(&mut m, None, "A");
    let av = image(&mut m, &a, "Gur xrl.");
    let b = context(&mut m, Some(a.workspace_id), "B");
    let bv = image(&mut m, &b, "Secret");
    let d = definition();
    let p = pipeline(&mut m, &a, &d);
    let f = figure(&av);
    let t = trail(&av, &f["asset"]);
    for bindings in [
        json!({}),
        json!({"caption":ref_of(&figure(&bv)["caption"])}),
        json!({"caption":ref_of(&f["asset"])}),
        json!({"caption":{"asset_id":f["caption"]["id"],"revision":9}}),
        json!({"source":ref_of(&f["asset"]),"caption":ref_of(&f["caption"])}),
    ] {
        assert!(
            m.handle(serde_json::from_value(save(&a, t, &p, bindings)).unwrap())
                .is_err()
        );
        assert_eq!(view(&mut m, &a), av);
    }
    let bound = bind(&mut m, &a, &av, &p, "Valid");
    let t = trail(&bound, &f["asset"]);
    let mut r = request(&a, t, &d["nodes"][1]);
    r.target_node_id = Uuid::new_v4();
    assert_eq!(m.prepare_run(&r).err(), Some("invalid_connection"));
    let mut r = request(&a, t, &d["nodes"][1]);
    r.operation = "image.pixels.read".into();
    assert_eq!(m.prepare_run(&r).err(), Some("pipeline_mismatch"));
    let accepted = m.prepare_run(&request(&a, t, &d["nodes"][1])).unwrap();
    assert_eq!(
        accepted
            .invocation
            .inputs
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        vec!["caption"],
        "unconsumed image bytes must not cross the worker boundary"
    );
}

#[test]
fn actual_schema_ten_upgrade_preserves_the_finished_run_and_advances_working_graphs() {
    use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
    use std::fs;
    let f: Value = serde_json::from_str(include_str!("fixtures/native-inputs-v10.json")).unwrap();
    let c: Context = serde_json::from_value(f["context"].clone()).unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let root = tmp.path().join("private");
    drop(Manager::new(root.clone()).unwrap());
    let directory = root.join(c.workspace_id.to_string());
    fs::create_dir(&directory).unwrap();
    fs::write(directory.join("workspace.json"), f["identity"].to_string()).unwrap();
    fs::write(directory.join("owner.lock"), []).unwrap();
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    rt.block_on(async {
        let mut db = SqliteConnection::connect_with(
            &SqliteConnectOptions::new()
                .filename(directory.join("workspace.sqlite3"))
                .create_if_missing(true)
                .foreign_keys(false),
        )
        .await
        .unwrap();
        sqlx::raw_sql(include_str!("fixtures/native-inputs-v10.sql"))
            .execute(&mut db)
            .await
            .unwrap();
        db.close().await.unwrap();
    });
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
        for file in ["workspace.json", "owner.lock", "workspace.sqlite3"] {
            fs::set_permissions(directory.join(file), fs::Permissions::from_mode(0o600)).unwrap();
        }
    }
    let mut m = Manager::new(root).unwrap();
    call(&mut m, json!({"kind":"open","workspace_id":c.workspace_id}));
    assert_eq!(
        m.run_detail(&c, serde_json::from_value(f["run"]["id"].clone()).unwrap())
            .unwrap(),
        f["run"]
    );
    let view = call(
        &mut m,
        json!({"kind":"open_exploration","workspace_id":c.workspace_id,"exploration_id":c.exploration_id,"stage_id":c.stage_id}),
    );
    assert_eq!(view["stage"], f["view"]["stage"]);
    for t in view["trails"].as_array().unwrap() {
        assert_eq!(t["pipeline"]["definition"]["formatVersion"], 4);
        assert_eq!(t["pipeline"]["revision"], 2);
        assert_eq!(t["bindings"], json!({}));
    }
}
#[test]
fn named_inputs_enforce_closed_contracts_and_aggregate_worker_budgets() {
    use play_engine::{
        graph::Definition,
        worker::{InputValue, PipelineInvocation, validate_pipeline},
    };
    use std::collections::BTreeMap;
    let base = definition();
    for invalid in [
        json!([]),
        json!([{"name":"source","semanticType":"image.samples"},{"name":"source","semanticType":"text"}]),
        json!([{"name":"source","semanticType":"image.samples"},{"name":"Caption","semanticType":"text"}]),
        json!([{"name":"source","semanticType":"image.samples"},{"name":"caption","semanticType":"bytes"}]),
        json!([{"name":"source","semanticType":"image.samples"},{"name":"caption","semanticType":"text"},{"name":"a","semanticType":"text"},{"name":"b","semanticType":"text"},{"name":"c","semanticType":"text"}]),
    ] {
        let mut d = base.clone();
        d["inputs"] = invalid;
        assert!(Definition::parse(d).is_err());
    }
    let mut d = base.clone();
    d["inputs"][0]["semanticType"] = json!("text");
    d["nodes"][0] = d["nodes"][1].clone();
    d["nodes"][0]["id"] = json!(Uuid::new_v4());
    d["nodes"][0]["inputs"]["input"]["name"] = json!("source");
    let d = Definition::parse(d).unwrap();
    let mut invocation = PipelineInvocation {
        definition: d,
        inputs: BTreeMap::from([
            (
                "source".into(),
                InputValue::Text {
                    text: "a".repeat(4096),
                },
            ),
            (
                "caption".into(),
                InputValue::Text {
                    text: "b".repeat(4096),
                },
            ),
        ]),
    };
    assert_eq!(validate_pipeline(&invocation), Ok(()));
    invocation.inputs.insert(
        "caption".into(),
        InputValue::Text {
            text: "b".repeat(4097),
        },
    );
    assert_eq!(validate_pipeline(&invocation), Err("input_limit"));
    invocation.inputs.remove("caption");
    assert_eq!(validate_pipeline(&invocation), Err("invalid_binding"));
    invocation
        .inputs
        .insert("caption".into(), InputValue::Image { png: String::new() });
    assert_eq!(validate_pipeline(&invocation), Err("contract_mismatch"));
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let mut d = base.clone();
    d["inputs"][1]["semanticType"] = json!("image.samples");
    d["nodes"][1] = d["nodes"][0].clone();
    d["nodes"][1]["id"] = d["output"].clone();
    d["nodes"][1]["inputs"]["input"]["name"] = json!("caption");
    let mut bytes = include_bytes!("../../../tests/fixtures/sample.png").to_vec();
    bytes.resize(65536, 0);
    let mut i = PipelineInvocation {
        definition: Definition::parse(d).unwrap(),
        inputs: BTreeMap::from([
            (
                "source".into(),
                InputValue::Image {
                    png: STANDARD.encode(&bytes),
                },
            ),
            (
                "caption".into(),
                InputValue::Image {
                    png: STANDARD.encode(&bytes),
                },
            ),
        ]),
    };
    assert_eq!(validate_pipeline(&i), Ok(()));
    bytes.push(0);
    i.inputs.insert(
        "caption".into(),
        InputValue::Image {
            png: STANDARD.encode(&bytes),
        },
    );
    assert_eq!(validate_pipeline(&i), Err("input_limit"));
}
#[test]
fn page_inputs_are_stage_scoped_and_archival_blocks_only_new_admissions() {
    let tmp = tempfile::tempdir().unwrap();
    let mut m = Manager::new(tmp.path().join("private")).unwrap();
    let c = context(&mut m, None, "Page clue");
    image(&mut m, &c, "Figure caption");
    let d = definition();
    let p = pipeline(&mut m, &c, &d);
    call(
        &mut m,
        json!({"kind":"add_page","context":c,"key":Uuid::new_v4(),"title":"Page source","text":"Gur xrl."}),
    );
    let v = view(&mut m, &c);
    let page = v["stage"]["blocks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|b| b["asset"]["content"]["kind"] == "page")
        .unwrap();
    let t = trail(&v, &figure(&v)["asset"]);
    call(
        &mut m,
        save(&c, t, &p, json!({"caption":ref_of(&page["asset"])})),
    );
    let bound = view(&mut m, &c);
    let t = trail(&bound, &figure(&bound)["asset"]);
    let r = request(&c, t, &d["nodes"][1]);
    let admitted = m.prepare_run(&r).unwrap();
    let lifecycle = json!({"kind":"archive_block","context":c,"key":Uuid::new_v4(),"block_id":page["id"],"expected_revision":bound["stage"]["revision"],"source":{"kind":"asset","asset":ref_of(&page["asset"]),"caption":null}});
    call(&mut m, lifecycle.clone());
    let mut fresh = r.clone();
    fresh.key = Uuid::new_v4();
    assert_eq!(m.prepare_run(&fresh).err(), Some("block_archived"));
    let id = admitted.identity.clone();
    let output = play_engine::worker::execute_pipeline(admitted.invocation, |event| {
        let mut n = id.clone();
        n.node_id = serde_json::from_value(event["nodeId"].clone()).unwrap();
        if event["type"] == "node_started" {
            m.start_attempt(&n)
        } else {
            m.finish_attempt(&n, event)
        }
    })
    .unwrap();
    let completed = m
        .finish_run(&id, json!({"status":"completed","output":output}))
        .unwrap();
    assert_eq!(completed["result"]["outputs"]["output"], "The key.");
    assert_eq!(m.existing_run(&r).unwrap().unwrap(), completed);
    let archived = view(&mut m, &c);
    let mut restore = lifecycle;
    restore["kind"] = json!("restore_block");
    restore["key"] = json!(Uuid::new_v4());
    restore["expected_revision"] = archived["stage"]["revision"].clone();
    call(&mut m, restore);
    assert_eq!(
        execute(&mut m, &fresh)["result"]["outputs"]["output"],
        "The key."
    );
    let stage = call(
        &mut m,
        json!({"kind":"create_stage","workspace_id":c.workspace_id,"exploration_id":c.exploration_id,"key":Uuid::new_v4(),"title":"Another Stage"}),
    );
    let mut other = c.clone();
    other.stage_id = serde_json::from_value(stage["stageId"].clone()).unwrap();
    let other_view = image(&mut m, &other, "Foreign Stage");
    let other_ref = ref_of(&figure(&other_view)["caption"]);
    assert!(
        m.handle(serde_json::from_value(save(&c, t, &p, json!({"caption":other_ref}))).unwrap())
            .is_err()
    );
}

#[test]
fn registered_spiral_export_retains_metadata_and_bytes_through_the_real_graph_and_restart() {
    let root = tempfile::tempdir().unwrap();
    let mut m = Manager::new(root.path().join("private")).unwrap();
    let c = context(&mut m, None, "Spiral evidence");
    let v = image_from(
        &mut m,
        &c,
        "Image caption",
        include_bytes!("../../../tests/fixtures/spiral.png"),
    );
    let ids: [Uuid; 3] = std::array::from_fn(|_| Uuid::new_v4());
    let nodes=[
        ("image.pixels.export",json!({"input":{"kind":"input","name":"source"}}),json!({"traversal":"spiral_cw"})),
        ("text.utf8.decode",json!({"input":{"kind":"output","nodeId":ids[0],"port":"data"}}),json!({})),
        ("cipher.rot13",json!({"input":{"kind":"output","nodeId":ids[1],"port":"output"}}),json!({})),
    ].into_iter().enumerate().map(|(i,(op,inputs,parameters))|json!({"id":ids[i],"operation":op,"operationVersion":1,"inputs":inputs,"parameters":parameters,"position":{"x":80+i*240,"y":80}})).collect::<Vec<_>>();
    let d = json!({"formatVersion":4,"inputs":[{"name":"source","semanticType":"image.samples"}],"nodes":nodes,"output":ids[2]});
    let p = pipeline(&mut m, &c, &d);
    let t = trail(&v, &figure(&v)["asset"]);
    call(
        &mut m,
        json!({"kind":"save_trail","context":c,"key":Uuid::new_v4(),"trail_id":t["id"],"expected_revision":t["revision"],"hypothesis":"Read the spiral","pipeline":{"id":p["pipelineId"],"revision":p["revision"]},"bindings":{}}),
    );
    let v = view(&mut m, &c);
    let r = request(&c, trail(&v, &figure(&v)["asset"]), &d["nodes"][2]);
    let completed = execute(&mut m, &r);
    assert_eq!(completed["result"]["outputs"]["output"], "The key!!");
    assert_eq!(
        completed["attempts"][0]["result"]["outputs"]["output"]["traversal"],
        "spiral_cw"
    );
    assert_eq!(
        completed["attempts"][0]["result"]["outputs"]["data"]["base64"],
        cube_core::encoding::base64_encode(b"Gur xrl!!")
    );
    assert_eq!(completed["plan"]["adapterVersion"], 3);
    let id = serde_json::from_value(completed["id"].clone()).unwrap();
    drop(m);
    let mut m = Manager::new(root.path().join("private")).unwrap();
    call(&mut m, json!({"kind":"open","workspace_id":c.workspace_id}));
    assert_eq!(m.run_detail(&c, id).unwrap(), completed);
}
