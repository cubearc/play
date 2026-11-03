//! The only owner of Win32 FFI; the engine keeps `unsafe_code = forbid`.
//! No Core algorithm, domain storage, renderer authority or network permission lives here.
#[cfg(windows)]
mod handles;
#[cfg(windows)]
mod process;
#[cfg(windows)]
mod profile;
#[cfg(windows)]
mod verify;
#[cfg(windows)]
pub use process::{Child, Limits};
#[cfg(windows)]
pub use verify::current;
