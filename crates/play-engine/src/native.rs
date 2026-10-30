//! Worker policies apply before task parsing or thread creation.
use crate::Result;

/// Fix allocator reservation policy before startup, without inheriting host settings.
/// Per-CPU arenas can otherwise exhaust the worker's VM allowance on tiny inputs.
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
    cfg!(target_os = "macos")
        || cfg!(all(
            target_os = "linux",
            any(target_arch = "x86_64", target_arch = "aarch64")
        ))
}

/// Called only by a fresh, single-threaded helper before receiving private data.
/// No operation runs if any required policy cannot be enforced.
pub fn restrict() -> Result<()> {
    #[cfg(target_os = "macos")]
    return macos::restrict();
    #[cfg(all(
        target_os = "linux",
        any(target_arch = "x86_64", target_arch = "aarch64")
    ))]
    return linux::restrict();
    #[cfg(not(any(
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
