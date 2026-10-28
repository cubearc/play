use crate::{
    Result,
    protocol::{self, Command, Invocation},
};
use serde_json::{Value, json};
use std::{
    io::{self, BufReader, Read, Write},
    path::{Path, PathBuf},
    process::{Child, Command as Process, Stdio},
    sync::mpsc::{self, Receiver, RecvTimeoutError},
    thread,
    time::{Duration, Instant},
};

struct Active {
    id: u32,
    child: Child,
    output: Receiver<Result<Value>>,
    result: Option<Value>,
    deadline: Instant,
}

impl Drop for Active {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Active {
    fn spawn(id: u32, worker: &Path, invocation: &Invocation) -> Result<Self> {
        let mut child = Process::new(worker)
            .arg("--worker")
            .env_clear()
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| "worker_failed")?;
        let stdout = child.stdout.take().ok_or("worker_failed")?;
        let (sender, output) = mpsc::sync_channel(1);
        let mut active = Self {
            id,
            child,
            output,
            result: None,
            deadline: Instant::now() + Duration::from_millis(protocol::contract().run_ms),
        };
        thread::spawn(move || {
            let result = (|| {
                let mut bytes = Vec::new();
                stdout
                    .take(protocol::contract().frame_bytes as u64 + 1)
                    .read_to_end(&mut bytes)
                    .map_err(|_| "worker_failed")?;
                if bytes.len() > protocol::contract().frame_bytes {
                    return Err("output_limit");
                }
                let text = std::str::from_utf8(&bytes).map_err(|_| "worker_failed")?;
                cube_core::pipeline::json::parse(text).map_err(|_| "worker_failed")
            })();
            let _ = sender.send(result);
        });
        // Closing stdin is the worker's complete input boundary, including EOF.
        let mut input = active.child.stdin.take().ok_or("worker_failed")?;
        protocol::write_frame(&mut input, invocation)?;
        drop(input);
        Ok(active)
    }

    fn poll(&mut self) -> Result<Option<Value>> {
        if Instant::now() >= self.deadline {
            return Err("deadline_exceeded");
        }
        if let Ok(result) = self.output.try_recv() {
            self.result = Some(result?);
        }
        if let Some(status) = self.child.try_wait().map_err(|_| "worker_failed")? {
            if !status.success() {
                return Err("worker_failed");
            }
            if let Some(mut result) = self.result.take() {
                if !result.is_object()
                    || !matches!(result["status"].as_str(), Some("completed" | "failed"))
                {
                    return Err("worker_failed");
                }
                result["type"] = json!("finished");
                result["id"] = json!(self.id);
                return Ok(Some(result));
            }
        }
        Ok(None)
    }
}

fn ready(command: Command) -> Result<(u32, Value)> {
    let Command::Hello {
        id,
        protocol,
        engine,
        nonce,
    } = command
    else {
        return Err("handshake_required");
    };
    let contract = protocol::contract();
    if protocol != contract.protocol || engine != contract.engine {
        return Err("binary_mismatch");
    }
    if id == 0 || nonce.len() != 32 || !nonce.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("invalid_handshake");
    }
    Ok((
        id,
        json!({"type":"ready", "id":id, "protocol":protocol, "engine":engine, "nonce":nonce}),
    ))
}

fn receive() -> Receiver<Result<Option<Vec<u8>>>> {
    let (sender, receiver) = mpsc::sync_channel(protocol::contract().queue_commands);
    thread::spawn(move || {
        let mut input = BufReader::new(io::stdin().lock());
        loop {
            let frame = protocol::read_frame(&mut input);
            let end = !matches!(&frame, Ok(Some(_)));
            if sender.send(frame).is_err() || end {
                break;
            }
        }
    });
    receiver
}

