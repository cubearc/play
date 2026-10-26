use std::{
    io::Read,
    net::SocketAddr,
    path::{Path, PathBuf},
};

use axum::http::Uri;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkspaceContext {
    pub id: uuid::Uuid,
    pub name: String,
}

impl WorkspaceContext {
    fn load(path: &Path) -> Result<Self, String> {
        let metadata =
            std::fs::symlink_metadata(path).map_err(|_| "Workspace context is unavailable")?;
        if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 8192 {
            return Err("Workspace context must be a bounded regular file".into());
        }
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(|_| "Workspace context is unavailable")?
            .take(8193)
            .read_to_end(&mut bytes)
            .map_err(|_| "Could not read workspace context")?;
        if bytes.len() > 8192 {
            return Err("Workspace context exceeds its size limit".into());
        }
        let context: Self = serde_json::from_slice(&bytes)
            .map_err(|_| "Workspace context is invalid or unsupported")?;
        if context.id.get_version_num() != 4
            || context.name.is_empty()
            || context.name.len() > 4096
            || context.name.contains('\0')
        {
            return Err("Workspace context identity or display name is invalid".into());
        }
        Ok(context)
    }
}

/// Trusted process configuration. User workspaces are not served as static assets.
#[derive(Clone, Debug)]
pub struct Config {
    pub bind: SocketAddr,
    pub origin: String,
    pub frontend_dir: PathBuf,
    pub workspace: Option<WorkspaceContext>,
    pub source_dir: Option<PathBuf>,
    pub data_dir: Option<PathBuf>,
    pub state_dir: Option<PathBuf>,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        let mut config = Self::parse(
            &std::env::var("CUBE_PLAY_BIND").unwrap_or_else(|_| "127.0.0.1:7331".into()),
            &std::env::var("CUBE_PLAY_ORIGIN").unwrap_or_else(|_| "http://127.0.0.1:7331".into()),
            std::env::var_os("CUBE_PLAY_FRONTEND_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| "frontend/dist".into()),
            std::env::var("CUBE_PLAY_CONTAINER").as_deref() == Ok("1"),
        )?;
        match (
            std::env::var_os("CUBE_PLAY_WORKSPACE"),
            std::env::var_os("CUBE_PLAY_SOURCE"),
        ) {
            (None, None) => {}
            (Some(context), Some(source)) => {
                let source = PathBuf::from(source);
                if !source.is_absolute() || !source.is_dir() || source.parent().is_none() {
                    return Err("Workspace source must be an authorized local directory".into());
                }
                config.workspace = Some(WorkspaceContext::load(Path::new(&context))?);
                std::fs::read_dir(&source).map_err(|_|"Workspace source is not readable by the non-root runtime; check Docker file sharing and UID/GID mapping")?;
                config.source_dir = Some(
                    source
                        .canonicalize()
                        .map_err(|_| "Workspace source is unavailable")?,
                );
            }
            _ => return Err("Workspace context and source must be configured together".into()),
        }
        if let Some(path) = std::env::var_os("CUBE_PLAY_DATA") {
            let path = PathBuf::from(path);
            if !path.is_absolute() || path.parent().is_none() {
                return Err("CUBE_PLAY_DATA must be an absolute managed storage directory".into());
            }
            config.data_dir = Some(path);
        }
        if let Some(path) = std::env::var_os("CUBE_PLAY_STATE") {
            let path = PathBuf::from(path);
            if !path.is_absolute() || path.parent().is_none() {
                return Err("CUBE_PLAY_STATE must be an absolute private directory".into());
            }
            config.state_dir = Some(path);
        }
        Ok(config)
    }

    pub fn parse(
        bind: &str,
        origin: &str,
        frontend_dir: PathBuf,
        container: bool,
    ) -> Result<Self, String> {
        let bind: SocketAddr = bind
            .parse()
            .map_err(|_| "CUBE_PLAY_BIND must be an IP address and port")?;
        if bind.port() == 0
            || (!bind.ip().is_loopback() && !(container && bind.ip().is_unspecified()))
        {
            return Err("Bind to loopback; wildcard binding requires explicit container mode and a loopback-only Docker port publication".into());
        }
        let uri: Uri = origin
            .parse()
            .map_err(|_| "CUBE_PLAY_ORIGIN must be an HTTP loopback origin")?;
        let port = uri
            .port_u16()
            .filter(|port| *port != 0)
            .ok_or("CUBE_PLAY_ORIGIN requires an explicit nonzero port")?;
        if origin != format!("http://127.0.0.1:{port}") {
            return Err("CUBE_PLAY_ORIGIN must be exactly http://127.0.0.1:<port>".into());
        }
        Ok(Self {
            bind,
            origin: origin.into(),
            frontend_dir,
            workspace: None,
            source_dir: None,
            data_dir: None,
            state_dir: None,
        })
    }

    pub fn authority(&self) -> &str {
        self.origin.trim_start_matches("http://")
    }

    pub fn session_cookie(&self) -> String {
        format!(
            "cube_play_session_{}",
            self.origin.rsplit(':').next().unwrap_or("invalid")
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{Config, WorkspaceContext};
    #[test]
    fn validates_bounded_workspace_context_without_private_host_paths() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("workspace.json");
        std::fs::write(
            &path,
            serde_json::to_vec(
                &serde_json::json!({"id":uuid::Uuid::new_v4(),"name":"Investigation 空"}),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(
            WorkspaceContext::load(&path).unwrap().name,
            "Investigation 空"
        );
        for value in [
            serde_json::json!({"id":uuid::Uuid::nil(),"name":"name"}),
            serde_json::json!({"id":uuid::Uuid::new_v4(),"name":""}),
            serde_json::json!({"id":uuid::Uuid::new_v4(),"name":"name","root":"private"}),
        ] {
            std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
            assert!(WorkspaceContext::load(&path).is_err());
        }
        std::fs::write(&path, vec![b' '; 8193]).unwrap();
        assert!(WorkspaceContext::load(&path).is_err());
        assert!(WorkspaceContext::load(directory.path()).is_err());
    }

    #[test]
    fn requires_loopback_and_explicit_container_binding() {
        for bind in [
            "0.0.0.0:7331",
            "192.0.2.1:7331",
            "[::]:7331",
            "127.0.0.1:0",
            "invalid",
        ] {
            assert!(Config::parse(bind, "http://127.0.0.1:7331", ".".into(), false).is_err());
        }
        assert!(Config::parse("0.0.0.0:7331", "http://127.0.0.1:17331", ".".into(), true).is_ok());
        assert!(
            Config::parse("192.0.2.1:7331", "http://127.0.0.1:7331", ".".into(), true).is_err()
        );
    }

    #[test]
    fn rejects_ambiguous_or_remote_origins() {
        for origin in [
            "http://localhost:7331",
            "http://127.0.0.1",
            "https://127.0.0.1:7331",
            "http://127.0.0.1:0",
            "http://127.0.0.1:7331/",
            "http://127.0.0.1:7331?q=1",
            "http://user@127.0.0.1:7331",
            "http://example.com:7331",
            "invalid",
        ] {
            assert!(
                Config::parse("127.0.0.1:7331", origin, ".".into(), false).is_err(),
                "{origin}"
            );
        }
    }
}
