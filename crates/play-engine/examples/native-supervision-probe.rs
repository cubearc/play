//! Test-only supervisor control: hold a canonical node-start event under the real policy.
//! No operation result is fabricated, and the production worker never uses this helper.
#[cfg(unix)]
fn main() -> play_engine::Result<()> {
    use play_engine::{native, protocol, worker};
    use serde_json::json;
    use std::io;

    native::restrict()?;
    let worker::Task::Pipeline { invocation } = worker::receive_task()? else {
        return Err("invalid_command");
    };
    let selected = invocation.definition.schedule()?;
    let hold_after = invocation
        .definition
        .nodes
        .iter()
        .filter(|node| selected.contains(&node.id) && node.dependencies().next().is_none())
        .count()
        .min(protocol::contract().parallel_nodes);
    let mut started = 0;
    let mut output = io::stdout().lock();
    let outcome = worker::execute_pipeline(invocation, |event| {
        protocol::write_frame(&mut output, &event)?;
        if event["type"] == "node_started" {
            started += 1;
        }
        if started == hold_after {
            // Use the production lifetime guard, not a fixture-only PID check.
            // The engine's unchanged wall deadline also bounds this hold.
            loop {
                std::thread::park();
            }
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
