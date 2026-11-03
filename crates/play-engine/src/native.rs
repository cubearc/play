//! Native worker policies bound task parsing and fixed thread startup.
use crate::Result;

/// Fix allocator reservation policy before startup, without inheriting host settings.
/// Per-CPU arenas can otherwise exhaust the worker's VM allowance on tiny inputs.
#[cfg(not(windows))]
pub(crate) fn worker_environment(command: &mut std::process::Command) {
    command.env_clear();
    #[cfg(target_os = "macos")]
    command.env("MallocMaxMagazines", "1");
    #[cfg(target_os = "linux")]
    command.env("MALLOC_ARENA_MAX", "1");
}

#[cfg(all(
    target_os = "linux",
    any(target_arch = "x86_64", target_arch = "aarch64")
))]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

/// Platform implementation exists; enforcement must still succeed in the worker.
pub fn available() -> bool {
    cfg!(windows)
        || cfg!(target_os = "macos")
        || cfg!(all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        ))
}

/// Called only by a fresh, single-threaded helper before receiving private data.
/// No operation runs if any required policy cannot be enforced.
pub fn restrict() -> Result<()> {
    #[cfg(windows)]
    return play_windows_boundary::current(windows_limits())
        .map_err(|_| "native_policy_unavailable");
    #[cfg(target_os = "macos")]
    return macos::restrict();
    #[cfg(all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    ))]
    return linux::restrict();
    #[cfg(not(any(
        windows,
        target_os = "macos",
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    )))]
    Err("native_policy_unavailable")
}

/// Finish bounded startup before dispatching any Core work. Linux seals the
/// current thread set in the kernel; macOS keeps its fixed software pool under
/// the already-applied native boundary and does not claim a kernel thread quota.
pub fn seal_threads() -> Result<()> {
    #[cfg(windows)]
    return play_windows_boundary::current(windows_limits())
        .map_err(|_| "native_policy_unavailable");
    #[cfg(all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    ))]
    return linux::seal_threads();
    #[cfg(target_os = "macos")]
    return Ok(());
    #[cfg(not(any(
        windows,
        target_os = "macos",
        all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        )
    )))]
    Err("native_policy_unavailable")
}

/// Terminal-only inherited PTY/control descriptors; no filesystem grants.
#[cfg(target_os = "linux")]
pub fn restrict_terminal() -> Result<()> {
    #[cfg(any(target_arch = "x86_64", target_arch = "aarch64"))]
    return linux::restrict_terminal();
    #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
    Err("native_policy_unavailable")
}

#[cfg(all(test, any(target_os = "macos", target_os = "linux")))]
mod tests {
    #[test]
    fn worker_startup_replaces_host_environment_with_fixed_allocator_policy() {
        let mut command = std::process::Command::new("/usr/bin/env");
        command.env("PRIVATE_SENTINEL", "must-not-reach-worker");
        command.env("MALLOC_ARENA_MAX", "999");
        command.env("MallocMaxMagazines", "999");
        super::worker_environment(&mut command);
        let output = command.output().unwrap();
        assert!(output.status.success());
        #[cfg(target_os = "macos")]
        assert_eq!(output.stdout, b"MallocMaxMagazines=1\n");
        #[cfg(target_os = "linux")]
        assert_eq!(output.stdout, b"MALLOC_ARENA_MAX=1\n");
    }
}

#[cfg(windows)]
fn windows_limits() -> play_windows_boundary::Limits {
    let contract = crate::protocol::contract();
    play_windows_boundary::Limits {
        private_memory_bytes: contract.private_commit_bytes as usize,
        cpu_seconds: contract.cpu_seconds,
    }
}
#[cfg(windows)]
pub(crate) type WorkerChild = play_windows_boundary::Child;
#[cfg(not(windows))]
pub(crate) type WorkerChild = std::process::Child;
pub(crate) fn spawn_worker(worker: &std::path::Path) -> Result<WorkerChild> {
    #[cfg(windows)]
    return WorkerChild::spawn(worker, &["--worker"], windows_limits())
        .map_err(|_| "native_policy_unavailable");
    #[cfg(not(windows))]
    {
        let mut command = std::process::Command::new(worker);
        worker_environment(&mut command);
        command
            .arg("--worker")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|_| "worker_failed")
    }
}

/// Snapshot only the memory accounting enforced by the current native adapter.
pub(crate) struct MemoryLimits {
    pub virtual_address_growth_bytes: Option<u64>,
    pub private_commit_bytes: Option<u64>,
}
pub(crate) fn memory_limits() -> MemoryLimits {
    let limits = crate::protocol::contract();
    MemoryLimits {
        virtual_address_growth_bytes: if cfg!(windows) {
            None
        } else {
            Some(limits.vm_growth_bytes)
        },
        private_commit_bytes: if cfg!(windows) {
            Some(limits.private_commit_bytes)
        } else {
            None
        },
    }
}
