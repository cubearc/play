//! Controlled Docker CLI protocol fixture; real Docker flows run in runtime_smoke.py.
use serde_json::{Value, json};
use std::{env, fs, io::Write, path::PathBuf};

fn emit(value: &Value) {
    println!("{value}");
}
fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args
        .first()
        .is_some_and(|arg| arg.starts_with("http://127.0.0.1:"))
    {
        fs::write(env::var_os("CUBE_PLAY_BROWSER_RECORD").unwrap(), b"opened").unwrap();
        return;
    }
    if args.first().is_some_and(|arg| arg == "child") {
        std::thread::sleep(std::time::Duration::from_secs(4));
        return;
    }
    if args
        .first()
        .is_some_and(|arg| arg == "hang" || arg == "orphan-pipe")
    {
        let _child = std::process::Command::new(env::current_exe().unwrap())
            .arg("child")
            .spawn()
            .unwrap();
        if args[0] == "orphan-pipe" {
            return;
        }
        std::thread::sleep(std::time::Duration::from_secs(10));
        return;
    }
    if args.first().is_some_and(|arg| arg == "flood") {
        std::io::stdout().write_all(&vec![b'x'; 300_000]).unwrap();
        return;
    }
    let home = PathBuf::from(env::var_os("CUBE_PLAY_FIXTURE_DIR").unwrap());
    let mode = fs::read_to_string(home.join("mode")).unwrap_or_default();
    let mut events = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(home.join("events"))
        .unwrap();
    writeln!(events, "{}", json!(args)).unwrap();
    if args.first().is_some_and(|arg| arg == "context") {
        let host = if mode == "remote" {
            "ssh://remote.example".to_string()
        } else if mode == "socket-exposure" {
            format!(
                "unix://{}/docker.sock",
                env::var("CUBE_PLAY_FIXTURE_SOURCE").unwrap()
            )
        } else {
            if cfg!(windows) {
                "npipe:////./pipe/docker_engine".to_string()
            } else {
                "unix:///local/docker.sock".to_string()
            }
        };
        emit(&json!([{"Endpoints":{"docker":{"Host":host}}}]));
        return;
    }
    if args.first().is_some_and(|arg| arg == "info") {
        if mode == "daemon-off" {
            std::process::exit(1);
        }
        println!(
            "{}",
            if mode == "windows-engine" {
                "windows"
            } else {
                "linux"
            }
        );
        return;
    }
    if args.get(0..2) == Some(&["compose".to_string(), "version".to_string()]) {
        if mode == "old-compose" {
            std::process::exit(1);
        }
        println!("2.23.3");
        return;
    }
    if args.first().is_some_and(|arg| arg == "run") {
        if mode == "no-landlock" {
            std::process::exit(1);
        }
        return;
    }
    if args.first().is_some_and(|arg| arg == "image") {
        if mode == "missing-image" && !home.join("built").exists() {
            std::process::exit(1);
        }
        emit(
            &json!([{"Id":image(&mode),"Config":{"User":env::var("CUBE_PLAY_FIXTURE_USER").unwrap(),"Labels":{"org.cubearc.play.protocol":if mode=="bad-image"{"99"}else{"1"},"org.cubearc.play.version":"0.2.0","org.cubearc.play.runtime-revision":if mode=="old-runtime"{"22"}else{"23"}}}}]),
        );
        return;
    }
    if args.first().is_some_and(|arg| arg == "build") {
        fs::write(home.join("built"), b"installed").unwrap();
        return;
    }
    if args.first().is_some_and(|arg| arg == "volume") {
        if mode == "bad-volume" {
            emit(&json!([{"Labels":{"org.cubearc.play.workspace":"foreign"}}]));
            return;
        }
        if mode == "owned-volume" {
            let registry: Value = serde_json::from_slice(
                &fs::read(
                    PathBuf::from(env::var_os("CUBE_PLAY_HOME").unwrap()).join("workspaces.json"),
                )
                .unwrap(),
            )
            .unwrap();
            emit(
                &json!([{"Labels":{"org.cubearc.play.workspace":registry["workspaces"][0]["id"]}}]),
            );
            return;
        }
        std::process::exit(1);
    }
    if args.first().is_some_and(|arg| arg == "compose") {
        let file = args.iter().position(|arg| arg == "--file").unwrap();
        let project =
            args[args.iter().position(|arg| arg == "--project-name").unwrap() + 1].clone();
        let value: Value = serde_json::from_slice(&fs::read(&args[file + 1]).unwrap()).unwrap();
        if mode == "startup-failure" {
            std::process::exit(1);
        }
        for (role, service) in value["services"].as_object().unwrap() {
            let runner = role == "cube-runner";
            let container_name = service["container_name"].as_str().unwrap();
            let mut labels = service["labels"].clone();
            labels["com.docker.compose.project"] = json!(project);
            let mounts:Vec<_>=service["volumes"].as_array().unwrap().iter().map(|mount|{
            let kind=mount["type"].as_str().unwrap();let source=mount["source"].as_str().unwrap().replace("$$","$");
            json!({"Type":kind,"Source":source,"Destination":mount["target"],"RW":mount["read_only"]!=true,"Name":if kind=="volume"{value["volumes"][&source]["name"].clone()}else{Value::Null}})
        }).collect();
            let environment: Vec<_> = service["environment"]
                .as_object()
                .unwrap()
                .iter()
                .map(|(key, value)| format!("{key}={}", value.as_str().unwrap()))
                .collect();
            let ports = if runner {
                Value::Null
            } else {
                json!({"7331/tcp":[{"HostIp":service["ports"][0]["host_ip"],"HostPort":service["ports"][0]["published"]}]})
            };
            let data = json!({"Id":container_name,"Image":service["image"],"Config":{"Labels":labels,"User":service["user"],"Env":environment,"Entrypoint":if runner{json!(["/usr/local/bin/cube-runner"])}else{json!(["play-local"])}},"State":{"Running":true,"Health":{"Status":"healthy"}},"HostConfig":{"Privileged":false,"ReadonlyRootfs":true,"CapDrop":["ALL"],"CapAdd":[],"Devices":[],"PidMode":"","NetworkMode":if runner{"none".to_string()}else{format!("{project}_default")},"SecurityOpt":["no-new-privileges:true"],"Memory":if runner{536870912}else{268435456},"NanoCpus":1000000000,"PidsLimit":service["pids_limit"],"PortBindings":ports},"Mounts":mounts});
            fs::write(
                home.join(format!("{container_name}.json")),
                data.to_string(),
            )
            .unwrap();
        }
        if mode == "cancel-after-start" {
            std::thread::sleep(std::time::Duration::from_secs(10));
        }
        if mode == "fail-after-start" {
            std::process::exit(1);
        }
        return;
    }
    if args.first().is_some_and(|arg| arg == "container") {
        let name = args.last().unwrap();
        let path = home.join(format!("{name}.json"));
        if !path.exists() {
            std::process::exit(1);
        }
        let mut data: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        match args[1].as_str() {
            "inspect" => {
                match mode.as_str() {
                    "wrong-owner" => {
                        data["Config"]["Labels"]["org.cubearc.play.workspace"] = json!("foreign")
                    }
                    "unsafe-container" => data["HostConfig"]["Privileged"] = json!(true),
                    "host-network" => data["HostConfig"]["NetworkMode"] = json!("host"),
                    "host-pids" => data["HostConfig"]["PidMode"] = json!("host"),
                    "capabilities" => data["HostConfig"]["CapAdd"] = json!(["SYS_ADMIN"]),
                    "writable-source" => data["Mounts"][0]["RW"] = json!(true),
                    "wrong-port" if !name.ends_with("-runner") => {
                        data["HostConfig"]["PortBindings"]["7331/tcp"][0]["HostIp"] =
                            json!("0.0.0.0")
                    }
                    "unhealthy" => data["State"]["Health"]["Status"] = json!("unhealthy"),
                    _ => {}
                }
                emit(&json!([data]));
            }
            "stop" => {
                data["State"]["Running"] = json!(false);
                fs::write(path, data.to_string()).unwrap();
            }
            "logs" => {
                if mode == "cancel-existing-session" {
                    fs::write(home.join("session-waiting"), b"waiting").unwrap();
                    std::thread::sleep(std::time::Duration::from_secs(10));
                }
                if mode.contains("missing-session") {
                    println!("no session");
                    return;
                }
                let origin = data["Config"]["Env"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find_map(|value| {
                        value
                            .as_str()
                            .and_then(|value| value.strip_prefix("CUBE_PLAY_ORIGIN="))
                    })
                    .unwrap();
                println!("Open Cube Play: {origin}/#session=9e6f93a0-c62b-4c74-acf9-b77f17824239");
            }
            _ => std::process::exit(1),
        }
        return;
    }
    std::process::exit(1);
}
fn image(mode: &str) -> String {
    format!(
        "sha256:{}",
        if mode == "changed-image" { "b" } else { "a" }.repeat(64)
    )
}
