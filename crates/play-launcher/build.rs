use std::path::PathBuf;
fn main() {
    let root = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap())
        .join("../../frontend/launcher-dist");
    for name in ["launcher.js", "launcher.css"] {
        let file = root.join(name);
        println!("cargo:rerun-if-changed={}", file.display());
        assert!(
            file.is_file(),
            "Build the trusted launcher UI first: npm ci --prefix frontend && npm run build --prefix frontend"
        );
    }
}
