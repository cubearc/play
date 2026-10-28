use std::{env, path::PathBuf, process::ExitCode};

fn main() -> ExitCode {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let result = match args.as_slice() {
        [mode] if mode == "--worker" => play_engine::worker::run(),
        [mode, flag, worker] if mode == "--ipc" && flag == "--worker" => {
            play_engine::session::run(Some(PathBuf::from(worker)))
        }
        [mode] if mode == "--ipc" => play_engine::session::run(None),
        _ => Err("invalid_startup"),
    };
    if let Err(code) = result {
        eprintln!("{code}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
