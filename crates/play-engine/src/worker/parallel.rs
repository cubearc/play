//! One bounded node pool shares its enclosing worker's CPU, VM and owner lifetime.
use crate::{Result, protocol};
use std::{
    panic::{AssertUnwindSafe, catch_unwind},
    sync::mpsc,
    thread,
};
use uuid::Uuid;

struct Slot<T, R> {
    input: mpsc::SyncSender<T>,
    output: mpsc::Receiver<Result<R>>,
}
pub(super) struct Pool<T, R> {
    slots: Vec<Slot<T, R>>,
    failed: bool,
}
impl<T, R> Pool<T, R> {
    /// Admit every node before dispatch; collect its actual result in plan order.
    pub(super) fn batch(
        &mut self,
        tasks: Vec<(Uuid, T)>,
        mut admit: impl FnMut(Uuid) -> Result<()>,
    ) -> Result<Vec<Result<R>>> {
        if self.failed || tasks.is_empty() || tasks.len() > self.slots.len() {
            return Err("invalid_state");
        }
        for (id, _) in &tasks {
            admit(*id)?;
        }
        let count = tasks.len();
        for (slot, (_, task)) in self.slots.iter().zip(tasks) {
            if slot.input.send(task).is_err() {
                self.failed = true;
                return Err("worker_failed");
            }
        }
        let mut results = Vec::with_capacity(count);
        for slot in self.slots.iter().take(count) {
            match slot.output.recv() {
                Ok(value) => results.push(value),
                Err(_) => {
                    self.failed = true;
                    return Err("worker_failed");
                }
            }
        }
        Ok(results)
    }
}

/// Prestart the fixed pool, apply the host policy, then permit bounded work.
/// Dropping all senders releases idle threads on every return path before join.
pub(super) fn with_pool<T: Send, R: Send, O>(
    execute: impl Fn(T) -> Result<R> + Sync,
    seal: impl FnOnce() -> Result<()>,
    body: impl FnOnce(&mut Pool<T, R>) -> Result<O>,
) -> Result<O> {
    thread::scope(|scope| {
        let mut slots = Vec::new();
        for _ in 0..protocol::contract().parallel_nodes {
            let (input, tasks) = mpsc::sync_channel(0);
            let (output, results) = mpsc::sync_channel(1);
            let execute = &execute;
            thread::Builder::new()
                .name("pipeline-node".into())
                .spawn_scoped(scope, move || {
                    while let Ok(task) = tasks.recv() {
                        let value = catch_unwind(AssertUnwindSafe(|| execute(task)))
                            .unwrap_or(Err("worker_failed"));
                        if output.send(value).is_err() {
                            break;
                        }
                    }
                })
                .map_err(|_| "worker_failed")?;
            slots.push(Slot {
                input,
                output: results,
            });
        }
        let mut pool = Pool {
            slots,
            failed: false,
        };
        seal()?;
        body(&mut pool)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn batch<T: Send, R: Send>(
        tasks: Vec<(Uuid, T)>,
        execute: impl Fn(T) -> Result<R> + Sync,
        admit: impl FnMut(Uuid) -> Result<()>,
    ) -> Result<Vec<Result<R>>> {
        with_pool(execute, || Ok(()), |pool| pool.batch(tasks, admit))
    }
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
    #[test]
    fn one_worker_reuses_the_same_bounded_threads_across_dependency_waves() {
        let ids = with_pool(
            |_: ()| Ok(std::thread::current().id()),
            || Ok(()),
            |pool| {
                let mut ids = std::collections::HashSet::new();
                for _ in 0..4 {
                    for result in pool.batch(
                        vec![(Uuid::new_v4(), ()); protocol::contract().parallel_nodes],
                        |_| Ok(()),
                    )? {
                        ids.insert(result?);
                    }
                }
                Ok(ids)
            },
        )
        .unwrap();
        assert_eq!(
            ids.len(),
            protocol::contract().parallel_nodes,
            "a worker must create its bounded threads once, before sealing further creation"
        );
    }
    #[test]
    fn rejected_startup_policy_joins_idle_threads_without_executing_or_admitting_work() {
        use std::sync::atomic::{AtomicBool, Ordering};
        let invoked = AtomicBool::new(false);
        let admitted = AtomicBool::new(false);
        let result = with_pool(
            |_: ()| {
                invoked.store(true, Ordering::SeqCst);
                Ok(())
            },
            || Err("thread_policy_failed"),
            |pool| {
                pool.batch(vec![(Uuid::new_v4(), ())], |_| {
                    admitted.store(true, Ordering::SeqCst);
                    Ok(())
                })
            },
        );
        assert_eq!(result, Err("thread_policy_failed"));
        assert!(!invoked.load(Ordering::SeqCst));
        assert!(!admitted.load(Ordering::SeqCst));
    }
}
