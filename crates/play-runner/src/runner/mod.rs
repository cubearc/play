mod console;
mod policy;
mod sandbox;
mod session;

use crate::{
    protocol::{Client, SESSION_LIMIT, Server},
    transport::{channel, receive, send},
};
use std::{
    fs, io,
    os::unix::fs::{FileTypeExt, MetadataExt, OpenOptionsExt, PermissionsExt},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use tokio::{
    net::{UnixListener, UnixStream},
    sync::{Semaphore, watch},
};

#[derive(Clone)]
pub struct Config {
    pub socket: PathBuf,
    pub source: PathBuf,
    pub imports: PathBuf,
    pub output: PathBuf,
    pub cube: PathBuf,
}
impl Config {
    pub fn from_env() -> io::Result<Self> {
        let path = |name: &str, default: &str| {
            std::env::var_os(name)
                .map(PathBuf::from)
                .unwrap_or_else(|| default.into())
        };
        let config = Self {
            socket: path("CUBE_RUNNER_SOCKET", "/run/cube-play/terminal/runner.sock"),
            source: path("CUBE_RUNNER_SOURCE", "/workspace/source"),
            imports: path("CUBE_RUNNER_IMPORTS", "/workspace/data/imports"),
            output: path("CUBE_RUNNER_OUTPUT", "/workspace/output"),
            cube: path("CUBE_RUNNER_CUBE", "/usr/local/bin/cube"),
        };
        if [
            &config.socket,
            &config.source,
            &config.imports,
            &config.output,
            &config.cube,
        ]
        .iter()
        .any(|p| !p.is_absolute())
        {
            return Err(io::Error::other(
                "Runner paths must be trusted absolute paths",
            ));
        }
        Ok(config)
    }
    fn directory(&self, location: &crate::protocol::Location) -> io::Result<PathBuf> {
        if !location.valid() {
            return Err(io::Error::other("Invalid working directory"));
        }
        let root = match location.store {
            crate::protocol::Store::Source => &self.source,
            crate::protocol::Store::Imports => &self.imports,
            crate::protocol::Store::Output => &self.output,
        };
        let root = root.canonicalize()?;
        let handle = cap_std::fs::Dir::open_ambient_dir(&root, cap_std::ambient_authority())?;
        let mut relative = PathBuf::new();
        for component in std::path::Path::new(&location.path).components() {
            relative.push(component);
            if handle.symlink_metadata(&relative)?.file_type().is_symlink() {
                return Err(io::Error::other(
                    "Symbolic working directories are not allowed",
                ));
            }
        }
        let selected = root.join(&location.path).canonicalize()?;
        if !selected.starts_with(&root) || !selected.is_dir() {
            return Err(io::Error::other(
                "Working directory leaves its authorized root",
            ));
        }
        Ok(selected)
    }
}
struct SocketGuard {
    path: PathBuf,
    _lock: fs::File,
}
impl Drop for SocketGuard {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
fn listener(config: &Config) -> io::Result<(UnixListener, SocketGuard, u32)> {
    if nix::unistd::geteuid().is_root() {
        return Err(io::Error::other(
            "Cube Runner must run without root privileges",
        ));
    }
    let parent = config
        .socket
        .parent()
        .ok_or_else(|| io::Error::other("Invalid runner socket"))?;
    let metadata = fs::symlink_metadata(parent)?;
    if !metadata.is_dir()
        || metadata.file_type().is_symlink()
        || metadata.permissions().mode() & 0o077 != 0
        || metadata.uid() != nix::unistd::geteuid().as_raw()
    {
        return Err(io::Error::other(
            "Runner socket storage must be a private directory owned by the runtime user",
        ));
    }
    let lockpath = parent.join("owner.lock");
    let lock = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .mode(0o600)
        .custom_flags(nix::libc::O_NOFOLLOW)
        .open(&lockpath)?;
    if !lock.metadata()?.is_file() || lock.metadata()?.nlink() != 1 {
        return Err(io::Error::other("Unsafe runner lock"));
    }
    lock.try_lock()
        .map_err(|_| io::Error::other("Another runner owns this socket directory"))?;
    match fs::symlink_metadata(&config.socket) {
        Ok(metadata) if metadata.file_type().is_socket() => fs::remove_file(&config.socket)?,
        Ok(_) => {
            return Err(io::Error::other(
                "Runner socket path is occupied by an unexpected file",
            ));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let socket = UnixListener::bind(&config.socket)?;
    fs::set_permissions(&config.socket, fs::Permissions::from_mode(0o600))?;
    Ok((
        socket,
        SocketGuard {
            path: config.socket.clone(),
            _lock: lock,
        },
        metadata.uid(),
    ))
}
pub async fn serve(config: Config) -> io::Result<()> {
    let (listener, _guard, owner) = listener(&config)?;
    let slots = Arc::new(Semaphore::new(SESSION_LIMIT));
    let (shutdown, rx) = watch::channel(false);
    let mut tasks = tokio::task::JoinSet::new();
    loop {
        tokio::select! {
            result=listener.accept()=>{
                let (socket,_)=result?;
                if socket.peer_cred()?.uid()!=owner {continue;}
                // Every connection, including probes, is bounded while its first frame arrives.
                if tasks.len()>=8 {drop(socket);continue;}
                tasks.spawn(connection(socket,config.clone(),slots.clone(),rx.clone()));
            },
            _=tasks.join_next(),if !tasks.is_empty()=>{},
            _=stop_signal()=>break,
        }
    }
    let _ = shutdown.send(true);
    while tasks.join_next().await.is_some() {}
    Ok(())
}
async fn connection(
    socket: UnixStream,
    config: Config,
    slots: Arc<Semaphore>,
    shutdown: watch::Receiver<bool>,
) -> io::Result<()> {
    let mut channel = channel(socket);
    let first = tokio::time::timeout(Duration::from_secs(3), receive::<Client>(&mut channel))
        .await
        .map_err(|_| io::Error::other("Runner handshake timed out"))??;
    match first {
        Some(Client::Probe) => {
            send(
                &mut channel,
                &Server::Status {
                    active: SESSION_LIMIT - slots.available_permits(),
                    limit: SESSION_LIMIT,
                },
            )
            .await
        }
        Some(Client::Start { location, size }) => {
            if !size.valid() || !location.valid() {
                return send(&mut channel, &Server::error("invalid_request")).await;
            }
            let Ok(permit) = slots.try_acquire_owned() else {
                return send(&mut channel, &Server::error("session_limit")).await;
            };
            let Ok(directory) = config.directory(&location) else {
                return send(&mut channel, &Server::error("path_denied")).await;
            };
            session::run(channel, config, directory, location, size, permit, shutdown).await
        }
        _ => send(&mut channel, &Server::error("invalid_request")).await,
    }
}
async fn stop_signal() {
    if let Ok(mut signal) =
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
    {
        tokio::select! {_=signal.recv()=>{},_=tokio::signal::ctrl_c()=>{}}
    } else {
        let _ = tokio::signal::ctrl_c().await;
    }
}
pub fn console(config: Config, location: crate::protocol::Location) -> io::Result<()> {
    console::run(config, location)
}
pub fn sandbox_check(config: &Config) -> io::Result<()> {
    sandbox::apply(config, &std::env::temp_dir())
}
