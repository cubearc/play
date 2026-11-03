use crate::{Result, protocol};
use serde_json::{Value, json};
use std::{
    io::{BufReader, Write},
    path::Path,
    sync::mpsc::{self, Receiver},
    thread,
};

#[cfg(not(windows))]
use std::time::{Duration, Instant};

pub(super) struct Active {
    pub(super) id: u32,
    pub(super) child: crate::native::WorkerChild,
    output: Receiver<Result<Value>>,
    result: Option<Value>,
    #[cfg(not(windows))]
    deadline: Instant,
    pub(super) durable: Option<crate::workspace::RunIdentity>,
}

impl Drop for Active {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Active {
    pub(super) fn spawn(
        id: u32,
        worker: &Path,
        frame: &[u8],
        spawner: &mut crate::native::Spawner,
    ) -> Result<Self> {
        let mut child = spawner.spawn(worker)?;
        let stdout = child.stdout.take().ok_or("worker_failed")?;
        // A complete graph has two events per admitted node and one terminal frame.
        // The reader still bounds aggregate raw bytes by the existing frame ceiling.
        let (sender, output) = mpsc::sync_channel(2 * protocol::contract().pipeline_node_limit + 1);
        let mut active = Self {
            id,
            child,
            output,
            result: None,
            durable: None,
            #[cfg(not(windows))]
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
        // Keep the sole write end until the worker exits. Engine death closes it
        // in the kernel, including SIGKILL where Active::drop cannot run.
        let input = active.child.stdin.as_mut().ok_or("worker_failed")?;
        input
            .write_all(frame)
            .and_then(|()| input.flush())
            .map_err(|_| "transport_closed")?;
        Ok(active)
    }

    pub(super) fn poll(&mut self) -> Result<Option<Value>> {
        #[cfg(windows)]
        if let Some(failure) = self.child.budget_failure() {
            return Err(match failure {
                play_windows_boundary::BudgetFailure::Wall => "deadline_exceeded",
                _ => "worker_failed",
            });
        }
        #[cfg(not(windows))]
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
