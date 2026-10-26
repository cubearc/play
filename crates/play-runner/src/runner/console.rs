use super::{Config, sandbox};
use crate::protocol::{COMMAND_SECONDS, Location, Store};
use std::{
    io::{self, BufRead, Read, Write},
    process::{Command, Stdio},
    time::{Duration, Instant},
};

fn arguments(line: &str) -> io::Result<Vec<String>> {
    let args =
        shell_words::split(line).map_err(|_| io::Error::other("Unclosed command quotation"))?;
    if args.len() > 128 {
        return Err(io::Error::other("Use at most 128 command arguments"));
    }
    if args.first().is_some_and(|arg| arg == "cube") {
        let command = args
            .iter()
            .skip(1)
            .find(|arg| !matches!(arg.as_str(), "--json-errors" | "--"));
        if command.is_some_and(|command| command == "play") {
            return Err(io::Error::other(
                "Run cube play from the trusted host launcher",
            ));
        }
    }
    Ok(args)
}
pub(super) fn run(config: Config, mut location: Location) -> io::Result<()> {
    let temporary = std::env::var_os("TMPDIR")
        .map(std::path::PathBuf::from)
        .ok_or_else(|| io::Error::other("Private session storage is missing"))?;
    sandbox::apply(&config, &temporary)?;
    ctrlc::set_handler(|| {})
        .map_err(|_| io::Error::other("Could not configure terminal interrupt handling"))?;
    println!(
        "Cube terminal · local and private\r\nUse cube --help, cd <source|imports|output> [path], pwd, clear or exit.\r\nShell scripts, pipes and expansions are not executed. Commands and output are not saved."
    );
    let stdin = io::stdin();
    let mut input = stdin.lock();
    loop {
        print!("{}:/{}> ", location.store.name(), location.path);
        io::stdout().flush()?;
        let mut line = String::new();
        let count = input.by_ref().take(8193).read_line(&mut line)?;
        if count == 0 {
            return Ok(());
        }
        if count > 8192 {
            return Err(io::Error::other(
                "Command exceeded its 8 KiB limit; session closed",
            ));
        }
        let args = match arguments(&line) {
            Ok(value) => value,
            Err(error) => {
                println!("{error}");
                continue;
            }
        };
        let Some(command) = args.first() else {
            continue;
        };
        match command.as_str() {
            "exit" if args.len() == 1 => return Ok(()),
            "pwd" if args.len() == 1 => println!("{}:/{}", location.store.name(), location.path),
            "clear" if args.len() == 1 => print!("\x1b[2J\x1b[H"),
            "cd" if (2..=3).contains(&args.len()) => {
                let store = match args[1].as_str() {
                    "source" => Store::Source,
                    "imports" => Store::Imports,
                    "output" => Store::Output,
                    _ => {
                        println!("Choose source, imports or output.");
                        continue;
                    }
                };
                let next = Location {
                    store,
                    path: args.get(2).cloned().unwrap_or_default(),
                };
                match config.directory(&next) {
                    Ok(path) => {
                        std::env::set_current_dir(path)?;
                        location = next;
                    }
                    Err(_) => println!("Working directory is unavailable or unauthorized."),
                }
            }
            "cube" => {
                let mut child = match Command::new(&config.cube)
                    .args(&args[1..])
                    .stdin(Stdio::inherit())
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .spawn()
                {
                    Ok(child) => child,
                    Err(_) => {
                        println!("The versioned Cube CLI could not start.");
                        continue;
                    }
                };
                let deadline = Instant::now() + Duration::from_secs(COMMAND_SECONDS);
                loop {
                    if let Some(status) = child.try_wait()? {
                        if !status.success() {
                            println!("Cube command ended with {status}.");
                        }
                        break;
                    }
                    if Instant::now() >= deadline {
                        let _ = child.kill();
                        let _ = child.wait();
                        println!("Cube command exceeded its 30-second execution limit.");
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(20));
                }
            }
            _ => println!("Use cube commands or the documented cd, pwd, clear and exit controls."),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn command_words_are_literal_and_host_delegation_is_rejected() {
        assert_eq!(
            arguments("cube encode base64 'hello world'").unwrap(),
            ["cube", "encode", "base64", "hello world"]
        );
        assert_eq!(
            arguments("cube encode base64 '$(id); private'").unwrap()[3],
            "$(id); private"
        );
        for command in ["cube play", "cube --json-errors play .", "cube -- play"] {
            assert!(arguments(command).is_err());
        }
        assert!(arguments("cube 'unclosed").is_err());
        assert!(arguments(&"x ".repeat(129)).is_err());
    }
}
