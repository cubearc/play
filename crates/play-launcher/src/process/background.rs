//! Long-lived trusted helpers must not retain the invoking CLI's output pipes.
use std::{ffi::OsString, io, path::Path};

#[cfg(not(windows))]
type NativeChild = std::process::Child;
#[cfg(windows)]
type NativeChild = win_custody::Process;

/// An owned handle to a trusted helper that may outlive its launcher.
#[derive(Debug)]
pub struct BackgroundChild(NativeChild);

impl BackgroundChild {
    /// Check completion without blocking.
    pub fn has_exited(&mut self) -> io::Result<bool> {
        #[cfg(windows)]
        {
            self.0.exit_code().map(|status| status.is_some())
        }
        #[cfg(not(windows))]
        {
            self.0.try_wait().map(|status| status.is_some())
        }
    }

    /// Stop a failed startup and reap it, with a bounded Windows wait.
    pub fn terminate(&mut self) -> io::Result<()> {
        #[cfg(windows)]
        {
            let _ = self.0.terminate(1);
            match self.0.wait(Some(std::time::Duration::from_secs(5)))? {
                win_custody::WaitOutcome::TimedOut => Err(io::Error::new(
                    io::ErrorKind::TimedOut,
                    "Background helper did not terminate",
                )),
                _ => Ok(()),
            }
        }
        #[cfg(not(windows))]
        {
            let _ = self.0.kill();
            self.0.wait().map(|_| ())
        }
    }
}

/// Spawn a trusted absolute executable without inheriting caller output pipes.
/// Dropping its handle leaves the background helper running.
pub fn spawn_background(
    executable: &Path,
    arguments: &[OsString],
    directory: &Path,
) -> io::Result<BackgroundChild> {
    if !executable.is_absolute() || !directory.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Background helpers require trusted absolute paths",
        ));
    }
    #[cfg(windows)]
    {
        // Stable std::process::Command inherits unrelated inheritable handles
        // even when all three standard streams are redirected to NUL.
        win_custody::Launch::new(executable)
            .args(arguments)
            .current_dir(directory)
            .spawn()
            .map(BackgroundChild)
    }
    #[cfg(not(windows))]
    {
        use std::process::{Command, Stdio};
        Command::new(executable)
            .args(arguments)
            .current_dir(directory)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map(BackgroundChild)
    }
}
