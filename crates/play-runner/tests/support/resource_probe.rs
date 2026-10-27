//! Test-only executable: exercises inherited kernel limits without a shell.
use std::{
    io::Write,
    process::{Child, Command, Stdio},
    time::Duration,
};

struct Children(Vec<Child>);
impl Drop for Children {
    fn drop(&mut self) {
        for child in &mut self.0 {
            let _ = child.kill();
        }
        for child in &mut self.0 {
            let _ = child.wait();
        }
    }
}
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("wait") => std::thread::sleep(Duration::from_secs(20)),
        Some("cpu") => loop {
            std::hint::spin_loop();
        },
        Some("file") => {
            let mut file = std::fs::File::create(&args[1]).unwrap();
            file.write_all(&vec![0u8; 2 * 1024 * 1024]).unwrap();
            panic!("File size limit did not stop the write");
        }
        Some("processes") => {
            let mut children = Children(Vec::new());
            for _ in 0..256 {
                match Command::new(std::env::current_exe().unwrap())
                    .arg("wait")
                    .stdin(Stdio::null())
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .spawn()
                {
                    Ok(child) => children.0.push(child),
                    Err(error) => {
                        assert_eq!(error.kind(), std::io::ErrorKind::WouldBlock);
                        return;
                    }
                }
            }
            panic!("Process limit did not reject additional children");
        }
        _ => panic!("Unknown resource test mode"),
    }
}
