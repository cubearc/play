use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    net::TcpListener,
    path::{Path, PathBuf},
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};

use serde::Deserialize;
use serde_json::{Value, json};

use crate::{
    Error, Result,
    process::HostProcess,
    state::{self, Workspace},
};

pub fn runtime_user() -> Result<String> {
    #[cfg(unix)]
    {
        let uid = nix::unistd::geteuid().as_raw();
        if uid == 0 {
            return Err(Error::new(
                "unsafe_user",
                "Run Cube Play as a normal user with local Docker access, without sudo; runtime containers must remain non-root",
            ));
        }
        Ok(format!("{uid}:{}", nix::unistd::getegid().as_raw()))
    }
    #[cfg(windows)]
    {
        Ok("65532:65532".into())
    }
}

pub fn default_image() -> Result<String> {
    Ok(format!(
        "cube-play-local:{}-r4-{}",
        env!("CARGO_PKG_VERSION"),
        runtime_user()?.replace(':', "-")
    ))
}
const SHORT: Duration = Duration::from_secs(30);
const START: Duration = Duration::from_secs(120);
const BUILD: Duration = Duration::from_secs(900);

#[derive(Clone)]
pub struct Docker {
    pub process: HostProcess,
    pub home: PathBuf,
}

pub struct ReadyRuntime {
    pub container: Container,
    pub port: u16,
    pub reused: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Container {
    #[serde(rename = "Id")]
    pub id: String,
    pub image: String,
    pub config: ContainerConfig,
    pub state: ContainerState,
    pub host_config: HostConfig,
    pub mounts: Vec<Mount>,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerConfig {
    pub labels: HashMap<String, String>,
    pub user: String,
    pub env: Vec<String>,
    #[serde(default)]
    pub entrypoint: Vec<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ContainerState {
    pub running: bool,
    pub health: Option<Health>,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Health {
    pub status: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct HostConfig {
    pub privileged: bool,
    pub readonly_rootfs: bool,
    pub cap_drop: Option<Vec<String>>,
    pub cap_add: Option<Vec<String>>,
    pub network_mode: String,
    pub pid_mode: String,
    pub devices: Option<Vec<Value>>,
    pub security_opt: Option<Vec<String>>,
    pub memory: u64,
    pub nano_cpus: u64,
    pub pids_limit: Option<i64>,
    pub port_bindings: Option<HashMap<String, Vec<PortBinding>>>,
}
#[derive(Deserialize)]
pub struct PortBinding {
    #[serde(rename = "HostIp")]
    pub host_ip: String,
    #[serde(rename = "HostPort")]
    pub host_port: String,
}
#[derive(Deserialize)]
pub struct Mount {
    #[serde(rename = "Type")]
    pub kind: String,
    #[serde(rename = "Source")]
    pub source: String,
    #[serde(rename = "Destination")]
    pub destination: String,
    #[serde(rename = "RW")]
    pub writable: bool,
    #[serde(rename = "Name")]
    pub name: Option<String>,
}

pub fn project(workspace: &Workspace) -> String {
    format!("cube-play-{}", workspace.id.simple())
}
pub fn container_name(workspace: &Workspace) -> String {
    format!("{}-local", project(workspace))
}
fn escaped(value: &str) -> String {
    value.replace('$', "$$")
}

pub fn configuration(
    workspace: &Workspace,
    directory: &Path,
    image: &str,
    port: u16,
) -> Result<Value> {
    let project = project(workspace);
    let source = escaped(&state::docker_path(&workspace.root)?);
    let context = escaped(&state::docker_path(&directory.join("workspace.json"))?);
    let runner = crate::runner::configuration(workspace, &source, image)?;
    Ok(json!({
        "services": { "cube-runner":runner, "play-local": {
            "image":image, "container_name":container_name(workspace),
            "labels":{"org.cubearc.play.workspace":workspace.id.to_string(),"org.cubearc.play.protocol":"1"},
            "environment":{"CUBE_PLAY_ORIGIN":format!("http://127.0.0.1:{port}"),"CUBE_PLAY_WORKSPACE":"/run/cube-play/workspace.json","CUBE_PLAY_SOURCE":"/workspace/source","CUBE_PLAY_RUNNER_SOCKET":crate::runner::SOCKET},
            "depends_on":{"cube-runner":{"condition":"service_healthy"}},
            "ports":[{"target":7331,"published":port.to_string(),"host_ip":"127.0.0.1","protocol":"tcp"}],
            "volumes":[
                {"type":"bind","source":source,"target":"/workspace/source","read_only":true,"bind":{"create_host_path":false}},
                {"type":"bind","source":context,"target":"/run/cube-play/workspace.json","read_only":true,"bind":{"create_host_path":false}},
                {"type":"volume","source":"data","target":"/var/lib/cube-play"},
                {"type":"volume","source":"artifacts","target":"/var/lib/cube-play/artifacts"},
                {"type":"volume","source":"terminal","target":"/run/cube-play/terminal","read_only":true}
            ],
            "user":runtime_user()?,"read_only":true,"cap_drop":["ALL"],"security_opt":["no-new-privileges:true"],
            "init":true,"pids_limit":128,"mem_limit":"256m","cpus":1,
            "tmpfs":["/tmp:rw,noexec,nosuid,size=16m"],"stop_grace_period":"10s"
        }},
        "volumes": {
            "data":{"name":format!("{project}-data"),"labels":{"org.cubearc.play.workspace":workspace.id.to_string()}},
            "artifacts":{"name":format!("{project}-artifacts"),"labels":{"org.cubearc.play.workspace":workspace.id.to_string()}},
            "terminal":{"name":format!("{project}-terminal"),"labels":{"org.cubearc.play.workspace":workspace.id.to_string()}}
        }
    }))
}

impl Docker {
    fn run(&self, args: &[&str], timeout: Duration) -> Result<crate::process::Output> {
        self.process.run(
            &args.iter().map(OsStr::new).collect::<Vec<_>>(),
            &self.home,
            timeout,
        )
    }

    pub fn detect(&self, workspace: Option<&Path>) -> Result<()> {
        let host = if std::env::var_os("DOCKER_CONTEXT").is_none() {
            std::env::var("DOCKER_HOST").ok()
        } else {
            None
        };
        let host = if let Some(host) = host {
            host
        } else {
            let context = self.run(&["context", "inspect"], SHORT)?;
            if !context.success {
                return Err(Error::new(
                    "docker_unavailable",
                    "Docker context is unavailable; select a local Docker Desktop or Engine context",
                ));
            }
            let values: Value = serde_json::from_slice(&context.stdout).map_err(|_| {
                Error::new(
                    "docker_unavailable",
                    "Docker returned an invalid context description",
                )
            })?;
            values
                .get(0)
                .and_then(|value| value.pointer("/Endpoints/docker/Host"))
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    Error::new(
                        "docker_unavailable",
                        "Docker context has no local Engine endpoint",
                    )
                })?
                .to_string()
        };
        if !local_endpoint(&host) {
            return Err(Error::new(
                "remote_docker",
                "Cube Play requires a local Docker Engine; remote TCP/SSH contexts cannot access private workspace files",
            ));
        }
        if let Some(root) = workspace {
            let endpoint = host
                .strip_prefix("unix://")
                .map(|value| {
                    percent_encoding::percent_decode_str(value)
                        .decode_utf8()
                        .map(|value| PathBuf::from(value.as_ref()))
                })
                .transpose()
                .map_err(|_| {
                    Error::new(
                        "docker_unavailable",
                        "Docker returned an unsupported local socket path",
                    )
                })?;
            for path in endpoint.into_iter().chain([
                PathBuf::from("/var/run/docker.sock"),
                PathBuf::from("/run/docker.sock"),
            ]) {
                let resolved = path.canonicalize().unwrap_or_else(|_| {
                    path.parent()
                        .and_then(|parent| parent.canonicalize().ok())
                        .map(|parent| parent.join(path.file_name().unwrap_or_default()))
                        .unwrap_or(path)
                });
                if state::contains_path(root, &resolved) {
                    return Err(Error::new(
                        "unsafe_workspace",
                        "Selected source directory contains a Docker control socket; choose a narrower investigation directory",
                    ));
                }
            }
        }
        let info = self.run(&["info", "--format", "{{.OSType}}"], SHORT)?;
        if !info.success {
            return Err(Error::new(
                "docker_unavailable",
                "Start Docker Desktop or the local Docker Engine, check access permissions, then run cube play again",
            ));
        }
        if info.stdout.trim_ascii() != b"linux" {
            return Err(Error::new(
                "unsupported_runtime",
                "Cube Play requires Linux containers; switch Docker Desktop to Linux container mode",
            ));
        }
        if !self.run(&["compose", "version", "--short"], SHORT)?.success {
            return Err(Error::new(
                "compose_unavailable",
                "Install Docker Compose v2 or update Docker Desktop; Cube Play manages it automatically",
            ));
        }
        Ok(())
    }

    pub fn image(&self, name: &str) -> Result<Option<String>> {
        let output = self.run(&["image", "inspect", name], SHORT)?;
        if !output.success {
            return Ok(None);
        }
        let values: Value = serde_json::from_slice(&output.stdout).map_err(|_| {
            Error::new(
                "invalid_runtime",
                "Docker returned an invalid image description",
            )
        })?;
        let image = values.get(0).ok_or_else(|| {
            Error::new(
                "invalid_runtime",
                "Docker did not describe the requested image",
            )
        })?;
        if image
            .pointer("/Config/Labels/org.cubearc.play.protocol")
            .and_then(Value::as_str)
            != Some("1")
            || image
                .pointer("/Config/Labels/org.cubearc.play.runtime-revision")
                .and_then(Value::as_str)
                != Some("4")
            || image
                .pointer("/Config/Labels/org.cubearc.play.version")
                .and_then(Value::as_str)
                != Some(env!("CARGO_PKG_VERSION"))
            || image.pointer("/Config/User").and_then(Value::as_str) != Some(&runtime_user()?)
        {
            return Err(Error::new(
                "incompatible_runtime",
                "The runtime image is incompatible; install the Cube Play runtime matching this launcher version",
            ));
        }
        let id = image
            .get("Id")
            .and_then(Value::as_str)
            .filter(|id| id.starts_with("sha256:"))
            .ok_or_else(|| Error::new("invalid_runtime", "Runtime image identity is invalid"))?;
        Ok(Some(id.to_string()))
    }

    pub fn install(&self, name: &str, source: &Path) -> Result<String> {
        let source = source.canonicalize().map_err(|_|Error::new("runtime_missing", "Runtime sources are unavailable; set CUBE_PLAY_RUNTIME_SOURCE to the trusted Cube Play checkout and retry"))?;
        if !source.join("Dockerfile").is_file()
            || !source.join("crates/play-local/Cargo.toml").is_file()
            || name.starts_with("sha256:")
        {
            return Err(Error::new(
                "invalid_parameter",
                "Choose a trusted Cube Play source checkout and a local image tag for installation",
            ));
        }
        let user = runtime_user()?;
        let (uid, gid) = user
            .split_once(':')
            .ok_or_else(|| Error::new("unsafe_user", "Runtime user identity is invalid"))?;
        let uid_argument = OsString::from(format!("CUBE_PLAY_UID={uid}"));
        let gid_argument = OsString::from(format!("CUBE_PLAY_GID={gid}"));
        let core = std::env::var_os("CUBE_PLAY_CORE_SOURCE")
            .map(PathBuf::from)
            .unwrap_or_else(|| source.parent().unwrap_or(&source).join("cube"));
        let core=core.canonicalize().map_err(|_|Error::new("core_missing","Runtime builds require a trusted Cube 0.18.0 checkout beside Play, or CUBE_PLAY_CORE_SOURCE pointing to that checkout"))?;
        if !core.join("Cargo.toml").is_file() || !core.join("crates/cube-core/Cargo.toml").is_file()
        {
            return Err(Error::new(
                "core_missing",
                "The configured engine source is not a Cube Rust workspace",
            ));
        }
        let core_argument = OsString::from(format!("cube_engine={}", state::docker_path(&core)?));
        let output = self.process.run(
            &[
                OsStr::new("build"),
                OsStr::new("--tag"),
                OsStr::new(name),
                OsStr::new("--build-arg"),
                &uid_argument,
                OsStr::new("--build-arg"),
                &gid_argument,
                OsStr::new("--build-context"),
                &core_argument,
                OsStr::new("."),
            ],
            &source,
            BUILD,
        )?;
        if !output.success {
            return Err(Error::new(
                "build_failed",
                "Could not prepare the Cube Play runtime; check Docker build access, available disk space and setup-time dependency downloads",
            ));
        }
        self.image(name)?.ok_or_else(|| {
            Error::new(
                "runtime_missing",
                "The Cube Play runtime image was not created",
            )
        })
    }

    pub fn runner_support(&self, image: &str) -> Result<()> {
        let probe = self.run(
            &[
                "run",
                "--rm",
                "--pull",
                "never",
                "--network",
                "none",
                "--read-only",
                "--cap-drop",
                "ALL",
                "--security-opt",
                "no-new-privileges:true",
                "--entrypoint",
                "/usr/local/bin/cube-runner",
                image,
                "--sandbox-check",
            ],
            SHORT,
        )?;
        if !probe.success {
            return Err(Error::new(
                "unsupported_isolation",
                "Cube Runner requires Landlock ABI 3 with its LSM enabled. Use Docker Desktop 4.94.0 or newer, or a compatible local Linux engine. No unrestricted terminal was started",
            ));
        }
        Ok(())
    }
    pub fn inspect(&self, workspace: &Workspace) -> Result<Option<Container>> {
        self.inspect_named(workspace, &container_name(workspace))
    }
    fn inspect_named(&self, workspace: &Workspace, name: &str) -> Result<Option<Container>> {
        let output = self.run(&["container", "inspect", name], SHORT)?;
        if !output.success {
            if !self
                .run(&["info", "--format", "{{.OSType}}"], SHORT)?
                .success
            {
                return Err(Error::new(
                    "docker_unavailable",
                    "The Docker Engine became unavailable; restore it and retry before changing workspace state",
                ));
            }
            return Ok(None);
        }
        let mut containers: Vec<Container> =
            serde_json::from_slice(&output.stdout).map_err(|_| {
                Error::new(
                    "invalid_runtime",
                    "Docker returned an invalid container description",
                )
            })?;
        if containers.len() != 1 {
            return Err(Error::new(
                "invalid_runtime",
                "Docker returned an ambiguous container identity",
            ));
        }
        let container = containers.remove(0);
        if container.config.labels.get("org.cubearc.play.workspace")
            != Some(&workspace.id.to_string())
            || container.config.labels.get("com.docker.compose.project")
                != Some(&project(workspace))
            || container
                .config
                .labels
                .get("org.cubearc.play.protocol")
                .map(String::as_str)
                != Some("1")
        {
            return Err(Error::new(
                "ownership_mismatch",
                "A container with this name is not owned by this Cube Play workspace; no runtime was changed",
            ));
        }
        Ok(Some(container))
    }

    pub fn compose(
        &self,
        workspace: &Workspace,
        directory: &Path,
        args: &[&str],
        timeout: Duration,
    ) -> Result<()> {
        let project = OsString::from(project(workspace));
        let file = directory.join("compose.json");
        let environment = directory.join("empty.env");
        let mut values: Vec<&OsStr> = vec![
            OsStr::new("compose"),
            OsStr::new("--project-name"),
            &project,
            OsStr::new("--file"),
            file.as_os_str(),
            OsStr::new("--env-file"),
            environment.as_os_str(),
        ];
        values.extend(args.iter().map(OsStr::new));
        let output = self.process.run(&values, directory, timeout)?;
        if !output.success {
            return Err(Error::new(
                "startup_failed",
                "The local runtime could not become ready; check Docker file-sharing permissions, available resources and port conflicts, then retry",
            ));
        }
        Ok(())
    }

    pub fn stop(&self, container: &Container) -> Result<()> {
        let cleanup = Self {
            process: HostProcess {
                executable: self.process.executable.clone(),
                cancelled: Arc::new(AtomicBool::new(false)),
            },
            home: self.home.clone(),
        };
        if !cleanup
            .run(&["container", "stop", "--time", "10", &container.id], SHORT)?
            .success
        {
            return Err(Error::new(
                "shutdown_failed",
                "Could not stop this workspace runtime; investigation volumes were preserved",
            ));
        }
        Ok(())
    }

    pub fn stop_workspace(&self, workspace: &Workspace) -> Result<()> {
        for name in [container_name(workspace), crate::runner::name(workspace)] {
            if let Some(container) = self.inspect_named(workspace, &name)?
                && container.state.running
            {
                self.stop(&container)?;
            }
        }
        Ok(())
    }
    fn ready_runner(&self, workspace: &Workspace, image: &str) -> Result<()> {
        let container = self
            .inspect_named(workspace, &crate::runner::name(workspace))?
            .ok_or_else(|| {
                Error::new(
                    "runtime_unhealthy",
                    "The isolated runner is missing; stop this workspace and restart it",
                )
            })?;
        crate::runner::validate(&container, workspace, image)?;
        if !container.state.running
            || container.state.health.as_ref().map(|h| h.status.as_str()) != Some("healthy")
        {
            return Err(Error::new(
                "runtime_unhealthy",
                "The isolated runner is not healthy; stop this workspace and restart it",
            ));
        }
        Ok(())
    }
    pub fn start(
        &self,
        workspace: &Workspace,
        directory: &Path,
        image: &str,
    ) -> Result<ReadyRuntime> {
        for suffix in ["data", "artifacts", "terminal"] {
            let output = self.run(
                &[
                    "volume",
                    "inspect",
                    &format!("{}-{suffix}", project(workspace)),
                ],
                SHORT,
            )?;
            if output.success {
                let values: Value = serde_json::from_slice(&output.stdout).map_err(|_| {
                    Error::new("invalid_runtime", "Docker returned invalid volume metadata")
                })?;
                if values
                    .get(0)
                    .and_then(|value| value.pointer("/Labels/org.cubearc.play.workspace"))
                    .and_then(Value::as_str)
                    != Some(&workspace.id.to_string())
                {
                    return Err(Error::new(
                        "ownership_mismatch",
                        "An existing data volume is not owned by this workspace; it was not mounted or modified",
                    ));
                }
            }
        }
        if let Some(runner) = self.inspect_named(workspace, &crate::runner::name(workspace))?
            && runner.state.running
        {
            crate::runner::validate(&runner, workspace, image)?;
        }
        if let Some(container) = self.inspect(workspace)?
            && container.state.running
        {
            let port = validate(&container, workspace, directory, image)?;
            self.ready_runner(workspace, image)?;
            if container
                .state
                .health
                .as_ref()
                .map(|health| health.status.as_str())
                != Some("healthy")
            {
                return Err(Error::new(
                    "runtime_unhealthy",
                    "The existing runtime is not healthy; run cube play --stop for this workspace, then restart it",
                ));
            }
            return Ok(ReadyRuntime {
                container,
                port,
                reused: true,
            });
        }
        for _ in 0..3 {
            let listener = TcpListener::bind("127.0.0.1:0").map_err(Error::io)?;
            let port = listener.local_addr().map_err(Error::io)?.port();
            drop(listener);
            state::write_json(
                &directory.join("compose.json"),
                &configuration(workspace, directory, image, port)?,
            )?;
            match self.compose(
                workspace,
                directory,
                &[
                    "up",
                    "--detach",
                    "--wait",
                    "--wait-timeout",
                    "60",
                    "--no-build",
                    "--pull",
                    "never",
                ],
                START,
            ) {
                Ok(()) => {
                    let container = self.inspect(workspace)?.ok_or_else(|| {
                        Error::new("startup_failed", "The runtime disappeared during startup")
                    })?;
                    let actual = match validate(&container, workspace, directory, image) {
                        Ok(port) => port,
                        Err(error) => {
                            self.stop_workspace(workspace)?;
                            return Err(error);
                        }
                    };
                    if actual != port
                        || !container.state.running
                        || container
                            .state
                            .health
                            .as_ref()
                            .map(|health| health.status.as_str())
                            != Some("healthy")
                    {
                        self.stop_workspace(workspace)?;
                        return Err(Error::new(
                            "runtime_unhealthy",
                            "The runtime did not pass readiness checks",
                        ));
                    }
                    if let Err(error) = self.ready_runner(workspace, image) {
                        self.stop_workspace(workspace)?;
                        return Err(error);
                    }
                    return Ok(ReadyRuntime {
                        container,
                        port,
                        reused: false,
                    });
                }
                Err(error) => {
                    let cleanup = Self {
                        process: HostProcess {
                            executable: self.process.executable.clone(),
                            cancelled: Arc::new(AtomicBool::new(false)),
                        },
                        home: self.home.clone(),
                    };
                    let existing = cleanup.inspect(workspace)?;
                    cleanup.stop_workspace(workspace)?;
                    let occupied =
                        TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).is_err();
                    if existing
                        .as_ref()
                        .is_some_and(|container| container.state.running)
                    {
                        return Err(error);
                    }
                    if error.code == "cancelled" || !occupied {
                        return Err(error);
                    }
                }
            }
        }
        Err(Error::new(
            "port_unavailable",
            "Could not reserve a free loopback port after bounded retries; stop conflicting local applications and retry",
        ))
    }

    pub fn session(
        &self,
        container: &Container,
        port: u16,
        selection: Option<&(String, &'static str)>,
    ) -> Result<String> {
        let output = self.run(&["container", "logs", "--tail", "32", &container.id], SHORT)?;
        if !output.success {
            return Err(Error::new(
                "session_missing",
                "The local session link is unavailable; stop and restart this workspace",
            ));
        }
        let text = std::str::from_utf8(&output.stdout).map_err(|_| {
            Error::new(
                "session_missing",
                "The runtime returned invalid local session output",
            )
        })?;
        let expected = format!("Open Cube Play: http://127.0.0.1:{port}/#session=");
        let secret=text.lines().rev().find_map(|line|line.strip_prefix(&expected)).filter(|secret|uuid::Uuid::parse_str(secret).is_ok())
            .ok_or_else(||Error::new("session_missing", "The runtime did not provide a compatible session link; stop and restart this workspace"))?;
        let mut fragment = form_urlencoded::Serializer::new(String::new());
        fragment.append_pair("session", secret);
        if let Some((name, intent)) = selection {
            fragment
                .append_pair("file", name)
                .append_pair("intent", intent);
        }
        Ok(format!("http://127.0.0.1:{port}/#{}", fragment.finish()))
    }
}

pub fn local_endpoint(host: &str) -> bool {
    host.starts_with("unix:///") || host.starts_with("npipe:////./pipe/")
}

pub fn validate(
    container: &Container,
    workspace: &Workspace,
    directory: &Path,
    image: &str,
) -> Result<u16> {
    let host = &container.host_config;
    let failure = || {
        Error::new(
            "unsafe_runtime",
            "Existing runtime configuration does not match this workspace's security boundaries; no source or data was changed",
        )
    };
    if container.image != image {
        return Err(Error::new(
            "runtime_changed",
            "The installed runtime image changed; stop this workspace before restarting with the updated image. Investigation volumes are preserved",
        ));
    }
    if container.config.user != runtime_user()?
        || host.privileged
        || !host.readonly_rootfs
        || host.memory != 256 * 1024 * 1024
        || host.nano_cpus != 1_000_000_000
        || host.pids_limit != Some(128)
        || host
            .cap_add
            .as_ref()
            .is_some_and(|values| !values.is_empty())
        || host
            .devices
            .as_ref()
            .is_some_and(|values| !values.is_empty())
        || host.network_mode != format!("{}_default", project(workspace))
        || !host.pid_mode.is_empty()
        || !host
            .cap_drop
            .as_ref()
            .is_some_and(|values| values.iter().any(|value| value.eq_ignore_ascii_case("ALL")))
        || !host.security_opt.as_ref().is_some_and(|values| {
            values
                .iter()
                .any(|value| value == "no-new-privileges:true" || value == "no-new-privileges")
        })
        || host.port_bindings.as_ref().map_or(0, HashMap::len) != 1
    {
        return Err(Error::new(
            "unsafe_runtime",
            "Existing runtime user, resource, network or privilege settings do not match this workspace; stop it explicitly before recovery",
        ));
    }
    let binding = host
        .port_bindings
        .as_ref()
        .ok_or_else(failure)?
        .get("7331/tcp")
        .filter(|values| values.len() == 1)
        .map(|values| &values[0])
        .ok_or_else(failure)?;
    let port = binding
        .host_port
        .parse::<u16>()
        .ok()
        .filter(|port| *port != 0)
        .ok_or_else(failure)?;
    if binding.host_ip != "127.0.0.1"
        || !container
            .config
            .env
            .iter()
            .any(|value| value == &format!("CUBE_PLAY_ORIGIN=http://127.0.0.1:{port}"))
    {
        return Err(Error::new(
            "unsafe_runtime",
            "Existing runtime loopback publication does not match its application origin; stop it explicitly before recovery",
        ));
    }
    let binds: Vec<_> = container
        .mounts
        .iter()
        .filter(|mount| mount.kind == "bind")
        .collect();
    if binds.len() != 2
        || !binds.iter().any(|mount| {
            mount.destination == "/workspace/source"
                && !mount.writable
                && mount_source_matches(&mount.source, &workspace.root)
        })
        || !binds.iter().any(|mount| {
            mount.destination == "/run/cube-play/workspace.json"
                && !mount.writable
                && mount_source_matches(&mount.source, &directory.join("workspace.json"))
        })
    {
        return Err(Error::new(
            "unsafe_runtime",
            "Existing runtime source/context mounts do not match the authorized read-only paths; stop it explicitly before recovery",
        ));
    }
    let volumes: Vec<_> = container
        .mounts
        .iter()
        .filter(|mount| mount.kind == "volume")
        .collect();
    let project = project(workspace);
    if volumes.len() != 3
        || !volumes.iter().any(|mount| {
            mount.destination == "/var/lib/cube-play"
                && mount.writable
                && mount.name.as_deref() == Some(&format!("{project}-data"))
        })
        || !volumes.iter().any(|mount| {
            mount.destination == "/var/lib/cube-play/artifacts"
                && mount.writable
                && mount.name.as_deref() == Some(&format!("{project}-artifacts"))
        })
        || !volumes.iter().any(|mount| {
            mount.destination == "/run/cube-play/terminal"
                && !mount.writable
                && mount.name.as_deref() == Some(&format!("{project}-terminal"))
        })
        || container
            .mounts
            .iter()
            .any(|mount| mount.kind != "bind" && mount.kind != "volume" && mount.kind != "tmpfs")
    {
        return Err(Error::new(
            "unsafe_runtime",
            "Existing runtime data/artifact mounts do not match this workspace's persistent volumes; stop it explicitly before recovery",
        ));
    }
    Ok(port)
}

pub(crate) fn mount_source_matches(actual: &str, expected: &Path) -> bool {
    let Ok(expected) = state::docker_path(expected) else {
        return false;
    };
    if actual == expected {
        return true;
    }
    #[cfg(target_os = "macos")]
    {
        macos_mount_matches(actual, &expected)
    }
    #[cfg(windows)]
    {
        windows_mount_matches(actual, &expected)
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        false
    }
}

#[cfg(any(target_os = "macos", test))]
fn macos_mount_matches(actual: &str, expected: &str) -> bool {
    // Docker Desktop can report its VM host-mount prefix after container recreation.
    expected.starts_with('/') && actual.strip_prefix("/host_mnt") == Some(expected)
}

#[cfg(any(windows, test))]
fn windows_mount_matches(actual: &str, expected: &str) -> bool {
    let normalized = expected.replace('\\', "/");
    if normalized.len() < 3
        || normalized.as_bytes()[1] != b':'
        || normalized.as_bytes()[2] != b'/'
        || !normalized.as_bytes()[0].is_ascii_alphabetic()
    {
        return false;
    }
    let drive = (normalized.as_bytes()[0] as char).to_ascii_lowercase();
    let tail = &normalized[3..];
    actual == format!("/run/desktop/mnt/host/{drive}/{tail}")
        || actual == format!("/host_mnt/{drive}/{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn macos_desktop_mount_translation_keeps_the_exact_authorized_path() {
        let source = "/private/var/investigation $private 空";
        assert!(macos_mount_matches(&format!("/host_mnt{source}"), source));
        for other in [
            "/host_mnt/private/var/other",
            "/host_mnt/private/var/investigation $private 空/child",
            "/host_mnt//private/var/investigation $private 空",
        ] {
            assert!(!macos_mount_matches(other, source));
        }
    }
    #[test]
    fn compose_does_not_interpolate_paths_or_publish_runner_ports() {
        let root = tempfile::tempdir().unwrap();
        let workspace = Workspace {
            id: Uuid::new_v4(),
            root: root.path().join("files $private"),
            name: "private".into(),
        };
        let value = configuration(
            &workspace,
            &root.path().join("state $data"),
            "sha256:123",
            17331,
        )
        .unwrap();
        assert_eq!(
            value["services"]["play-local"]["volumes"][0]["source"],
            state::docker_path(&workspace.root)
                .unwrap()
                .replace('$', "$$")
        );
        assert_eq!(value["services"].as_object().unwrap().len(), 2);
        assert_eq!(value["services"]["cube-runner"]["network_mode"], "none");
        assert!(value["services"]["cube-runner"]["ports"].is_null());
        assert_eq!(
            value["services"]["play-local"]["ports"][0]["host_ip"],
            "127.0.0.1"
        );
        assert!(!value.to_string().contains("docker.sock"));
        assert!(local_endpoint("unix:///var/run/docker.sock"));
        assert!(local_endpoint("npipe:////./pipe/docker_engine"));
        for host in [
            "tcp://127.0.0.1:2375",
            "ssh://remote",
            "npipe:////other/pipe/docker_engine",
        ] {
            assert!(!local_endpoint(host));
        }
    }
    #[test]
    fn windows_mount_translation_keeps_the_exact_authorized_directory() {
        assert!(super::windows_mount_matches(
            "/run/desktop/mnt/host/c/Users/Private 空",
            r"C:\Users\Private 空"
        ));
        assert!(super::windows_mount_matches(
            "/host_mnt/d/Investigations",
            r"D:\Investigations"
        ));
        assert!(!super::windows_mount_matches(
            "/run/desktop/mnt/host/c/Users/Other",
            r"C:\Users\Private 空"
        ));
        assert!(!super::windows_mount_matches(
            "/tmp/Users/Private 空",
            r"C:\Users\Private 空"
        ));
    }
    use uuid::Uuid;
}
