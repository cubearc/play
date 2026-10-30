//! Worker policies apply before task parsing or thread creation.
use crate::Result;

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
