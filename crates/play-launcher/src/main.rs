use clap::Parser;
use play_launcher::{
    Error, Result, process,
    runtime::{self, Options as Args},
};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
    process::{Command, ExitCode, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

fn open_browser(url: &str, workspace: &Path, home: &Path) -> Result<()> {
    if let Some(path) = std::env::var_os("CUBE_PLAY_BROWSER") {
        let browser = process::executable("", Some(PathBuf::from(path)), Some(workspace))?;
        Command::new(browser)
            .arg(url)
            .current_dir(home)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(Error::io)?;
        return Ok(());
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("/usr/bin/open")
            .arg(url)
            .current_dir(home)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(Error::io)?;
        Ok(())
    }
    #[cfg(windows)]
    {
        webbrowser::open(url).map_err(Error::io)
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let browser = process::executable("xdg-open", None, Some(workspace))?;
        Command::new(browser)
            .arg(url)
            .current_dir(home)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(Error::io)?;
        Ok(())
    }
    #[cfg(not(any(unix, windows)))]
    {
        Err(Error::new(
            "browser_unavailable",
            "Automatic browser opening is unsupported on this platform",
        ))
    }
}

fn run(args: &Args, cancelled: Arc<AtomicBool>) -> Result<()> {
    if args.protocol != 1 {
        return Err(Error::new(
            "incompatible_launcher",
            "This launcher supports invocation protocol 1",
        ));
    }
    if args.workspace_host {
        return play_launcher::host::serve(args.clone(), cancelled);
    }
    if args.stop_chooser {
        play_launcher::host::stop(&runtime::home(args)?)?;
        println!(
            "The workspace chooser stopped. Investigation containers and data were preserved."
        );
        return Ok(());
    }
    if let Some(opened) = runtime::run(args, cancelled.clone())? {
        println!("Workspace: {}", opened.workspace.id);
        println!("Open Cube Play: {}", opened.url);
        if !args.no_browser {
            let chooser = play_launcher::host::ensure(args, &opened.url, &cancelled)?;
            println!("Open workspace chooser: {chooser}");
            if open_browser(&chooser, &opened.root, &opened.home).is_err() && !args.json_errors {
                eprintln!(
                    "The browser could not be opened automatically. Open the private workspace chooser link printed above."
                );
            }
        }
    }
    Ok(())
}

fn main() -> ExitCode {
    let json_errors = std::env::args_os().any(|argument| argument == OsStr::new("--json-errors"));
    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(error) => {
            if json_errors && error.use_stderr() {
                print_error(&Error::new(
                    "invalid_usage",
                    "Invalid Cube Play arguments; use --help",
                ));
            } else {
                let _ = error.print();
            }
            return ExitCode::from(error.exit_code() as u8);
        }
    };
    if args.launcher_protocol {
        println!("cube-play-launcher 1");
        return ExitCode::SUCCESS;
    }
    let cancelled = Arc::new(AtomicBool::new(false));
    let flag = cancelled.clone();
    if ctrlc::set_handler(move || flag.store(true, Ordering::Relaxed)).is_err() {
        let error = Error::new(
            "signal_unavailable",
            "Could not install interruption handling; local runtime startup was not attempted",
        );
        if args.json_errors {
            print_error(&error);
        } else {
            eprintln!("error: {error}");
        }
        return ExitCode::FAILURE;
    }
    match run(&args, cancelled) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            if args.json_errors {
                print_error(&error);
            } else {
                eprintln!("error: {error}");
            }
            ExitCode::from(if error.code == "cancelled" { 130 } else { 1 })
        }
    }
}

fn print_error(error: &Error) {
    eprintln!(
        "{}",
        serde_json::json!({"schema_version":1,"error":{"code":error.code,"message":error.message,"operation":null,"parameter":null,"offset":null}})
    );
}
