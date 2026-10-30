mod child;
mod queue;
mod scheduler;
use scheduler::Scheduler;

use crate::{
    Result,
    protocol::{self, Command, Invocation},
};
use serde_json::{Value, json};
use std::{
    io::{self, BufReader, Write},
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, RecvTimeoutError},
    thread,
    time::Duration,
};

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
    active: &mut Scheduler,
    output: &mut impl Write,
) -> Result<()> {
    if let Err(code) = crate::worker::validate(&invocation) {
        return reject(output, id, code);
    }
    let Some(worker) = worker.filter(|_| crate::native::available()) else {
        return reject(output, id, "native_policy_unavailable");
    };
    match active.probe(id, worker, invocation) {
        Ok(worker_pid) => protocol::write_frame(
            output,
            &json!({"type":"started", "id":id, "worker_pid":worker_pid}),
        ),
        Err(code) => reject(output, id, code),
    }
}

fn dispatch(
    command: Command,
    worker: Option<&Path>,
    active: &mut Scheduler,
    store: &mut Option<crate::workspace::Manager>,
    terminals: &mut crate::terminal::Manager,
    terminal_path: Option<&Path>,
    output: &mut impl Write,
) -> Result<()> {
    let id = command.id();
    let cli = matches!(command, Command::StartCliRun { .. });
    match command {
        Command::Terminal { command, .. } => {
            let result = store
                .as_mut()
                .ok_or("workspace_unavailable")
                .and_then(|store| terminals.handle(command, terminal_path, store));
            match result {
                Ok(data) => {
                    protocol::write_frame(output, &json!({"type":"terminal","id":id,"data":data}))
                }
                Err(code) => reject(output, id, code),
            }
        }
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
            if let Err(code) = active.capacity(request.context.exploration_id) {
                return reject(output, id, code);
            }
            let prepared = match if cli {
                manager.prepare_cli_run(&request)
            } else {
                manager.prepare_run(&request)
            } {
                Ok(value) => value,
                Err(code) => return reject(output, id, code),
            };
            let identity = prepared.identity.clone();
            if let Err(code) = active.enqueue(id, prepared) {
                manager.finish_run(&identity, json!({"status":"failed","code":code}))?;
            }
            active.dispatch(worker, manager)?;
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
                let Some(identity) = active.cancel(run_id) else {
                    return reject(output, id, "not_running");
                };
                let data = manager.finish_run(
                    &identity,
                    json!({"status":"cancelled","code":"user_cancelled"}),
                )?;
                return protocol::write_frame(output, &json!({"type":"run","id":id,"data":data}));
            }
            protocol::write_frame(output, &json!({"type":"run","id":id,"data":detail}))
        }
        Command::Workspace { command, .. } => {
            let switching = matches!(
                command,
                crate::workspace::Command::Open { .. } | crate::workspace::Command::Create { .. }
            );
            if switching && !active.is_empty() {
                return reject(output, id, "busy");
            }
            if switching {
                terminals.close();
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
            if !active.cancel_probe(run_id) {
                return reject(output, id, "not_running");
            }
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

pub fn run(
    worker: Option<PathBuf>,
    root: Option<PathBuf>,
    terminal: Option<PathBuf>,
) -> Result<()> {
    if worker
        .as_ref()
        .is_some_and(|path| !path.is_absolute() || !path.is_file())
    {
        return Err("invalid_worker");
    }
    if terminal
        .as_ref()
        .is_some_and(|path| !path.is_absolute() || !path.is_file())
    {
        return Err("invalid_terminal");
    }
    let mut terminals = crate::terminal::Manager::default();
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
    let mut active = Scheduler::default();
    let result = (|| {
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
                            &mut terminals,
                            terminal.as_deref(),
                            &mut output,
                        )?;
                    }
                }
                Ok(None) => (),
                Err("transport_closed") => return Ok(()),
                Err(code) => {
                    reject(&mut output, 0, code)?;
                    return Err(code);
                }
            }
            if let Some(command) = terminals.poll()? {
                // Only the closed scoped CLI variants can enter this route. Reuse
                // the same admission, ownership, cancellation and persistence code.
                let mut reply = Vec::new();
                dispatch(
                    command,
                    worker.as_deref(),
                    &mut active,
                    &mut store,
                    &mut terminals,
                    terminal.as_deref(),
                    &mut reply,
                )?;
                let response = serde_json::from_slice(&reply).map_err(|_| "invalid_response")?;
                if terminals.reply(response).is_err() {
                    terminals.close();
                }
            }
            active.poll(&mut store, &mut output)?;
            if let (Some(worker), Some(store)) = (worker.as_deref(), store.as_mut()) {
                active.dispatch(worker, store)?;
            }
        }
    })();
    terminals.close();
    let code = result.as_ref().err().copied().unwrap_or("session_closed");
    let cleanup = active.shutdown(&mut store, code, &mut output);
    result.and(cleanup)
}
