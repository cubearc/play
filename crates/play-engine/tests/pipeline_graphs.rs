use play_engine::workspace::{Context, Manager};
use serde_json::{Value, json};
use uuid::Uuid;
fn call(m: &mut Manager, v: Value) -> Value {
    m.handle(serde_json::from_value(v).unwrap()).unwrap()
}
fn setup(m: &mut Manager) -> Context {
    let w = call(
        m,
        json!({"kind":"create","key":Uuid::new_v4(),"title":"Graph study"}),
    );
    let e = call(
        m,
        json!({"kind":"create_exploration","workspace_id":w["id"],"key":Uuid::new_v4(),"title":"Independent trail"}),
    );
    serde_json::from_value(
        json!({"workspace_id":w["id"],"exploration_id":e["explorationId"],"stage_id":e["stageId"]}),
    )
    .unwrap()
}
fn node(id: Uuid, source: Value, x: u32) -> Value {
    json!({"id":id,"operation":"cipher.rot13","operationVersion":1,"parameters":{},"position":{"x":x,"y":120},"inputs":{"input":source}})
}
fn graph() -> Value {
    let a = Uuid::new_v4();
    let b = Uuid::new_v4();
    json!({"formatVersion":3,"inputs":[{"name":"source","semanticType":"text"}],"nodes":[node(a,json!({"kind":"input","name":"source"}),80),node(b,json!({"kind":"output","nodeId":a,"port":"output"}),400)],"output":b})
}
fn create(m: &mut Manager, c: &Context, g: &Value) -> Value {
    call(
        m,
        json!({"kind":"create_pipeline","workspace_id":c.workspace_id,"key":Uuid::new_v4(),"title":"Decode twice","definition":g}),
    )
}
#[test]
fn typed_graph_revisions_keep_node_connections_layout_and_selected_output() {
    let t = tempfile::tempdir().unwrap();
    let root = t.path().join("private");
    let mut m = Manager::new(root.clone()).unwrap();
    let c = setup(&mut m);
    let g = graph();
    let p = create(&mut m, &c, &g);
    let read = call(
        &mut m,
        json!({"kind":"get_pipeline","workspace_id":c.workspace_id,"pipeline_id":p["pipelineId"],"revision":1}),
    );
    assert_eq!(read["definition"], g);
    assert_eq!(read["outputNodeId"], g["output"]);
    let mut next = g.clone();
    next["nodes"][1]["position"]["y"] = json!(240);
    let revised = call(
        &mut m,
        json!({"kind":"revise_pipeline","workspace_id":c.workspace_id,"key":Uuid::new_v4(),"pipeline_id":p["pipelineId"],"expected_revision":1,"definition":next}),
    );
    assert_eq!(revised["revision"], 2);
    assert_eq!(
        call(
            &mut m,
            json!({"kind":"get_pipeline","workspace_id":c.workspace_id,"pipeline_id":p["pipelineId"],"revision":1})
        )["definition"],
        g
    );
    drop(m);
    let mut m = Manager::new(root).unwrap();
    call(&mut m, json!({"kind":"open","workspace_id":c.workspace_id}));
    assert_eq!(
        call(
            &mut m,
            json!({"kind":"get_pipeline","workspace_id":c.workspace_id,"pipeline_id":p["pipelineId"],"revision":2})
        )["definition"],
        next
    );
}
#[test]
fn cycles_ports_versions_and_undeclared_bindings_fail_without_saving_a_definition() {
    let t = tempfile::tempdir().unwrap();
    let mut m = Manager::new(t.path().join("private")).unwrap();
    let c = setup(&mut m);
    let baseline = call(&mut m, json!({"kind":"view","workspace_id":c.workspace_id}));
    for flaw in [
        "cycle",
        "port",
        "type",
        "version",
        "slot",
        "duplicate",
        "layout",
    ] {
        let mut g = graph();
        match flaw {
            "cycle" => {
                g["nodes"][0]["inputs"]["input"] =
                    json!({"kind":"output","nodeId":g["nodes"][1]["id"],"port":"output"})
            }
            "port" => g["nodes"][1]["inputs"]["input"]["port"] = json!("missing"),
            "type" => g["inputs"][0]["semanticType"] = json!("image.samples"),
            "version" => g["nodes"][1]["operationVersion"] = json!(999),
            "slot" => g["nodes"][0]["inputs"]["input"]["name"] = json!("private_asset"),
            "duplicate" => g["nodes"][1]["id"] = g["nodes"][0]["id"].clone(),
            "layout" => g["nodes"][0]["position"]["x"] = json!(-1),
            _ => unreachable!(),
        };
        let command = json!({"kind":"create_pipeline","workspace_id":c.workspace_id,"key":Uuid::new_v4(),"title":"Invalid","definition":g});
        assert!(
            m.handle(serde_json::from_value(command).unwrap()).is_err(),
            "{flaw}"
        );
        assert_eq!(
            call(&mut m, json!({"kind":"view","workspace_id":c.workspace_id})),
            baseline
        );
    }
    for op in [
        "cipher.rot13",
        "image.pixels.read",
        "esolang.brainfuck.execute",
    ] {
        let d = cube_core::operations::require(op)
            .unwrap()
            .contract()
            .unwrap();
        println!("{op}: {} bytes", serde_json::to_string(&d).unwrap().len());
    }
}
#[test]
fn selected_output_executes_ancestors_once_with_real_core_outputs_and_no_unrelated_nodes() {
    let g = play_engine::graph::Definition::parse(graph()).unwrap();
    let mut events = Vec::new();
    let result = play_engine::worker::execute_pipeline(
        play_engine::worker::PipelineInvocation {
            definition: g.clone(),
            input: "Gur xrl.".into(),
        },
        |v| {
            events.push(v);
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(result, json!({"text":"Gur xrl."}));
    assert_eq!(events.len(), 4);
    assert_eq!(events[1]["output"]["text"], "The key.");
    assert_eq!(events[3]["nodeId"], g.output.to_string());
    let mut g = g;
    let disconnected = node(Uuid::new_v4(), json!({"kind":"literal","value":"abc"}), 800);
    g.nodes.push(serde_json::from_value(disconnected).unwrap());
    assert_eq!(g.schedule().unwrap().len(), 2);
}
fn execute(m: &mut Manager, c: &Context, g: &Value, retain: bool) -> Value {
    let p = create(m, c, g);
    call(
        m,
        json!({"kind":"add_text","context":c,"key":Uuid::new_v4(),"title":"Clue","text":"Gur xrl."}),
    );
    let view = call(
        m,
        json!({"kind":"open_exploration","workspace_id":c.workspace_id,"exploration_id":c.exploration_id,"stage_id":c.stage_id}),
    );
    let t = &view["trails"][0];
    call(
        m,
        json!({"kind":"save_trail","context":c,"key":Uuid::new_v4(),"trail_id":t["id"],"expected_revision":t["revision"],"hypothesis":"Decode twice","pipeline":{"id":p["pipelineId"],"revision":1}}),
    );
    let output = g["nodes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|n| n["id"] == g["output"])
        .unwrap();
    let request=serde_json::from_value(json!({"context":c,"key":Uuid::new_v4(),"trail_id":t["id"],"pipeline_id":p["pipelineId"],"pipeline_revision":1,"source_asset_id":t["source"]["id"],"source_revision":1,"operation":output["operation"],"operation_version":output["operationVersion"],"retain_output":retain})).unwrap();
    let prepared = m.prepare_run(&request).unwrap();
    let id = prepared.identity.clone();
    let outcome = play_engine::worker::execute_pipeline(prepared.invocation, |event| {
        let mut attempt = id.clone();
        attempt.node_id = serde_json::from_value(event["nodeId"].clone()).unwrap();
        match event["type"].as_str().unwrap() {
            "node_started" => m.start_attempt(&attempt),
            "node_finished" => m.finish_attempt(&attempt, event),
            _ => unreachable!(),
        }
    });
    m.finish_run(
        &id,
        match outcome {
            Ok(output) => json!({"status":"completed","output":output}),
            Err(code) => json!({"status":"failed","code":code}),
        },
    )
    .unwrap()
}
#[test]
fn durable_node_evidence_obeys_retention_and_reopens_without_mutating_the_accepted_graph() {
    for retain in [true, false] {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("private");
        let mut m = Manager::new(root.clone()).unwrap();
        let c = setup(&mut m);
        let g = graph();
        let run = execute(&mut m, &c, &g, retain);
        assert_eq!(run["status"], "completed");
        assert_eq!(run["attempts"].as_array().unwrap().len(), 2);
        assert_eq!(run["plan"]["pipeline"]["definition"], g);
        if retain {
            assert_eq!(
                run["attempts"][0]["result"]["outputs"]["output"],
                "The key."
            );
            assert_eq!(run["result"]["outputs"]["output"], "Gur xrl.");
        } else {
            assert_eq!(run["result"]["replayGap"], "output_not_retained");
            for a in run["attempts"].as_array().unwrap() {
                assert!(a["result"].get("outputs").is_none());
                assert_eq!(a["result"]["replayGap"], "output_not_retained");
            }
        }
        drop(m);
        let mut m = Manager::new(root).unwrap();
        call(&mut m, json!({"kind":"open","workspace_id":c.workspace_id}));
        assert_eq!(
            m.run_detail(&c, serde_json::from_value(run["id"].clone()).unwrap())
                .unwrap(),
            run
        );
    }
}
#[test]
fn a_failed_node_keeps_earlier_evidence_and_never_creates_downstream_attempts() {
    let tmp = tempfile::tempdir().unwrap();
    let mut m = Manager::new(tmp.path().join("private")).unwrap();
    let c = setup(&mut m);
    let mut g = graph();
    let id = g["nodes"][1]["id"].clone();
    g["nodes"][1] = json!({"id":id,"operation":"esolang.brainfuck.execute","operationVersion":2,"parameters":{"max_instructions":1,"max_output_bytes":16384,"max_memory_bytes":8388608},"position":{"x":400,"y":120},"inputs":{"source":{"kind":"literal","value":"++."},"input":{"kind":"literal","value":[]}}});
    let third = Uuid::new_v4();
    g["nodes"].as_array_mut().unwrap().push(node(
        third,
        json!({"kind":"literal","value":"untouched"}),
        700,
    ));
    // Force the failing node to depend on a completed text predecessor.
    g["nodes"][1]["inputs"]["source"] =
        json!({"kind":"output","nodeId":g["nodes"][0]["id"],"port":"output"});
    // A text program containing a long sequence after ROT13 reaches the instruction cap.
    g["nodes"][0]["inputs"]["input"] = json!({"kind":"literal","value":"++."});
    let run = execute(&mut m, &c, &g, true);
    assert_eq!(run["status"], "failed");
    assert_eq!(run["attempts"].as_array().unwrap().len(), 2);
    assert_eq!(run["attempts"][0]["status"], "completed");
    assert_eq!(run["attempts"][1]["status"], "failed");
    assert_eq!(run["attempts"][0]["result"]["outputs"]["output"], "++.");
}
#[test]
fn byte_representations_are_never_silently_coerced_at_a_join() {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let pixel = Uuid::new_v4();
    let program = Uuid::new_v4();
    let join = Uuid::new_v4();
    let image = STANDARD.encode(include_bytes!("../../../tests/fixtures/sample.png"));
    let d = json!({"formatVersion":3,"inputs":[{"name":"source","semanticType":"image.samples"}],"nodes":[{"id":pixel,"operation":"image.pixels.read","operationVersion":1,"parameters":{"x":0,"y":0,"expand_palette":false},"position":{"x":0,"y":0},"inputs":{"input":{"kind":"input","name":"source"}}},node(program,json!({"kind":"literal","value":",."}),300),{"id":join,"operation":"esolang.brainfuck.execute","operationVersion":2,"parameters":{"max_instructions":1000000,"max_output_bytes":16384,"max_memory_bytes":8388608},"position":{"x":600,"y":0},"inputs":{"source":{"kind":"output","nodeId":program,"port":"output"},"input":{"kind":"output","nodeId":pixel,"port":"output"}}}],"output":join});
    assert!(
        matches!(
            play_engine::graph::Definition::parse(d),
            Err("contract_mismatch")
        ),
        "bytes and bytes.array require an explicit conversion; the engine cannot invent one"
    );
    assert!(!image.is_empty());
}
#[test]
fn actual_schema_nine_upgrade_preserves_the_finished_run_and_advances_working_graphs() {
    use sqlx::{Connection, SqliteConnection, sqlite::SqliteConnectOptions};
    use std::fs;
    let f: Value = serde_json::from_str(include_str!("fixtures/native-graph-v9.json")).unwrap();
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
        sqlx::raw_sql(include_str!("fixtures/native-graph-v9.sql"))
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
        assert_eq!(t["pipeline"]["definition"]["formatVersion"], 3);
        assert_eq!(t["pipeline"]["revision"], 2);
    }
}
#[test]
fn output_budget_measures_raw_bytes_once_across_the_whole_run() {
    let input = "x".repeat(8192);
    let mut d = play_engine::graph::Definition::parse(graph()).unwrap();
    let exact = play_engine::worker::execute_pipeline(
        play_engine::worker::PipelineInvocation {
            definition: d.clone(),
            input: input.clone(),
        },
        |_| Ok(()),
    );
    assert_eq!(exact.unwrap(), json!({"text":input}));
    let next = Uuid::new_v4();
    let mut n = d.nodes[0].clone();
    n.id = next;
    n.inputs = std::collections::BTreeMap::from([(
        "input".into(),
        play_engine::graph::Source::Output {
            node_id: d.output,
            port: "output".into(),
        },
    )]);
    d.nodes.push(n);
    d.output = next;
    let mut events = Vec::new();
    assert_eq!(
        play_engine::worker::execute_pipeline(
            play_engine::worker::PipelineInvocation {
                definition: d,
                input
            },
            |v| {
                events.push(v);
                Ok(())
            }
        ),
        Err("output_limit")
    );
    assert_eq!(
        events
            .iter()
            .filter(|v| v["type"] == "node_finished")
            .count(),
        2
    );
}
