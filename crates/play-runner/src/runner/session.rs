use super::Config;
use crate::{
    protocol::{Client, Location, Server, Size},
    transport::{Channel, receive, send},
};
use nix::{
    sys::signal::{Signal, killpg},
    unistd::Pid,
};
use portable_pty::{CommandBuilder, MasterPty, PtySize, native_pty_system};
use std::{
    fs,
    io::{self, Read, Write},
    path::PathBuf,
    time::Duration,
};
use tokio::sync::{OwnedSemaphorePermit, mpsc, watch};
use uuid::Uuid;

type OpenedSession = (Session, Box<dyn Read + Send>, Box<dyn Write + Send>);
struct Session {
    child: Box<dyn portable_pty::Child + Send + Sync>,
    master: std::sync::Mutex<Box<dyn MasterPty + Send>>,
    temporary: PathBuf,
}
impl Drop for Session {
    fn drop(&mut self) {
        if let Some(pid) = self
            .child
            .process_id()
            .and_then(|pid| i32::try_from(pid).ok())
        {
            let _ = killpg(Pid::from_raw(pid), Signal::SIGKILL);
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_dir_all(&self.temporary);
    }
}
fn pty_size(size: Size) -> PtySize {
    PtySize {
        rows: size.rows,
        cols: size.cols,
        pixel_width: 0,
        pixel_height: 0,
    }
}
fn failed() -> io::Error {
    io::Error::other("The isolated terminal could not start")
}
impl Session {
    fn open(
        config: &Config,
        directory: PathBuf,
        location: &Location,
        size: Size,
    ) -> io::Result<OpenedSession> {
        use std::os::unix::fs::DirBuilderExt;
        let temporary = std::env::temp_dir().join(format!("cube-terminal-{}", Uuid::new_v4()));
        fs::DirBuilder::new().mode(0o700).create(&temporary)?;
        let result = (|| {
            let pair = native_pty_system()
                .openpty(pty_size(size))
                .map_err(|_| failed())?;
            let mut command = CommandBuilder::new(std::env::current_exe()?);
            command.args([
                "--console",
                &serde_json::to_string(location).map_err(|_| failed())?,
            ]);
            command.cwd(directory);
            command.env_clear();
            for (name, path) in [
                ("CUBE_RUNNER_SOURCE", &config.source),
                ("CUBE_RUNNER_IMPORTS", &config.imports),
                ("CUBE_RUNNER_OUTPUT", &config.output),
                ("CUBE_RUNNER_CUBE", &config.cube),
                ("TMPDIR", &temporary),
                ("HOME", &temporary),
            ] {
                command.env(name, path);
            }
            command.env("TERM", "xterm-256color");
            command.env("LANG", "C.UTF-8");
            command.env("PATH", "/usr/local/bin:/usr/bin:/bin");
            let child = pair.slave.spawn_command(command).map_err(|_| failed())?;
            drop(pair.slave);
            let session = Self {
                child,
                master: std::sync::Mutex::new(pair.master),
                temporary: temporary.clone(),
            };
            let reader = session
                .master
                .lock()
                .map_err(|_| failed())?
                .try_clone_reader()
                .map_err(|_| failed())?;
            let writer = session
                .master
                .lock()
                .map_err(|_| failed())?
                .take_writer()
                .map_err(|_| failed())?;
            Ok((session, reader, writer))
        })();
        if result.is_err() {
            let _ = fs::remove_dir_all(temporary);
        }
        result
    }
}
pub(super) async fn run(
    mut channel: Channel,
    config: Config,
    directory: PathBuf,
    location: Location,
    size: Size,
    _permit: OwnedSemaphorePermit,
    mut shutdown: watch::Receiver<bool>,
) -> io::Result<()> {
    let opened =
        tokio::task::spawn_blocking(move || Session::open(&config, directory, &location, size))
            .await
            .map_err(|_| failed())??;
    let (mut session, mut reader, mut writer) = opened;
    let (output_tx, mut output) = mpsc::channel(16);
    let read_thread = std::thread::spawn(move || {
        let mut buffer = [0u8; 4096];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) | Err(_) => break,
                Ok(count) => {
                    if output_tx.blocking_send(buffer[..count].to_vec()).is_err() {
                        break;
                    }
                }
            }
        }
    });
    let (input_tx, mut input) = mpsc::channel::<String>(8);
    let write_thread = std::thread::spawn(move || {
        while let Some(data) = input.blocking_recv() {
            if writer
                .write_all(data.as_bytes())
                .and_then(|_| writer.flush())
                .is_err()
            {
                break;
            }
        }
    });
    // Cleanup is unconditional even if a transport error or task cancellation occurs.
    let result = async {
        send(&mut channel, &Server::Ready { id: Uuid::new_v4(), size }).await?;
        let mut budget = super::policy::Budget::new(std::time::Instant::now());
        let mut tick = tokio::time::interval(Duration::from_millis(100));
        loop {
            let reason = tokio::select! {
                frame = receive::<Client>(&mut channel) => match frame? {
                    Some(frame) => {
                        if let Err(reason) = budget.frame(&frame, std::time::Instant::now()) {
                            Some(reason)
                        } else {
                            match frame {
                                Client::Input { data } => {
                                    match tokio::time::timeout(Duration::from_secs(2), input_tx.send(data)).await {
                                        Ok(Ok(())) => None,
                                        _ => Some("input_backpressure"),
                                    }
                                },
                                Client::Resize { size } => {
                                    let actual = {
                                        let master = session.master.lock().map_err(|_| failed())?;
                                        master.resize(pty_size(size)).map_err(|_| failed())?;
                                        master.get_size().map_err(|_| failed())?
                                    };
                                    send(&mut channel, &Server::Resized {
                                        size: Size { cols: actual.cols, rows: actual.rows },
                                    }).await?;
                                    None
                                },
                                Client::Close => Some("closed"),
                                _ => Some("invalid_input"),
                            }
                        }
                    },
                    None => Some("closed"),
                },
                chunk = output.recv() => match chunk {
                    Some(data) => {
                        if let Err(reason) = budget.output(data.len()) {
                            Some(reason)
                        } else {
                            send(&mut channel, &Server::Output { data }).await?;
                            None
                        }
                    },
                    None => Some("exited"),
                },
                _ = tick.tick() => {
                    if let Some(reason) = budget.deadline(std::time::Instant::now()) {
                        Some(reason)
                    } else if session.child.try_wait().map_err(|_| failed())?.is_some() && output.is_closed() {
                        Some("exited")
                    } else { None }
                },
                _ = shutdown.changed() => Some("shutdown"),
            };
            if let Some(reason) = reason {
                let _ = send(&mut channel, &Server::End { reason: reason.into() }).await;
                break;
            }
        }
        Ok(())
    }.await;
    drop(input_tx);
    drop(output);
    drop(session);
    let _ = tokio::task::spawn_blocking(move || {
        let _ = read_thread.join();
        let _ = write_thread.join();
    })
    .await;
    result
}
