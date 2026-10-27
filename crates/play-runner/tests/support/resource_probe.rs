//! Test-only executable: exercises inherited kernel limits without a shell.
use std::io::Write;

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("exit") => {}
        Some("cpu") => loop {
            std::hint::spin_loop();
        },
        Some("file") => {
            let mut file = std::fs::File::create(&args[1]).unwrap();
            file.write_all(&vec![0u8; 2 * 1024 * 1024]).unwrap();
            panic!("File size limit did not stop the write");
        }
        _ => panic!("Unknown resource test mode"),
    }
}
