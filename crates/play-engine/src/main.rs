use std::{env, path::PathBuf, process::ExitCode};

fn main() -> ExitCode {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let result = match args.as_slice() {
        [mode, flag, worker, root_flag, root, terminal_flag, terminal]
            if mode == "--ipc"
                && flag == "--worker"
                && root_flag == "--data-root"
                && terminal_flag == "--terminal" =>
        {
            play_engine::session::run(
                Some(PathBuf::from(worker)),
                Some(PathBuf::from(root)),
                Some(PathBuf::from(terminal)),
            )
        }
        [mode] if mode == "--terminal-worker" => play_engine::terminal::worker(),
        [mode] if mode == "--worker" => play_engine::worker::run(),
        [mode, flag, worker] if mode == "--ipc" && flag == "--worker" => {
            play_engine::session::run(Some(PathBuf::from(worker)), None, None)
        }
        [mode, flag, root] if mode == "--ipc" && flag == "--data-root" => {
            play_engine::session::run(None, Some(PathBuf::from(root)), None)
        }
        [mode, flag, worker, root_flag, root]
            if mode == "--ipc" && flag == "--worker" && root_flag == "--data-root" =>
        {
            play_engine::session::run(Some(PathBuf::from(worker)), Some(PathBuf::from(root)), None)
        }
        [mode] if mode == "--ipc" => play_engine::session::run(None, None, None),
        _ => Err("invalid_startup"),
    };
    if let Err(code) = result {
        eprintln!("{code}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
