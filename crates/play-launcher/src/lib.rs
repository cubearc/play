//! Host-side runtime ownership. No investigation algorithms execute here.
pub mod docker;
pub mod process;
mod runner;
pub mod state;

use std::fmt;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Error {
    pub code: &'static str,
    pub message: &'static str,
}
impl Error {
    pub fn new(code: &'static str, message: &'static str) -> Self {
        Self { code, message }
    }
    pub fn io(_: std::io::Error) -> Self {
        Self::new(
            "io",
            "Local file access failed; check permissions, available space and the selected directory",
        )
    }
    pub fn cancelled() -> Self {
        Self::new(
            "cancelled",
            "Cube Play startup was interrupted; investigation data was preserved",
        )
    }
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message)
    }
}
impl std::error::Error for Error {}
