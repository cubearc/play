//! Shared trusted runtime lifecycle for CLI and host-authorized workspace intents.
use crate::{
    Error, Result,
    docker::{Docker, default_image},
    process::{self, HostProcess},
    state,
};
use clap::Parser;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

pub struct Opened {
    pub workspace: state::Workspace,
    pub root: PathBuf,
    pub home: PathBuf,
    pub url: String,
}

#[derive(Clone, Parser)]
#[command(
    name = "cube-play",
    version,
    about = "Open a private local Cube Play investigation workspace"
)]
pub struct Options {
    /// Directory, file or local challenge; defaults to the current directory
    pub path: Option<PathBuf>,
    /// Stop this workspace without deleting investigation data
    #[arg(long, conflicts_with_all=["no_browser","offline","install_runtime"])]
    pub stop: bool,
    /// Print the local session URL without opening a browser
    #[arg(long)]
    pub no_browser: bool,
    /// Use an installed image without builds or setup-time downloads
    #[arg(long)]
    pub offline: bool,
    /// Build the local runtime from trusted installation sources
    #[arg(long, conflicts_with_all=["path","offline","no_browser"])]
    pub install_runtime: bool,
    /// Trusted Cube Play source checkout for runtime installation
    #[arg(long, env = "CUBE_PLAY_RUNTIME_SOURCE")]
    pub runtime_source: Option<PathBuf>,
    #[arg(long, env = "CUBE_PLAY_HOME", hide = true)]
    pub state_home: Option<PathBuf>,
    #[arg(long, env = "CUBE_PLAY_IMAGE", hide = true)]
    pub image: Option<String>,
    #[arg(long)]
    pub json_errors: bool,
    #[arg(long, default_value_t = 1, hide = true)]
    pub protocol: u32,
    #[arg(long, hide=true, conflicts_with_all=["path","stop","offline","no_browser","install_runtime"])]
    pub launcher_protocol: bool,
    #[arg(long, hide=true, conflicts_with_all=["path","stop","install_runtime","no_browser","launcher_protocol","stop_chooser"])]
    pub workspace_host: bool,
    /// Stop the local workspace chooser without stopping investigations
    #[arg(long, conflicts_with_all=["path","stop","install_runtime","no_browser","launcher_protocol","workspace_host"])]
    pub stop_chooser: bool,
}

pub fn home(args: &Options) -> Result<PathBuf> {
    let path=args.state_home.clone().or_else(||directories::ProjectDirs::from("org", "CubeArc", "CubePlay").map(|dirs|dirs.data_local_dir().to_path_buf()))
        .ok_or_else(||Error::new("state_unavailable", "A private user data directory is unavailable; set CUBE_PLAY_HOME to a private absolute directory"))?;
    if !path.is_absolute() || path.parent().is_none() {
        return Err(Error::new(
            "invalid_parameter",
            "CUBE_PLAY_HOME requires an absolute private directory, not a filesystem root",
        ));
    }
    state::private_directory(&path)?;
    path.canonicalize().map_err(Error::io)
}

pub fn runtime_source(args: &Options) -> Result<PathBuf> {
    let source = args
        .runtime_source
        .clone()
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."));
    if !source.is_absolute() {
        return Err(Error::new(
            "invalid_parameter",
            "Runtime sources require an explicitly trusted absolute path",
        ));
    }
    Ok(source)
}

