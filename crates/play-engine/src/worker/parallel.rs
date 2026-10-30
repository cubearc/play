//! A bounded wave shares its enclosing worker's CPU, VM and owner lifetime.
use crate::{Result, protocol};
use std::{sync::mpsc, thread};
use uuid::Uuid;

/// Start every admitted task before collecting results in stable plan order.
/// Closed gates release unstarted threads if admission, spawn or transport fails.
pub(super) fn batch<T: Send, R: Send>(
    tasks: Vec<(Uuid, T)>,
    execute: impl Fn(T) -> Result<R> + Sync,
    mut admit: impl FnMut(Uuid) -> Result<()>,
) -> Result<Vec<Result<R>>> {
    if tasks.is_empty() || tasks.len() > protocol::contract().parallel_nodes {
        return Err("invalid_state");
    }
    thread::scope(|scope| {
        let mut handles = Vec::new();
        let mut gates = Vec::new();
        for (id, task) in tasks {
            let (gate, ready) = mpsc::channel();
            let execute = &execute;
            let handle = thread::Builder::new()
                .name("pipeline-node".into())
                .spawn_scoped(scope, move || {
                    ready.recv().map_err(|_| "operation_stopped")?;
                    execute(task)
                })
                .map_err(|_| "worker_failed")?;
            handles.push(handle);
            gates.push((id, gate));
        }
        for (id, _) in &gates {
            admit(*id)?;
        }
        for (_, gate) in gates {
            gate.send(()).map_err(|_| "worker_failed")?;
        }
        Ok(handles
            .into_iter()
            .map(|h| h.join().unwrap_or(Err("worker_failed")))
            .collect())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Condvar, Mutex};
    use std::time::Duration;

    #[test]
    fn actual_threads_overlap_and_share_a_bounded_wave() {
        let state = (Mutex::new(0), Condvar::new());
        let results = batch(
            vec![(Uuid::new_v4(), "left"), (Uuid::new_v4(), "right")],
            |text| {
                let (mutex, changed) = &state;
                let mut arrived = mutex.lock().unwrap();
                *arrived += 1;
                changed.notify_all();
                let (arrived, _) = changed
                    .wait_timeout_while(arrived, Duration::from_secs(2), |n| *n < 2)
                    .unwrap();
                if *arrived != 2 {
                    return Err("not_parallel");
                }
                drop(arrived);
                // Exercise the real operation after both executing threads rendezvous.
                cube_core::operations::execution::execute(
                    "cipher.rot13",
                    1,
                    &serde_json::json!({"input":text}),
                    &serde_json::json!({}),
                    Default::default(),
                )
                .map_err(|e| e.0)
            },
            |_| Ok(()),
        )
        .unwrap();
        assert_eq!(
            results.into_iter().collect::<Result<Vec<_>>>().unwrap(),
            vec![
                serde_json::json!({"output":"yrsg"}),
                serde_json::json!({"output":"evtug"})
            ]
        );
    }

    #[test]
    fn failed_admission_never_dispatches_and_panics_do_not_escape_supervision() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let invoked = AtomicUsize::new(0);
        assert_eq!(
            batch(
                vec![(Uuid::new_v4(), ())],
                |_| {
                    invoked.fetch_add(1, Ordering::SeqCst);
                    Ok(())
                },
                |_| Err("transport_closed")
            ),
            Err("transport_closed")
        );
        assert_eq!(invoked.load(Ordering::SeqCst), 0);
        assert_eq!(
            batch(
                vec![(Uuid::new_v4(), ())],
                |_| -> Result<()> { panic!("controlled worker panic") },
                |_| Ok(())
            )
            .unwrap(),
            vec![Err("worker_failed")]
        );
        assert_eq!(
            batch(vec![(Uuid::new_v4(), ()); 3], |_| Ok(()), |_| Ok(())),
            Err("invalid_state")
        );
    }
}
