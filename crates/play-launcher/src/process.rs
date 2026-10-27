use std::{
    ffi::OsStr,
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use crate::{Error, Result};

const OUTPUT_LIMIT: u64 = 256 * 1024;

/// Search only absolute host PATH entries, never relative entries or workspace binaries.
pub fn executable(
    name: &str,
    explicit: Option<PathBuf>,
    workspace: Option<&Path>,
) -> Result<PathBuf> {
    if let Some(path) = explicit {
        if !path.is_absolute() {
            return Err(Error::new(
                "invalid_parameter",
                "Executable overrides require a trusted absolute path",
            ));
        }
        return path
            .canonicalize()
            .ok()
            .filter(|path| path.is_file())
            .ok_or_else(|| {
                Error::new(
                    "missing_executable",
                    "The configured executable is unavailable; check its absolute path",
                )
            });
    }
    let cwd = std::env::current_dir().ok();
    for entry in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()) {
        if !entry.is_absolute() {
            continue;
        }
        let candidate = entry.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
        if let Ok(candidate) = candidate.canonicalize()
            && candidate.is_file()
            && !workspace.is_some_and(|root| crate::state::contains_path(root, &candidate))
            && candidate.parent() != cwd.as_deref()
        {
            return Ok(candidate);
        }
    }
    Err(Error::new(
        "missing_executable",
        "Required host executable is unavailable; install it in a trusted directory on PATH",
    ))
}

#[derive(Clone)]
pub struct HostProcess {
    pub executable: PathBuf,
    pub cancelled: Arc<AtomicBool>,
}

#[derive(Debug)]
pub struct Output {
    pub success: bool,
    pub stdout: Vec<u8>,
}

fn drain(mut stream: impl Read) -> io::Result<(Vec<u8>, bool)> {
    let mut output = Vec::new();
    (&mut stream)
        .take(OUTPUT_LIMIT + 1)
        .read_to_end(&mut output)?;
    let exceeded = output.len() as u64 > OUTPUT_LIMIT;
    if exceeded {
        io::copy(&mut stream, &mut io::sink())?;
        output.truncate(OUTPUT_LIMIT as usize);
    }
    Ok((output, exceeded))
}

fn terminate(child: &mut dyn process_wrap::std::ChildWrapper) {
    #[cfg(windows)]
    {
        // Terminate the complete job, then reap its immediate process. The job
        // wrapper's try_wait already consumes completion notifications; waiting
        // for another notification after termination can block indefinitely.
        let _ = child.start_kill();
        let _ = child.inner_mut().wait();
    }
    #[cfg(not(windows))]
    {
        let _ = child.kill();
        let _ = child.wait();
    }
}

impl HostProcess {
    pub fn run(&self, args: &[&OsStr], cwd: &Path, timeout: Duration) -> Result<Output> {
        if self.cancelled.load(Ordering::Relaxed) {
            return Err(Error::cancelled());
        }
        let mut command = Command::new(&self.executable);
        command
            .args(args)
            .current_dir(cwd)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for key in [
            "COMPOSE_FILE",
            "COMPOSE_PROJECT_NAME",
            "COMPOSE_PROFILES",
            "COMPOSE_ENV_FILES",
            "COMPOSE_REMOVE_ORPHANS",
        ] {
            command.env_remove(key);
        }
        let mut command = process_wrap::std::CommandWrap::from(command);
        #[cfg(unix)]
        command.wrap(process_wrap::std::ProcessGroup::leader());
        #[cfg(windows)]
        command.wrap(process_wrap::std::JobObject);
        let mut child = command.spawn().map_err(|_|Error::new("process_failed", "Could not start the required host executable; check installation and executable permissions"))?;
        let stdout = child
            .stdout()
            .take()
            .ok_or_else(|| Error::new("process_failed", "Host command output is unavailable"))?;
        let stderr = child.stderr().take().ok_or_else(|| {
            Error::new("process_failed", "Host command diagnostics are unavailable")
        })?;
        let output_thread = std::thread::spawn(move || drain(stdout));
        let error_thread = std::thread::spawn(move || drain(stderr));
        let deadline = Instant::now() + timeout;
        let mut completed = None;
        let status = loop {
            if self.cancelled.load(Ordering::Relaxed) || Instant::now() >= deadline {
                terminate(child.as_mut());
                let _ = output_thread.join();
                let _ = error_thread.join();
                return Err(if self.cancelled.load(Ordering::Relaxed) {
                    Error::cancelled()
                } else {
                    Error::new(
                        "process_timeout",
                        "Local runtime command timed out; check Docker Desktop and retry",
                    )
                });
            }
            match child.try_wait() {
                Ok(Some(status)) => completed = Some(status),
                Ok(None) => std::thread::sleep(Duration::from_millis(30)),
                Err(_) => {
                    terminate(child.as_mut());
                    let _ = output_thread.join();
                    let _ = error_thread.join();
                    return Err(Error::new(
                        "process_failed",
                        "Could not read host command status",
                    ));
                }
            }
            if let Some(status) = completed {
                if output_thread.is_finished() && error_thread.is_finished() {
                    break status;
                }
                // A descendant may retain a pipe after its parent exits. Keep the deadline active.
                std::thread::sleep(Duration::from_millis(30));
            }
        };
        let (stdout, output_exceeded) = output_thread
            .join()
            .map_err(|_| Error::new("process_failed", "Host output reader failed"))?
            .map_err(Error::io)?;
        let (_, error_exceeded) = error_thread
            .join()
            .map_err(|_| Error::new("process_failed", "Host diagnostic reader failed"))?
            .map_err(Error::io)?;
        if output_exceeded || error_exceeded {
            return Err(Error::new(
                "resource_limit",
                "Host command output exceeded the local diagnostic limit",
            ));
        }
        Ok(Output {
            success: status.success(),
            stdout,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::drain;
    #[test]
    fn diagnostic_output_is_bounded() {
        let (short, exceeded) = drain(&b"ready"[..]).unwrap();
        assert_eq!(short, b"ready");
        assert!(!exceeded);
        let data = vec![0u8; super::OUTPUT_LIMIT as usize + 10];
        let (output, exceeded) = drain(&data[..]).unwrap();
        assert!(exceeded);
        assert_eq!(output.len(), super::OUTPUT_LIMIT as usize);
    }
}
