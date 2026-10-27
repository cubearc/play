//! Private host-only control records. No control capability enters a container.
#[cfg(windows)]
mod windows;
use crate::{
    Error, Result,
    runtime::{self, Options},
    state,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    io::{Read, Write},
    net::{Ipv4Addr, SocketAddr, TcpStream},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use uuid::Uuid;

#[derive(Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Profile {
    image: Option<String>,
    offline: bool,
    source: PathBuf,
    revision: String,
}
impl Profile {
    fn new(options: &Options) -> Result<Self> {
        Ok(Self {
            image: options.image.clone(),
            offline: options.offline,
            source: runtime::runtime_source(options)?,
            revision: crate::docker::RUNTIME_REVISION.into(),
        })
    }
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Control {
    version: u32,
    pub instance: Uuid,
    pub port: u16,
    pub secret: String,
    profile: Profile,
}
impl Control {
    pub(super) fn new(port: u16, options: &Options) -> Result<Self> {
        Ok(Self {
            version: 1,
            instance: Uuid::new_v4(),
            port,
            secret: Uuid::new_v4().to_string(),
            profile: Profile::new(options)?,
        })
    }
    fn valid(&self) -> bool {
        self.version == 1
            && self.port != 0
            && self.instance.get_version_num() == 4
            && Uuid::parse_str(&self.secret).is_ok_and(|id| id.get_version_num() == 4)
    }
    fn origin(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }
    fn link(&self, application: &str) -> String {
        let query = form_urlencoded::Serializer::new(String::new())
            .append_pair("session", &self.secret)
            .append_pair("open", application)
            .finish();
        format!("{}/#{query}", self.origin())
    }
}
fn unavailable() -> Error {
    Error::new(
        "chooser_unavailable",
        "The local workspace chooser could not start. Preserve launcher state, retry cube play, or use the ready workspace link printed above.",
    )
}
fn read(home: &Path) -> Result<Option<Control>> {
    let path = home.join("chooser.json");
    match std::fs::symlink_metadata(&path) {
        Ok(_) => {
            let control: Control = state::read_json(&path)?;
            if !control.valid() {
                return Err(unavailable());
            }
            Ok(Some(control))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(Error::io(e)),
    }
}
fn request(control: &Control, method: &str, path: &str) -> Result<(u16, Value)> {
    let timeout = Duration::from_secs(2);
    let mut stream = TcpStream::connect_timeout(
        &SocketAddr::from((Ipv4Addr::LOCALHOST, control.port)),
        timeout,
    )
    .map_err(Error::io)?;
    stream.set_read_timeout(Some(timeout)).map_err(Error::io)?;
    stream.set_write_timeout(Some(timeout)).map_err(Error::io)?;
    write!(stream,"{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nOrigin: {}\r\nAuthorization: Bearer {}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",control.port,control.origin(),control.secret).map_err(Error::io)?;
    let mut bytes = Vec::new();
    stream
        .take(65537)
        .read_to_end(&mut bytes)
        .map_err(Error::io)?;
    if bytes.len() > 65536 {
        return Err(unavailable());
    }
    let response = std::str::from_utf8(&bytes).map_err(|_| unavailable())?;
    let (head, body) = response.split_once("\r\n\r\n").ok_or_else(unavailable)?;
    let status = head
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .ok_or_else(unavailable)?;
    Ok((status, serde_json::from_str(body).unwrap_or(Value::Null)))
}
fn alive(control: &Control) -> bool {
    request(control, "GET", "/api/health").is_ok_and(|(status, value)| {
        status == 200
            && value["schema_version"] == 1
            && value["instance"] == control.instance.to_string()
    })
}
pub fn stop(home: &Path) -> Result<()> {
    let Some(control) = read(home)? else {
        return Ok(());
    };
    if !alive(&control) {
        return Ok(());
    }
    if request(&control, "POST", "/api/shutdown")?.0 != 204 {
        return Err(Error::new(
            "workspace_busy",
            "Finish the current workspace action before stopping the chooser",
        ));
    }
    let deadline = Instant::now() + Duration::from_secs(5);
    while alive(&control) {
        if Instant::now() >= deadline {
            return Err(unavailable());
        }
        std::thread::sleep(Duration::from_millis(30));
    }
    Ok(())
}
pub fn ensure(options: &Options, application: &str, cancelled: &Arc<AtomicBool>) -> Result<String> {
    let home = runtime::home(options)?;
    let _lock = state::lock(&home.join("chooser-startup.lock"), cancelled)?;
    if let Some(control) = read(&home)?
        && alive(&control)
    {
        if control.profile == Profile::new(options)? {
            return Ok(control.link(application));
        }
        stop(&home)?;
    }
    let mut command = Command::new(std::env::current_exe().map_err(Error::io)?);
    command
        .arg("--workspace-host")
        .arg("--state-home")
        .arg(&home)
        .arg("--runtime-source")
        .arg(runtime::runtime_source(options)?)
        .current_dir(&home)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if options.offline {
        command.arg("--offline");
    }
    if let Some(image) = &options.image {
        command.arg("--image").arg(image);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let profile = Profile::new(options)?;
    #[cfg(not(windows))]
    let mut child = command.spawn().map_err(Error::io)?;
    #[cfg(windows)]
    let mut child = windows::spawn(&command).map_err(Error::io)?;
    let result = (|| {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if cancelled.load(Ordering::Relaxed) || Instant::now() >= deadline {
                return Err(if cancelled.load(Ordering::Relaxed) {
                    Error::cancelled()
                } else {
                    unavailable()
                });
            }
            if child.try_wait().map_err(Error::io)?.is_some() {
                return Err(unavailable());
            }
            if let Some(control) = read(&home)?
                && control.profile == profile
                && alive(&control)
            {
                return Ok(control.link(application));
            }
            std::thread::sleep(Duration::from_millis(30));
        }
    })();
    if result.is_err() {
        let _ = child.kill();
        let _ = child.wait();
    }
    result
}
