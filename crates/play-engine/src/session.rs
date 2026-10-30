use crate::{
    Result,
    protocol::{self, Command, Invocation},
};
use serde_json::{Value, json};
use std::{
    io::{self, BufReader, Write},
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
    durable: Option<crate::workspace::RunIdentity>,
}

impl Drop for Active {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Active {
    fn spawn(id: u32, worker: &Path, task: &crate::worker::Task) -> Result<Self> {
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
            durable: None,
            deadline: Instant::now() + Duration::from_millis(protocol::contract().run_ms),
        };
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut total = 0usize;
            loop {
                let frame = match protocol::read_frame(&mut reader) {
                    Ok(Some(frame)) => frame,
                    Ok(None) => break,
                    Err(code) => {
                        let _ = sender.send(Err(code));
                        break;
                    }
                };
                total = total.saturating_add(frame.len());
                if total > protocol::contract().frame_bytes {
                    let _ = sender.send(Err("output_limit"));
                    break;
                }
                let parsed = std::str::from_utf8(&frame)
                    .map_err(|_| "worker_failed")
                    .and_then(|s| cube_core::pipeline::json::parse(s).map_err(|_| "worker_failed"));
                if sender.send(parsed).is_err() {
                    break;
                }
            }
        });
        // Closing stdin is the worker's complete input boundary, including EOF.
        let mut input = active.child.stdin.take().ok_or("worker_failed")?;
        protocol::write_frame(&mut input, task)?;
        drop(input);
        Ok(active)
    }

    fn poll(&mut self) -> Result<Option<Value>> {
        if Instant::now() >= self.deadline {
            return Err("deadline_exceeded");
        }
        if let Ok(result) = self.output.try_recv() {
            let result = result?;
            if matches!(
                result["type"].as_str(),
                Some("node_started" | "node_finished")
            ) {
                return Ok(Some(result));
            }
            self.result = Some(result);
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

fn launch(
    id: u32,
    invocation: Invocation,
    worker: Option<&Path>,
    active: &mut Option<Active>,
    output: &mut impl Write,
) -> Result<()> {
    if let Err(code) = crate::worker::validate(&invocation) {
        return reject(output, id, code);
    }
    let Some(worker) = worker.filter(|_| crate::native::available()) else {
        return reject(output, id, "native_policy_unavailable");
    };
    if active.is_some() {
        return reject(output, id, "busy");
    }
    match Active::spawn(id, worker, &crate::worker::Task::Probe { invocation }) {
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

fn dispatch(
    command: Command,
    worker: Option<&Path>,
    active: &mut Option<Active>,
    store: &mut Option<crate::workspace::Manager>,
    output: &mut impl Write,
) -> Result<()> {
    let id = command.id();
    let cli = matches!(command, Command::StartCliRun { .. });
    match command {
        Command::Hello { .. } => reject(output, id, "already_negotiated"),
        Command::Catalog { .. } => protocol::write_frame(
            output,
            &json!({"type":"catalog", "id":id, "operations":crate::worker::catalog()?}),
        ),
        Command::Invoke { invocation, .. } => {
            if store.is_some() {
                return reject(output, id, "scoped_invocation_required");
            }
            launch(id, invocation, worker, active, output)
        }
        Command::StartRun { request, .. } | Command::StartCliRun { request, .. } => {
            let Some(manager) = store.as_mut() else {
                return reject(output, id, "workspace_unavailable");
            };
            match if cli {
                manager.existing_cli_run(&request)
            } else {
                manager.existing_run(&request)
            } {
                Ok(Some(data)) => {
                    return protocol::write_frame(
                        output,
                        &json!({"type":"run","id":id,"data":data}),
                    );
                }
                Err(code) => return reject(output, id, code),
                Ok(None) => (),
            }
            let Some(worker) = worker.filter(|_| crate::native::available()) else {
                return reject(output, id, "native_policy_unavailable");
            };
            if active.is_some() {
                return reject(output, id, "busy");
            }
            let prepared = match if cli {
                manager.prepare_cli_run(&request)
            } else {
                manager.prepare_run(&request)
            } {
                Ok(value) => value,
                Err(code) => return reject(output, id, code),
            };
            let identity = prepared.identity;
            match Active::spawn(
                id,
                worker,
                &crate::worker::Task::Pipeline {
                    invocation: prepared.invocation,
                },
            ) {
                Ok(mut child) => {
                    child.durable = Some(identity.clone());
                    *active = Some(child);
                }
                Err(code) => {
                    let data =
                        manager.finish_run(&identity, json!({"status":"failed","code":code}))?;
                    return protocol::write_frame(
                        output,
                        &json!({"type":"run","id":id,"data":data}),
                    );
                }
            }
            let data = manager.run_detail(&identity.context, identity.id)?;
            protocol::write_frame(output, &json!({"type":"run","id":id,"data":data}))
        }
        Command::CliRead {
            context, command, ..
        } => {
            match store
                .as_mut()
                .ok_or("workspace_unavailable")
                .and_then(|store| store.cli_read(&context, command))
            {
                Ok(data) => {
                    protocol::write_frame(output, &json!({"type":"workspace","id":id,"data":data}))
                }
                Err(code) => reject(output, id, code),
            }
        }
        Command::CancelRun {
            context, run_id, ..
        } => {
            let Some(manager) = store.as_mut() else {
                return reject(output, id, "workspace_unavailable");
            };
            let detail = match manager.run_detail(&context, run_id) {
                Ok(value) => value,
                Err(code) => return reject(output, id, code),
            };
            if matches!(detail["status"].as_str(), Some("queued" | "running")) {
                let Some(identity) = active
                    .as_ref()
                    .and_then(|run| run.durable.clone())
                    .filter(|run| run.id == run_id)
                else {
                    return reject(output, id, "not_running");
                };
                drop(active.take());
                let data = manager.finish_run(
                    &identity,
                    json!({"status":"cancelled","code":"user_cancelled"}),
                )?;
                return protocol::write_frame(output, &json!({"type":"run","id":id,"data":data}));
            }
            protocol::write_frame(output, &json!({"type":"run","id":id,"data":detail}))
        }
        Command::Workspace { command, .. } => {
            if active.is_some()
                && matches!(
                    command,
                    crate::workspace::Command::Open { .. }
                        | crate::workspace::Command::Create { .. }
                )
            {
                return reject(output, id, "busy");
            }
            match store
                .as_mut()
                .ok_or("workspace_unavailable")
                .and_then(|store| store.handle(command))
            {
                Ok(data) => {
                    protocol::write_frame(output, &json!({"type":"workspace","id":id,"data":data}))
                }
                Err(code) => reject(output, id, code),
            }
        }
        Command::Cancel { run_id, .. } => {
            if store.is_some() {
                return reject(output, id, "scoped_cancel_required");
            }
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

pub fn run(worker: Option<PathBuf>, root: Option<PathBuf>) -> Result<()> {
    if worker
        .as_ref()
        .is_some_and(|path| !path.is_absolute() || !path.is_file())
    {
        return Err("invalid_worker");
    }
    let mut store = root.map(crate::workspace::Manager::new).transpose()?;
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
                    dispatch(
                        command,
                        worker.as_deref(),
                        &mut active,
                        &mut store,
                        &mut output,
                    )?;
                }
            }
            Ok(None) => (),
            Err("transport_closed") => {
                finish_active(
                    &mut active,
                    &mut store,
                    json!({"status":"cancelled","code":"session_closed"}),
                    &mut output,
                )?;
                return Ok(());
            }
            Err(code) => {
                reject(&mut output, 0, code)?;
                return Err(code);
            }
        }
        if let Some(run) = &mut active {
            match run.poll() {
                Ok(Some(response)) => {
                    if matches!(
                        response["type"].as_str(),
                        Some("node_started" | "node_finished")
                    ) {
                        if let Err(code) = progress(run, store.as_mut(), response) {
                            finish_active(
                                &mut active,
                                &mut store,
                                json!({"status":"failed","code":code}),
                                &mut output,
                            )?;
                        }
                    } else {
                        finish_active(&mut active, &mut store, response, &mut output)?;
                    }
                }
                Ok(None) => (),
                Err(code) => finish_active(
                    &mut active,
                    &mut store,
                    json!({"status":"failed","code":code}),
                    &mut output,
                )?,
            }
        }
    }
}

fn finish_active(
    active: &mut Option<Active>,
    store: &mut Option<crate::workspace::Manager>,
    mut result: Value,
    output: &mut impl Write,
) -> Result<()> {
    let Some(run) = active.take() else {
        return Ok(());
    };
    let id = run.id;
    let durable = run.durable.clone();
    drop(run);
    if let Some(identity) = durable {
        let manager = store.as_mut().ok_or("workspace_closed")?;
        if let Err(code) = manager.finish_run(&identity, result) {
            manager.finish_run(&identity, json!({"status":"failed","code":code}))?;
        }
    } else {
        result["type"] = json!("finished");
        result["id"] = json!(id);
        protocol::write_frame(output, &result)?;
    }
    Ok(())
}

fn progress(
    run: &Active,
    store: Option<&mut crate::workspace::Manager>,
    event: Value,
) -> Result<()> {
    let mut identity = run.durable.clone().ok_or("worker_failed")?;
    identity.node_id =
        serde_json::from_value(event["nodeId"].clone()).map_err(|_| "worker_failed")?;
    let manager = store.ok_or("workspace_closed")?;
    match event["type"].as_str() {
        Some("node_started") => manager.start_attempt(&identity),
        Some("node_finished") => manager.finish_attempt(&identity, event),
        _ => Err("worker_failed"),
    }
}
