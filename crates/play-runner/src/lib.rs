//! Typed private transport. The runner executes the versioned CLI, never algorithms.
pub mod protocol;
#[cfg(target_os = "linux")]
pub mod runner;
#[cfg(unix)]
pub mod transport;
