use crate::{
    Error, Result,
    docker::{Container, mount_source_matches, project, runtime_user},
    state::Workspace,
};
use serde_json::{Value, json};
pub(crate) const SOCKET: &str = "/run/cube-play/terminal/runner.sock";
pub(crate) fn name(workspace: &Workspace) -> String {
    format!("{}-runner", project(workspace))
}
pub(crate) fn configuration(workspace: &Workspace, source: &str, image: &str) -> Result<Value> {
    Ok(json!({
        "image":image,"container_name":name(workspace),
        "labels":{"org.cubearc.play.workspace":workspace.id.to_string(),"org.cubearc.play.protocol":"1","org.cubearc.play.role":"runner"},
        "entrypoint":["/usr/local/bin/cube-runner"],
        "environment":{"CUBE_RUNNER_SOCKET":SOCKET,"CUBE_RUNNER_SOURCE":"/workspace/source","CUBE_RUNNER_IMPORTS":"/workspace/data/imports","CUBE_RUNNER_OUTPUT":"/workspace/output","CUBE_RUNNER_CUBE":"/usr/local/bin/cube"},
        "network_mode":"none","user":runtime_user()?,"read_only":true,"cap_drop":["ALL"],"security_opt":["no-new-privileges:true"],
        "init":true,"pids_limit":64,"mem_limit":"512m","cpus":1,"tmpfs":["/tmp:rw,noexec,nosuid,size=16m"],"stop_grace_period":"10s",
        "healthcheck":{"test":["CMD","/usr/local/bin/cube-runner","--healthcheck"],"interval":"5s","timeout":"3s","start_period":"5s","retries":6},
        "volumes":[
            {"type":"bind","source":source,"target":"/workspace/source","read_only":true,"bind":{"create_host_path":false}},
            {"type":"volume","source":"data","target":"/workspace/data","read_only":true},
            {"type":"volume","source":"artifacts","target":"/workspace/output"},
            {"type":"volume","source":"terminal","target":"/run/cube-play/terminal"}
        ]
    }))
}
pub(crate) fn validate(container: &Container, workspace: &Workspace, image: &str) -> Result<()> {
    if container.image != image {
        return Err(Error::new(
            "runtime_changed",
            "The installed runner image changed; stop this workspace before restarting with the updated image. Investigation volumes are preserved",
        ));
    }
    let invalid = || {
        Error::new(
            "unsafe_runtime",
            "The runner's image, privileges, resources, mounts or network boundaries changed; stop this workspace explicitly before recovery",
        )
    };
    let host = &container.host_config;
    if container.config.user != runtime_user()?
        || container.config.entrypoint != ["/usr/local/bin/cube-runner"]
        || host.privileged
        || !host.readonly_rootfs
        || host.network_mode != "none"
        || !host.pid_mode.is_empty()
        || host.memory != 512 * 1024 * 1024
        || host.nano_cpus != 1_000_000_000
        || host.pids_limit != Some(64)
        || host
            .port_bindings
            .as_ref()
            .is_some_and(|ports| !ports.is_empty())
        || host
            .cap_add
            .as_ref()
            .is_some_and(|values| !values.is_empty())
        || host
            .devices
            .as_ref()
            .is_some_and(|values| !values.is_empty())
        || !host
            .cap_drop
            .as_ref()
            .is_some_and(|values| values.iter().any(|v| v.eq_ignore_ascii_case("ALL")))
        || !host.security_opt.as_ref().is_some_and(|values| {
            values
                .iter()
                .any(|v| v == "no-new-privileges:true" || v == "no-new-privileges")
        })
    {
        return Err(invalid());
    }
    for expected in [
        format!("CUBE_RUNNER_SOCKET={SOCKET}"),
        "CUBE_RUNNER_SOURCE=/workspace/source".into(),
        "CUBE_RUNNER_IMPORTS=/workspace/data/imports".into(),
        "CUBE_RUNNER_OUTPUT=/workspace/output".into(),
        "CUBE_RUNNER_CUBE=/usr/local/bin/cube".into(),
    ] {
        if container
            .config
            .env
            .iter()
            .filter(|value| value.split('=').next() == expected.split('=').next())
            .collect::<Vec<_>>()
            != [&expected]
        {
            return Err(invalid());
        }
    }
    let binds: Vec<_> = container
        .mounts
        .iter()
        .filter(|m| m.kind == "bind")
        .collect();
    if binds.len() != 1
        || !binds.iter().any(|m| {
            m.destination == "/workspace/source"
                && !m.writable
                && mount_source_matches(&m.source, &workspace.root)
        })
    {
        return Err(invalid());
    }
    let volumes: Vec<_> = container
        .mounts
        .iter()
        .filter(|m| m.kind == "volume")
        .collect();
    if volumes.len() != 3 {
        return Err(invalid());
    }
    for (suffix, target, writable) in [
        ("data", "/workspace/data", false),
        ("artifacts", "/workspace/output", true),
        ("terminal", "/run/cube-play/terminal", true),
    ] {
        let name = format!("{}-{suffix}", project(workspace));
        if !volumes.iter().any(|m| {
            m.name.as_deref() == Some(&name) && m.destination == target && m.writable == writable
        }) {
            return Err(invalid());
        }
    }
    if container
        .mounts
        .iter()
        .any(|m| !matches!(m.kind.as_str(), "volume" | "bind" | "tmpfs"))
    {
        return Err(invalid());
    }
    Ok(())
}