fn next(
    receiver: &Receiver<Result<Option<Vec<u8>>>>,
    timeout: Duration,
) -> Result<Option<Command>> {
    match receiver.recv_timeout(timeout) {
        Ok(Ok(Some(frame))) => protocol::parse(&frame).map(Some),
        Ok(Ok(None)) | Err(RecvTimeoutError::Disconnected) => Err("transport_closed"),
        Ok(Err(code)) => Err(code),
        Err(RecvTimeoutError::Timeout) => Ok(None),
    }
}

fn reject(output: &mut impl Write, id: u32, code: &str) -> Result<()> {
    protocol::write_frame(output, &json!({"type":"rejected", "id":id, "code":code}))
}

fn dispatch(
    command: Command,
    worker: Option<&Path>,
    active: &mut Option<Active>,
    output: &mut impl Write,
) -> Result<()> {
    let id = command.id();
    match command {
        Command::Hello { .. } => reject(output, id, "already_negotiated"),
        Command::Catalog { .. } => protocol::write_frame(
            output,
            &json!({"type":"catalog", "id":id, "operations":crate::worker::catalog()?}),
        ),
        Command::Invoke { invocation, .. } => {
            if let Err(code) = crate::worker::validate(&invocation) {
                return reject(output, id, code);
            }
            let Some(worker) = worker.filter(|_| crate::native::available()) else {
                return reject(output, id, "native_policy_unavailable");
            };
            if active.is_some() {
                return reject(output, id, "busy");
            }
            match Active::spawn(id, worker, &invocation) {
                Ok(run) => {
                    let worker_pid = run.child.id();
                    *active = Some(run);
                    protocol::write_frame(
                        output,
                        &json!({"type":"started", "id":id, "worker_pid":worker_pid}),
                    )
                }
                Err(code) => reject(output, id, code),
            }
        }
        Command::Cancel { run_id, .. } => {
            if active.as_ref().is_none_or(|run| run.id != run_id) {
                return reject(output, id, "not_running");
            }
            drop(active.take()); // Kill and reap before announcing cancellation.
            protocol::write_frame(
                output,
                &json!({"type":"finished", "id":run_id, "status":"cancelled"}),
            )?;
            protocol::write_frame(
                output,
                &json!({"type":"cancelled", "id":id, "run_id":run_id}),
            )
        }
    }
}

pub fn run(worker: Option<PathBuf>) -> Result<()> {
    if worker
        .as_ref()
        .is_some_and(|path| !path.is_absolute() || !path.is_file())
    {
        return Err("invalid_worker");
    }
    let receiver = receive();
    let mut output = io::stdout().lock();
    let command = next(
        &receiver,
        Duration::from_millis(protocol::contract().handshake_ms),
    )?
    .ok_or("handshake_timeout")?;
    let (mut previous_id, response) = match ready(command) {
        Ok(value) => value,
        Err(code) => {
            reject(&mut output, 0, code)?;
            return Err(code);
        }
    };
    protocol::write_frame(&mut output, &response)?;
    let mut active: Option<Active> = None;
    loop {
        // Commands get the first chance to cancel an in-flight worker.
        match next(&receiver, Duration::from_millis(5)) {
            Ok(Some(command)) => {
                let id = command.id();
                if id <= previous_id {
                    reject(&mut output, id, "invalid_request_id")?;
                } else {
                    previous_id = id;
                    dispatch(command, worker.as_deref(), &mut active, &mut output)?;
                }
            }
            Ok(None) => (),
            Err("transport_closed") => return Ok(()),
            Err(code) => {
                reject(&mut output, 0, code)?;
                return Err(code);
            }
        }
        if let Some(run) = &mut active {
            match run.poll() {
                Ok(Some(response)) => {
                    drop(active.take());
                    protocol::write_frame(&mut output, &response)?;
                }
                Ok(None) => (),
                Err(code) => {
                    let id = run.id;
                    drop(active.take());
                    protocol::write_frame(
                        &mut output,
                        &json!({"type":"finished", "id":id, "status":"failed", "code":code}),
                    )?;
                }
            }
        }
    }
}
