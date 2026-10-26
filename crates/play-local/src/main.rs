use std::{
    io::{Read, Write},
    net::TcpStream,
    process::ExitCode,
    time::Duration,
};

use play_local::config::Config;

#[tokio::main]
async fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "play_local=info".into()),
        )
        .with_target(false)
        .init();
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(%error, "Cube Play could not start");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env().map_err(std::io::Error::other)?;
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args == ["--healthcheck"] {
        return healthcheck(&config);
    }
    if !args.is_empty() {
        return Err("Usage: play-local [--healthcheck]".into());
    }
    let listener = tokio::net::TcpListener::bind(config.bind).await?;
    let secret = play_local::session_secret();
    println!("Open Cube Play: {}/#session={secret}", config.origin);
    tracing::info!("Local application is ready");
    axum::serve(listener, play_local::app(config, secret))
        .with_graceful_shutdown(shutdown())
        .await?;
    Ok(())
}

fn healthcheck(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let address = std::net::SocketAddr::from(([127, 0, 0, 1], config.bind.port()));
    let timeout = Duration::from_secs(2);
    let mut stream = TcpStream::connect_timeout(&address, timeout)?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    write!(
        stream,
        "GET /healthz HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        config.authority()
    )?;
    let mut response = String::new();
    stream.take(1024).read_to_string(&mut response)?;
    if !response.starts_with("HTTP/1.1 200 ") {
        return Err("Local health endpoint is not ready".into());
    }
    Ok(())
}

async fn shutdown() {
    #[cfg(unix)]
    {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut terminate) => {
                tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = terminate.recv() => {} }
            }
            Err(error) => {
                tracing::error!(%error, "Could not register termination handler");
                let _ = tokio::signal::ctrl_c().await;
            }
        }
    }
    #[cfg(not(unix))]
    let _ = tokio::signal::ctrl_c().await;
}
