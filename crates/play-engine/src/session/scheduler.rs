//! Own every accepted worker and frozen queued task in the session's single writer.
use super::{child::Active, queue::Queue};
use crate::{
    Result, protocol, worker,
    workspace::{Manager, PreparedRun, RunIdentity},
};
use serde_json::{Value, json};
use std::{io::Write, path::Path};
use uuid::Uuid;

struct Pending {
    id: u32,
    identity: RunIdentity,
    frame: Vec<u8>,
}

#[derive(Default)]
pub(super) struct Scheduler {
    active: Vec<Active>,
    pending: Queue<Pending>,
}

impl Scheduler {
    pub fn is_empty(&self) -> bool {
        self.active.is_empty() && self.pending.len() == 0
    }

    pub fn capacity(&self, exploration: Uuid) -> Result<()> {
        let limits = protocol::contract();
        if self.pending.len() >= limits.queued_runs
            || self.pending.count(exploration) >= limits.queued_runs_per_exploration
        {
            return Err("queue_full");
        }
        Ok(())
    }

    pub fn enqueue(&mut self, id: u32, prepared: PreparedRun) -> Result<()> {
        self.capacity(prepared.identity.context.exploration_id)?;
        let mut frame = Vec::new();
        protocol::write_frame(
            &mut frame,
            &worker::Task::Pipeline {
                invocation: prepared.invocation,
            },
        )?;
        self.pending.push(
            prepared.identity.context.exploration_id,
            Pending {
                id,
                identity: prepared.identity,
                frame,
            },
        );
        Ok(())
    }

    pub fn probe(
        &mut self,
        id: u32,
        worker: &Path,
        invocation: protocol::Invocation,
    ) -> Result<u32> {
        // Unscoped diagnostic invocations never share a session with a Workspace.
        if !self.is_empty() {
            return Err("busy");
        }
        let mut frame = Vec::new();
        protocol::write_frame(&mut frame, &worker::Task::Probe { invocation })?;
        let active = Active::spawn(id, worker, &frame)?;
        let pid = active.child.id();
        self.active.push(active);
        Ok(pid)
    }

    pub fn dispatch(&mut self, worker: &Path, store: &mut Manager) -> Result<()> {
        while self.active.len() < protocol::contract().active_runs {
            let Some(pending) = self.pending.pop(|owner| {
                self.active
                    .iter()
                    .filter(|a| {
                        a.durable
                            .as_ref()
                            .is_some_and(|d| d.context.exploration_id == owner)
                    })
                    .count()
                    < protocol::contract().active_runs_per_exploration
            }) else {
                break;
            };
            match Active::spawn(pending.id, worker, &pending.frame) {
                Ok(mut active) => {
                    active.durable = Some(pending.identity);
                    self.active.push(active);
                }
                Err(code) => {
                    store.finish_run(&pending.identity, json!({"status":"failed","code":code}))?;
                }
            }
        }
        Ok(())
    }

    pub fn cancel(&mut self, id: Uuid) -> Option<RunIdentity> {
        if let Some(index) = self
            .active
            .iter()
            .position(|a| a.durable.as_ref().is_some_and(|d| d.id == id))
        {
            let run = self.active.remove(index);
            let identity = run.durable.clone();
            drop(run); // Kill and reap only this worker before committing cancellation.
            identity
        } else {
            self.pending
                .remove(|p| p.identity.id == id)
                .map(|p| p.identity)
        }
    }

    pub fn cancel_probe(&mut self, id: u32) -> bool {
        if let Some(index) = self
            .active
            .iter()
            .position(|a| a.id == id && a.durable.is_none())
        {
            drop(self.active.remove(index));
            true
        } else {
            false
        }
    }

    pub fn poll(&mut self, store: &mut Option<Manager>, output: &mut impl Write) -> Result<()> {
        for index in (0..self.active.len()).rev() {
            let result = self.active[index].poll();
            let terminal = match result {
                Ok(Some(event))
                    if matches!(
                        event["type"].as_str(),
                        Some("node_started" | "node_finished")
                    ) =>
                {
                    progress(&self.active[index], store.as_mut(), event)
                        .err()
                        .map(failed)
                }
                Ok(event) => event,
                Err(code) => Some(failed(code)),
            };
            if let Some(result) = terminal {
                let run = self.active.remove(index);
                let identity = run.durable.clone();
                let id = run.id;
                drop(run);
                finish(id, identity, store, result, output)?;
            }
        }
        Ok(())
    }

    pub fn shutdown(
        &mut self,
        store: &mut Option<Manager>,
        code: &str,
        output: &mut impl Write,
    ) -> Result<()> {
        // Reap ALL children before SQLite or a broken reply pipe can fail cleanup.
        let mut records = Vec::new();
        for run in self.active.drain(..) {
            records.push((run.id, run.durable.clone()));
            drop(run);
        }
        while let Some(pending) = self.pending.pop(|_| true) {
            records.push((pending.id, Some(pending.identity)));
        }
        let mut outcome = Ok(());
        for (id, identity) in records {
            let result = finish(
                id,
                identity,
                store,
                json!({"status":"cancelled","code":code}),
                output,
            );
            if outcome.is_ok() {
                outcome = result;
            }
        }
        outcome
    }
}

fn failed(code: &str) -> Value {
    json!({"status":"failed","code":code})
}

fn finish(
    id: u32,
    identity: Option<RunIdentity>,
    store: &mut Option<Manager>,
    mut result: Value,
    output: &mut impl Write,
) -> Result<()> {
    if let Some(identity) = identity {
        let manager = store.as_mut().ok_or("workspace_closed")?;
        if let Err(code) = manager.finish_run(&identity, result) {
            manager.finish_run(&identity, failed(code))?;
        }
    } else {
        result["type"] = json!("finished");
        result["id"] = json!(id);
        protocol::write_frame(output, &result)?;
    }
    Ok(())
}

fn progress(run: &Active, store: Option<&mut Manager>, event: Value) -> Result<()> {
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
