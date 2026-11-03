//! Actual-host adversarial fixture; never packaged as a production helper.
#[cfg(windows)]
fn main() {
    use std::{
        env, fs,
        net::{SocketAddr, TcpStream},
        process::Command,
        time::Duration,
    };
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args == ["owner"] {
        use std::io::{BufRead, BufReader, Read, Write};
        let contract = play_engine::protocol::contract();
        let mut child = play_windows_boundary::Child::spawn(
            &env::current_exe().unwrap(),
            &["idle"],
            play_windows_boundary::Limits {
                private_memory_bytes: contract.vm_growth_bytes as usize,
                cpu_seconds: contract.cpu_seconds,
            },
        )
        .unwrap();
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut line)
            .unwrap();
        print!("{line}");
        std::io::stdout().flush().unwrap();
        let _ = std::io::stdin().read(&mut [0]);
        return;
    }
    play_engine::native::restrict().expect("AppContainer and bounded job are required");
    match args.as_slice() {
        [mode, path, port] if mode == "boundaries" => {
            let binary = env::current_exe().unwrap();
            let source: SocketAddr = format!("127.0.0.1:{port}").parse().unwrap();
            println!(
                "{}",
                serde_json::json!({"image_readable":fs::read(&binary).is_ok(),"private_read_denied":fs::read(path).is_err_and(|e| e.kind()==std::io::ErrorKind::PermissionDenied),"private_write_denied":fs::write(path,b"forbidden").is_err_and(|e| e.kind()==std::io::ErrorKind::PermissionDenied),"network_denied":TcpStream::connect_timeout(&source,Duration::from_millis(200)).is_err(),"child_denied":Command::new(&binary).arg("child").status().is_err(),"image_directory_write_denied":fs::write(binary.parent().unwrap().join("forbidden.txt"),b"forbidden").is_err_and(|e| e.kind()==std::io::ErrorKind::PermissionDenied)})
            );
        }
        [mode] if mode == "idle" => {
            use std::io::Write;
            println!("{}", std::process::id());
            std::io::stdout().flush().unwrap();
            loop {
                std::thread::sleep(Duration::from_secs(1));
            }
        }
        [mode] if mode == "cpu" => loop {
            std::hint::black_box(42);
        },
        [mode] if mode == "memory" => {
            let mut bytes = Vec::<u8>::new();
            println!("{}", bytes.try_reserve_exact(256 * 1024 * 1024).is_err());
        }
        _ => panic!("explicit controlled inputs required"),
    }
}
#[cfg(not(windows))]
fn main() {
    eprintln!("Windows-only acceptance fixture");
    std::process::exit(1);
}
