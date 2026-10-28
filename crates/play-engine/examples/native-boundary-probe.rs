//! Development-only adversarial probe; never exposed through desktop IPC.
use std::{env, fs, io::ErrorKind, net::TcpStream, process::Command};

fn main() {
    play_engine::native::restrict().expect("native policy required");
    let args = env::args().skip(1).collect::<Vec<_>>();
    match args.as_slice() {
        [mode] if mode == "cpu" => loop {
            std::hint::black_box(42);
        },
        [mode] if mode == "memory" => {
            let mut bytes = Vec::<u8>::new();
            println!("{}", bytes.try_reserve_exact(256 * 1024 * 1024).is_err());
        }
        [path, port] => {
            let read_denied =
                fs::read(path).is_err_and(|error| error.kind() == ErrorKind::PermissionDenied);
            let write_denied = fs::write(path, b"forbidden")
                .is_err_and(|error| error.kind() == ErrorKind::PermissionDenied);
            let network_denied = TcpStream::connect(format!("127.0.0.1:{port}"))
                .is_err_and(|error| error.kind() == ErrorKind::PermissionDenied);
            let spawn_denied = Command::new("/usr/bin/true").status().is_err_and(|error| {
                matches!(
                    error.kind(),
                    ErrorKind::WouldBlock | ErrorKind::PermissionDenied
                )
            });
            println!(
                "{}",
                serde_json::json!({"read_denied":read_denied,"write_denied":write_denied,
                "network_denied":network_denied,"spawn_denied":spawn_denied})
            );
        }
        _ => panic!("explicit local test inputs required"),
    }
}
