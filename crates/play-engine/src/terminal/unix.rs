use super::*;
use crate::{protocol, workspace};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use rustix::{
    fs::{OFlags, fcntl_getfl, fcntl_setfl},
    pty, termios,
};
use serde_json::json;
use std::{
    fs::{File, OpenOptions},
    io::{BufReader, Read, Write},
    os::unix::{ffi::OsStrExt, net::UnixStream},
    process::{Child, Command as Process, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver},
    },
    thread,
    time::{Duration, Instant},
};

const OUTPUT_BYTES: usize = 262144;
const INPUT_BYTES: usize = 65536;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Request {
    #[serde(rename = "type")]
    kind: String,
    protocol: u32,
    nonce: String,
    context: Value,
    command: Action,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum Action {
    Asset { asset_id: Uuid, revision: i64 },
    Pipeline { pipeline_id: Uuid, revision: i64 },
    Start { request: RunRequest },
    Status { run_id: Uuid },
    Cancel { run_id: Uuid },
}

pub(super) struct Session {
    id: Uuid,
    context: Context,
    selection: Option<RunRequest>,
    nonce: String,
    child: Child,
    master: File,
    control: UnixStream,
    frames: Receiver<Result<Value>>,
    output: Receiver<Vec<u8>>,
    overflow: Arc<AtomicBool>,
    expected: Option<Value>,
    closed: Option<&'static str>,
    deadline: Instant,
    accessed: Instant,
    input_bytes: usize,
    run_id: Option<Uuid>,
    trail_id: Option<Uuid>,
    stopped: Arc<AtomicBool>,
}
impl Drop for Session {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Release);
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}
fn size(columns: u16, rows: u16) -> Result<termios::Winsize> {
    if !(20..=300).contains(&columns) || !(4..=100).contains(&rows) {
        return Err("invalid_terminal_size");
    }
    Ok(termios::Winsize {
        ws_col: columns,
        ws_row: rows,
        ws_xpixel: 0,
        ws_ypixel: 0,
    })
}
fn terminal(columns: u16, rows: u16) -> Result<(File, File)> {
    let dimensions = size(columns, rows)?;
    let master = pty::openpt(pty::OpenptFlags::RDWR | pty::OpenptFlags::NOCTTY)
        .map_err(|_| "terminal_unavailable")?;
    rustix::io::fcntl_setfd(&master, rustix::io::FdFlags::CLOEXEC)
        .map_err(|_| "terminal_unavailable")?;
    pty::grantpt(&master)
        .and_then(|()| pty::unlockpt(&master))
        .map_err(|_| "terminal_unavailable")?;
    let name = pty::ptsname(&master, Vec::new()).map_err(|_| "terminal_unavailable")?;
    let slave = OpenOptions::new()
        .read(true)
        .write(true)
        .open(std::ffi::OsStr::from_bytes(name.as_bytes()))
        .map_err(|_| "terminal_unavailable")?;
    let mut attributes = termios::tcgetattr(&slave).map_err(|_| "terminal_unavailable")?;
    attributes.make_raw();
    termios::tcsetattr(&slave, termios::OptionalActions::Now, &attributes)
        .map_err(|_| "terminal_unavailable")?;
    termios::tcsetwinsize(&master, dimensions).map_err(|_| "terminal_unavailable")?;
    let flags = fcntl_getfl(&master).map_err(|_| "terminal_unavailable")?;
    fcntl_setfl(&master, flags | OFlags::NONBLOCK).map_err(|_| "terminal_unavailable")?;
    Ok((File::from(master), slave))
}
impl Session {
    pub fn open(
        executable: &Path,
        context: Context,
        selection: Option<RunRequest>,
        columns: u16,
        rows: u16,
    ) -> Result<Self> {
        if selection.as_ref().is_some_and(|r| {
            serde_json::to_value(&r.context).ok() != serde_json::to_value(&context).ok()
        }) {
            return Err("context_mismatch");
        }
        let (master, slave) = terminal(columns, rows)?;
        let (control, peer) = UnixStream::pair().map_err(|_| "terminal_unavailable")?;
        control
            .set_write_timeout(Some(Duration::from_millis(100)))
            .map_err(|_| "terminal_unavailable")?;
        let mut reader = BufReader::new(control.try_clone().map_err(|_| "terminal_unavailable")?);
        let mut process = Process::new(executable);
        crate::native::worker_environment(&mut process);
        let child = process
            .arg("--terminal-worker")
            .stdin(Stdio::from(
                slave.try_clone().map_err(|_| "terminal_unavailable")?,
            ))
            .stderr(Stdio::from(slave))
            .stdout(Stdio::from(std::os::fd::OwnedFd::from(peer)))
            .spawn()
            .map_err(|_| "terminal_unavailable")?;
        let (send, frames) = mpsc::sync_channel(2);
        thread::spawn(move || {
            loop {
                let value = protocol::read_frame(&mut reader).and_then(|f| {
                    let f = f.ok_or("terminal_closed")?;
                    cube_core::pipeline::json::parse_with_limits(
                        std::str::from_utf8(&f).map_err(|_| "invalid_frame")?,
                        protocol::contract().frame_bytes,
                        64,
                        protocol::contract().frame_bytes,
                    )
                    .map_err(|_| "invalid_frame")
                });
                let failed = value.is_err();
                if send.send(value).is_err() || failed {
                    break;
                }
            }
        });
        let (send, output) = mpsc::sync_channel(8);
        let overflow = Arc::new(AtomicBool::new(false));
        let stopped = Arc::new(AtomicBool::new(false));
        let mut session = Self {
            id: Uuid::new_v4(),
            context,
            selection,
            nonce: String::new(),
            child,
            master,
            control,
            frames,
            output,
            overflow: overflow.clone(),
            expected: None,
            closed: None,
            deadline: Instant::now() + Duration::from_secs(300),
            accessed: Instant::now(),
            input_bytes: 0,
            run_id: None,
            trail_id: None,
            stopped: stopped.clone(),
        };
        let mut reader = session
            .master
            .try_clone()
            .map_err(|_| "terminal_unavailable")?;
        thread::spawn(move || {
            let mut total = 0;
            while !stopped.load(Ordering::Acquire) {
                let mut bytes = [0; 1024];
                match reader.read(&mut bytes) {
                    Ok(0) => break,
                    Ok(length) => {
                        total += length;
                        if total > OUTPUT_BYTES {
                            overflow.store(true, Ordering::Release);
                            break;
                        }
                        let mut pending = bytes[..length].to_vec();
                        loop {
                            match send.try_send(pending) {
                                Ok(()) => break,
                                Err(mpsc::TrySendError::Full(value))
                                    if !stopped.load(Ordering::Acquire) =>
                                {
                                    pending = value;
                                    thread::sleep(Duration::from_millis(5));
                                }
                                Err(_) => return,
                            }
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(5))
                    }
                    Err(_) => break,
                }
            }
        });
        session.bootstrap()?;
        Ok(session)
    }
    fn attached_context(&self) -> Value {
        json!({"workspace_id":self.context.workspace_id,"exploration_id":self.context.exploration_id,"stage_id":self.context.stage_id,"generation":protocol::contract().storage_generation})
    }
    fn bootstrap(&mut self) -> Result<()> {
        self.nonce = Uuid::new_v4().simple().to_string();
        if let Some(selection) = &mut self.selection {
            selection.key = Uuid::new_v4();
        }
        let context = self.attached_context();
        protocol::write_frame(
            &mut self.control,
            &json!({"type":"context","protocol":1,"cli":"0.18.0","nonce":self.nonce,"context":context,"run":self.selection}),
        )
    }
    pub fn is_open(&self) -> bool {
        self.closed.is_none()
    }
    fn stop(&mut self, code: &'static str) {
        self.closed.get_or_insert(code);
        self.frames = mpsc::channel().1;
        self.stopped.store(true, Ordering::Release);
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
    pub fn handle(&mut self, command: Command, store: &mut workspace::Manager) -> Result<()> {
        let (context, id) = match &command {
            Command::Open { .. } => return Err("terminal_busy"),
            Command::Read {
                context,
                session_id,
            }
            | Command::Write {
                context,
                session_id,
                ..
            }
            | Command::Resize {
                context,
                session_id,
                ..
            }
            | Command::Close {
                context,
                session_id,
            } => (context, *session_id),
        };
        if id != self.id
            || serde_json::to_value(context).ok() != serde_json::to_value(&self.context).ok()
        {
            return Err("context_mismatch");
        }
        store.authorize_context(context)?;
        self.accessed = Instant::now();
        match command {
            Command::Write { input, .. } => {
                if !self.is_open() {
                    return Err("terminal_closed");
                }
                self.input_bytes = self.input_bytes.saturating_add(input.len());
                if input.is_empty() || input.len() > 1024 || self.input_bytes > INPUT_BYTES {
                    self.stop("input_limit");
                    return Err("input_limit");
                }
                if self
                    .master
                    .write(input.as_bytes())
                    .map_err(|_| "terminal_closed")?
                    != input.len()
                {
                    self.stop("input_limit");
                    return Err("input_limit");
                }
            }
            Command::Resize { columns, rows, .. } => {
                termios::tcsetwinsize(&self.master, size(columns, rows)?)
                    .map_err(|_| "terminal_closed")?;
            }
            Command::Close { .. } => self.stop("closed"),
            _ => (),
        }
        Ok(())
    }
    pub fn view(&mut self) -> Result<Value> {
        let bytes = self.output.try_iter().take(8).flatten().collect::<Vec<_>>();
        Ok(
            json!({"id":self.id,"state":if self.is_open(){"open"}else{"closed"},"code":self.closed,"output":STANDARD.encode(bytes),"run_id":self.run_id.take(),"trail_id":self.trail_id.take()}),
        )
    }
    pub fn poll(&mut self) -> Result<Option<protocol::Command>> {
        if !self.is_open() {
            return Ok(None);
        }
        if Instant::now() >= self.deadline || self.accessed.elapsed() > Duration::from_secs(10) {
            self.stop("terminal_timeout");
            return Ok(None);
        }
        if self.overflow.load(Ordering::Acquire) {
            self.stop("output_limit");
            return Ok(None);
        }
        if let Some(status) = self.child.try_wait().map_err(|_| "terminal_closed")? {
            self.stop(if status.success() {
                "closed"
            } else {
                "terminal_failed"
            });
            return Ok(None);
        }
        let frame = match self.frames.try_recv() {
            Ok(Ok(frame)) => frame,
            Ok(Err(code)) => {
                self.stop(code);
                return Ok(None);
            }
            Err(_) => return Ok(None),
        };
        match self.request(frame) {
            Ok(value) => Ok(value),
            Err(code) => {
                self.stop(code);
                Ok(None)
            }
        }
    }
    fn request(&mut self, frame: Value) -> Result<Option<protocol::Command>> {
        if let Some(expected) = self.expected.take() {
            if frame != expected {
                return Err("invalid_response");
            }
            self.bootstrap()?;
            return Ok(None);
        }
        let request: Request = serde_json::from_value(frame).map_err(|_| "invalid_command")?;
        if request.kind != "request"
            || request.protocol != 1
            || request.nonce != self.nonce
            || request.context != self.attached_context()
        {
            return Err("context_mismatch");
        }
        let context = self.context.clone();
        let id = 0;
        Ok(Some(match request.command {
            Action::Asset { asset_id, revision } => protocol::Command::CliRead {
                id,
                context,
                command: workspace::Read::Asset { asset_id, revision },
            },
            Action::Pipeline {
                pipeline_id,
                revision,
            } => protocol::Command::CliRead {
                id,
                context,
                command: workspace::Read::Pipeline {
                    pipeline_id,
                    revision,
                },
            },
            Action::Status { run_id } => protocol::Command::Workspace {
                id,
                command: workspace::Command::RunDetail { context, run_id },
            },
            Action::Cancel { run_id } => protocol::Command::CancelRun {
                id,
                context,
                run_id,
            },
            Action::Start { request } => {
                if self
                    .selection
                    .as_ref()
                    .and_then(|r| serde_json::to_value(r).ok())
                    != serde_json::to_value(&request).ok()
                {
                    return Err("selection_mismatch");
                }
                protocol::Command::StartCliRun { id, request }
            }
        }))
    }
    pub fn reply(&mut self, response: Value) -> Result<()> {
        let result = if response["type"] == "rejected" {
            json!({"status":"rejected","code":response["code"]})
        } else {
            let data = &response["data"];
            if data.is_null() {
                return Err("invalid_response");
            }
            if data.get("caller").is_some() {
                self.run_id = data["id"].as_str().and_then(|id| Uuid::parse_str(id).ok());
                self.trail_id = data["trailId"]
                    .as_str()
                    .and_then(|id| Uuid::parse_str(id).ok());
            }
            json!({"status":"ok","data":data})
        };
        let mut reply = json!({"type":"reply","protocol":1,"nonce":self.nonce,"result":result});
        protocol::write_frame(&mut self.control, &reply)?;
        reply["type"] = json!("result");
        self.expected = Some(reply);
        Ok(())
    }
}
