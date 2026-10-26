use std::{net::SocketAddr, path::PathBuf};

use axum::http::Uri;

/// Trusted process configuration. User workspaces are not served as static assets.
#[derive(Clone, Debug)]
pub struct Config {
    pub bind: SocketAddr,
    pub origin: String,
    pub frontend_dir: PathBuf,
}

impl Config {
    pub fn from_env() -> Result<Self, String> {
        Self::parse(
            &std::env::var("CUBE_PLAY_BIND").unwrap_or_else(|_| "127.0.0.1:7331".into()),
            &std::env::var("CUBE_PLAY_ORIGIN").unwrap_or_else(|_| "http://127.0.0.1:7331".into()),
            std::env::var_os("CUBE_PLAY_FRONTEND_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| "frontend/dist".into()),
            std::env::var("CUBE_PLAY_CONTAINER").as_deref() == Ok("1"),
        )
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
        })
    }

    pub fn authority(&self) -> &str {
        self.origin.trim_start_matches("http://")
    }
}

#[cfg(test)]
mod tests {
    use super::Config;

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
