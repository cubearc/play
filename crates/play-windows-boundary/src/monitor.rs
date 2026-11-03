//! Resource timing never waits for domain persistence or output consumption.
use super::{
    Limits,
    handles::{checked, duplicate, raw},
};
use std::{
    io,
    os::windows::io::OwnedHandle,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU8, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
use windows_sys::Win32::{
    Foundation::{FILETIME, WAIT_OBJECT_0, WAIT_TIMEOUT},
    System::{
        JobObjects::TerminateJobObject,
        Threading::{GetProcessTimes, TerminateProcess, WaitForSingleObject},
    },
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BudgetFailure {
    Cpu,
    Wall,
    Monitoring,
}
fn cpu_time(process: &OwnedHandle) -> io::Result<u64> {
    let mut created = FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    let mut ended = created;
    let mut kernel = created;
    let mut user = created;
    // SAFETY: a live owning query handle and distinct writable FILETIME outputs.
    checked(unsafe {
        GetProcessTimes(
            raw(process),
            &mut created,
            &mut ended,
            &mut kernel,
            &mut user,
        )
    })?;
    let ticks = |t: FILETIME| (u64::from(t.dwHighDateTime) << 32) | u64::from(t.dwLowDateTime);
    ticks(user)
        .checked_add(ticks(kernel))
        .ok_or_else(|| io::Error::other("invalid CPU accounting"))
}
fn exceeded(process: &OwnedHandle, limits: Limits, start: Instant) -> io::Result<Option<u8>> {
    if start.elapsed() >= Duration::from_millis(limits.wall_ms) {
        return Ok(Some(3));
    }
    if cpu_time(process)? >= limits.cpu_seconds * 10_000_000 {
        return Ok(Some(2));
    }
    Ok(None)
}
fn watch(
    process: OwnedHandle,
    job: OwnedHandle,
    limits: Limits,
    start: Instant,
    stop: Arc<AtomicBool>,
    status: Arc<AtomicU8>,
) {
    while !stop.load(Ordering::Acquire) {
        // SAFETY: the monitor holds its own non-inheritable process handle throughout this wait.
        let waited = unsafe { WaitForSingleObject(raw(&process), 5) };
        if waited == WAIT_OBJECT_0 {
            status.store(1, Ordering::Release);
            return;
        }
        let failure = if waited == WAIT_TIMEOUT {
            exceeded(&process, limits, start).unwrap_or(Some(4))
        } else {
            Some(4)
        };
        if let Some(code) = failure {
            status.store(code, Ordering::Release);
            // SAFETY: both handles refer only to this worker and its private non-breakaway job.
            unsafe {
                TerminateJobObject(raw(&job), 1);
                TerminateProcess(raw(&process), 1);
            }
            return;
        }
    }
}
pub(crate) struct Monitor {
    stop: Arc<AtomicBool>,
    status: Arc<AtomicU8>,
    thread: Option<JoinHandle<()>>,
}
impl Monitor {
    pub fn new(process: &OwnedHandle, job: &OwnedHandle, limits: Limits) -> io::Result<Self> {
        let process = duplicate(process)?;
        let job = duplicate(job)?;
        let stop = Arc::new(AtomicBool::new(false));
        let status = Arc::new(AtomicU8::new(0));
        let (thread_stop, thread_status) = (stop.clone(), status.clone());
        let start = Instant::now();
        let thread = thread::Builder::new()
            .name("native-worker-budget".into())
            .spawn(move || watch(process, job, limits, start, thread_stop, thread_status))?;
        Ok(Self {
            stop,
            status,
            thread: Some(thread),
        })
    }
    pub fn failure(&self) -> Option<BudgetFailure> {
        match self.status.load(Ordering::Acquire) {
            0 | 1 => None,
            2 => Some(BudgetFailure::Cpu),
            3 => Some(BudgetFailure::Wall),
            _ => Some(BudgetFailure::Monitoring),
        }
    }
}
impl Drop for Monitor {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}
