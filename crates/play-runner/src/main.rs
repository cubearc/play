use std::process::ExitCode;

fn main() -> ExitCode {
    #[cfg(target_os = "linux")]
    let result = run();
    #[cfg(not(target_os = "linux"))]
    let result: std::io::Result<()> = Err(std::io::Error::other(
        "Cube Runner requires a Linux container. Native frontend/API development remains available.",
    ));
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Cube Runner: {error}");
            ExitCode::FAILURE
        }
    }
}
#[cfg(target_os = "linux")]
fn run() -> std::io::Result<()> {
    use play_runner::{
        protocol::{Client, Server},
        runner::{self, Config},
        transport::{channel, receive, send},
    };
    if nix::unistd::geteuid().is_root() {
        return Err(std::io::Error::other("Run Cube Runner as a non-root user"));
    }
    let config = Config::from_env()?;
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [flag, value] if flag == "--console" => {
            return runner::console(
                config,
                serde_json::from_str(value)
                    .map_err(|_| std::io::Error::other("Invalid terminal location"))?,
            );
        }
        [flag] if flag == "--sandbox-check" => return runner::sandbox_check(&config),
        [flag] if flag == "--healthcheck" => {
            return tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async {
                    tokio::time::timeout(std::time::Duration::from_secs(2), async {
                        let mut transport =
                            channel(tokio::net::UnixStream::connect(&config.socket).await?);
                        send(&mut transport, &Client::Probe).await?;
                        if !matches!(
                            receive::<Server>(&mut transport).await?,
                            Some(Server::Status { .. })
                        ) {
                            return Err(std::io::Error::other("Runner is not ready"));
                        }
                        Ok(())
                    })
                    .await
                    .map_err(|_| std::io::Error::other("Runner readiness timed out"))?
                });
        }
        [] => {}
        _ => return Err(std::io::Error::other("Usage: cube-runner [--healthcheck]")),
    }
    // Check the exact sandbox in a fresh single-threaded process before accepting sessions.
    if !std::process::Command::new(std::env::current_exe()?)
        .arg("--sandbox-check")
        .status()?
        .success()
    {
        return Err(std::io::Error::other(
            "Required runner isolation could not be enforced",
        ));
    }
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?
        .block_on(runner::serve(config))
}