pub fn run(args: &Options, cancelled: Arc<AtomicBool>) -> Result<Option<Opened>> {
    if args.protocol != 1 {
        return Err(Error::new(
            "incompatible_launcher",
            "This launcher supports invocation protocol 1",
        ));
    }
    let cwd = std::env::current_dir().map_err(Error::io)?;
    let selected = if args.install_runtime {
        None
    } else {
        match state::resolve(args.path.as_deref(), &cwd) {
            Ok(selected) => Some(selected),
            Err(error) if args.stop => {
                let path = args
                    .path
                    .as_ref()
                    .map(|path| {
                        if path.is_absolute() {
                            path.clone()
                        } else {
                            cwd.join(path)
                        }
                    })
                    .unwrap_or_else(|| cwd.clone());
                if !path.is_absolute()
                    || path
                        .components()
                        .any(|part| matches!(part, std::path::Component::ParentDir))
                {
                    return Err(error);
                }
                Some((path, None))
            }
            Err(error) => return Err(error),
        }
    };
    let home = home(args)?;
    if !args.stop
        && selected.as_ref().is_some_and(|(root, _)| {
            state::contains_path(root, &home) || state::contains_path(&home, root)
        })
    {
        return Err(Error::new(
            "unsafe_source",
            "Choose a source directory separate from the private launcher state",
        ));
    }
    let executable=process::executable("docker", std::env::var_os("CUBE_PLAY_DOCKER").map(PathBuf::from), selected.as_ref().map(|value|value.0.as_path()))
        .map_err(|_|Error::new("docker_unavailable", "Docker is unavailable; install Docker Desktop or a local Docker Engine with Compose v2, start it, then run cube play again"))?;
    let docker = Docker {
        process: HostProcess {
            executable,
            cancelled: cancelled.clone(),
        },
        home: home.clone(),
    };
    docker.detect(if args.stop {
        None
    } else {
        selected.as_ref().map(|value| value.0.as_path())
    })?;
    let image_name = args.image.clone().unwrap_or(default_image()?);
    if args.install_runtime {
        let _lock = state::lock(&home.join("runtime.lock"), &cancelled)?;
        docker.install(&image_name, &runtime_source(args)?)?;
        println!("Cube Play runtime is installed. Run cube play from an investigation directory.");
        return Ok(None);
    }
    let (root, selection) =
        selected.ok_or_else(|| Error::new("invalid_path", "No workspace was selected"))?;
    let workspace = state::workspace(&home, &root, !args.stop, &cancelled)?;
    let directory = home.join(workspace.id.simple().to_string());
    state::private_directory(&directory)?;
    let _lock = state::lock(&directory.join("startup.lock"), &cancelled)?;
    if args.stop {
        docker.stop_workspace(&workspace)?;
        println!("Cube Play workspace stopped. Investigation data was preserved.");
        return Ok(None);
    }
    let image = {
        let _lock = state::lock(&home.join("runtime.lock"), &cancelled)?;
        match docker.image(&image_name)? {
            Some(image) => image,
            None if args.offline => {
                return Err(Error::new(
                    "runtime_missing",
                    "Offline startup requires an installed Cube Play runtime; prepare it once with cube-play --install-runtime",
                ));
            }
            None if args.image.is_none() => {
                if !args.json_errors {
                    eprintln!(
                        "Preparing the Cube Play runtime. First-time setup may download build dependencies."
                    );
                }
                docker.install(&image_name, &runtime_source(args)?)?
            }
            None => {
                return Err(Error::new(
                    "runtime_missing",
                    "The configured runtime image is not installed; install it explicitly or use the default Cube Play runtime",
                ));
            }
        }
    };
    docker.runner_support(&image)?;
    let context = directory.join("workspace.json");
    let expected = serde_json::json!({"id":workspace.id,"name":workspace.name});
    match fs::symlink_metadata(&context) {
        Ok(_) => {
            if state::read_json::<serde_json::Value>(&context)? != expected {
                return Err(Error::new(
                    "invalid_state",
                    "Workspace context disagrees with its private registry; preserve the state for recovery",
                ));
            }
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            state::write_json(&context, &expected)?
        }
        Err(error) => return Err(Error::io(error)),
    }
    // Reuse the mounted file's identity: replacing it can invalidate Docker Desktop file sharing.
    // The parent is private; this non-secret metadata file must also work with Docker Desktop file sharing.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&context, fs::Permissions::from_mode(0o644)).map_err(Error::io)?;
    }
    fs::write(directory.join("empty.env"), b"").map_err(Error::io)?;
    if !args.json_errors {
        eprintln!("Starting Cube Play…");
    }
    let runtime = docker.start(&workspace, &directory, &image)?;
    if cancelled.load(Ordering::Relaxed) {
        if !runtime.reused {
            docker.stop_workspace(&workspace)?;
        }
        return Err(Error::cancelled());
    }
    let url = match docker.session(&runtime.container, runtime.port, selection.as_ref()) {
        Ok(url) => url,
        Err(error) => {
            if !runtime.reused {
                docker.stop_workspace(&workspace)?;
            }
            return Err(error);
        }
    };
    Ok(Some(Opened {
        workspace,
        root,
        home,
        url,
    }))
}
