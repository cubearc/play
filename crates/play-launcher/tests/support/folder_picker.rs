//! Controlled system-dialog boundary. No runtime binary target includes this file.
use std::{env, fs, io::Write, path::PathBuf};
fn main() {
    let root = PathBuf::from(env::var_os("CUBE_PLAY_PICKER_FIXTURE").unwrap());
    let mode = env::args().nth(1).unwrap();
    assert!(matches!(mode.as_str(), "open" | "new"));
    writeln!(
        fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(root.join("events"))
            .unwrap(),
        "{mode}"
    )
    .unwrap();
    while root.join("hold").exists() {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let selected = fs::read_to_string(root.join("selection")).unwrap();
    if !selected.is_empty() {
        println!("{selected}");
    }
}
