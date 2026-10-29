//! Test-only supervisor control: hold a canonical node-start event under the real policy.
//! No operation result is fabricated, and the production worker never uses this helper.
#[cfg(unix)]
fn main() -> play_engine::Result<()> {
    use play_engine::{native, protocol, worker};
    use serde_json::json;
    use std::io::{self, Read};

    native::restrict()?;
    let parent = rustix::process::getppid();
    let mut bytes = Vec::new();
    io::stdin()
        .take(protocol::contract().frame_bytes as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "invalid_frame")?;
    if bytes.len() > protocol::contract().frame_bytes {
        return Err("invalid_frame");
    }
    let value =
        cube_core::pipeline::json::parse(std::str::from_utf8(&bytes).map_err(|_| "invalid_frame")?)
            .map_err(|_| "invalid_frame")?;
    let worker::Task::Pipeline { invocation } =
        serde_json::from_value(value).map_err(|_| "invalid_command")?
    else {
        return Err("invalid_command");
    };
    let mut output = io::stdout().lock();
    let outcome = worker::execute_pipeline(invocation, |event| {
        protocol::write_frame(&mut output, &event)?;
        if event["type"] == "node_started" {
            // The engine's unchanged wall deadline bounds the hold. Owner death also releases it.
            while rustix::process::getppid() == parent {
                std::thread::sleep(std::time::Duration::from_millis(2));
            }
            return Err("supervisor_stopped");
        }
        Ok(())
    });
    let result = match outcome {
        Ok(value) => json!({"status":"completed","output":value}),
        Err(code) => json!({"status":"failed","code":code}),
    };
    protocol::write_frame(&mut output, &result)
}
#[cfg(not(unix))]
fn main() {
    eprintln!("Native supervisor policy unavailable");
    std::process::exit(1);
}
