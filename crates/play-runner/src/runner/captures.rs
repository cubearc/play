//! Session-scoped staging, never an arbitrary output or host-path reader.
use crate::protocol::{CAPTURE_LIMIT, valid_capture};
use cap_std::fs::{Dir, OpenOptions};
use std::{
    io::{self, Read},
    path::Path,
};
fn denied() -> io::Error {
    io::Error::other("Capture is missing, incomplete or exceeds its private staging bounds")
}
fn root(temporary: &Path) -> io::Result<Dir> {
    use std::os::unix::fs::OpenOptionsExt;
    // Check the final component atomically while opening it: the CLI can rename
    // entries in its own temporary directory between a metadata check and open.
    let directory = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(nix::libc::O_DIRECTORY | nix::libc::O_NOFOLLOW)
        .open(temporary.join("captures"))?;
    Ok(Dir::from_std_file(directory))
}
fn check(root: &Dir, name: &str) -> io::Result<()> {
    use cap_std::fs::MetadataExt;
    let m = root.symlink_metadata(name)?;
    if !m.is_file()
        || m.file_type().is_symlink()
        || m.nlink() != 1
        || m.len() > CAPTURE_LIMIT as u64
    {
        return Err(denied());
    }
    Ok(())
}

pub(super) fn list(temporary: &Path) -> io::Result<Vec<String>> {
    let root = root(temporary)?;
    let mut ids = Vec::new();
    for entry in root.entries()?.take(17) {
        let entry = entry?;
        if ids.len() == 16 {
            return Err(denied());
        }
        let name = entry.file_name();
        let name = name.to_str().ok_or_else(denied)?;
        let id = name
            .strip_suffix(".json")
            .filter(|id| valid_capture(id))
            .ok_or_else(denied)?;
        check(&root, name)?;
        ids.push(id.to_owned());
    }
    ids.sort();
    Ok(ids)
}
pub(super) fn read(temporary: &Path, id: &str) -> io::Result<Vec<u8>> {
    use cap_std::fs::OpenOptionsExt;
    if !valid_capture(id) {
        return Err(denied());
    }
    let root = root(temporary)?;
    let name = format!("{id}.json");
    check(&root, &name)?;
    let mut options = OpenOptions::new();
    options
        .read(true)
        .custom_flags(nix::libc::O_NOFOLLOW | nix::libc::O_NONBLOCK);
    let file = root.open_with(&name, &options)?;
    use cap_std::fs::MetadataExt;
    let m = file.metadata()?;
    if !m.is_file() || m.nlink() != 1 || m.len() > CAPTURE_LIMIT as u64 {
        return Err(denied());
    }
    let mut bytes = Vec::new();
    file.take(CAPTURE_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > CAPTURE_LIMIT {
        return Err(denied());
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn staging_rejects_paths_links_special_files_and_excessive_counts_or_bytes() {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("captures");
        std::fs::create_dir(&root).unwrap();
        std::fs::write(root.join("one.json"), b"exact bytes").unwrap();
        assert_eq!(list(temporary.path()).unwrap(), ["one"]);
        assert_eq!(read(temporary.path(), "one").unwrap(), b"exact bytes");
        for id in ["../one", "/etc/passwd", "one.json", ""] {
            assert!(read(temporary.path(), id).is_err());
        }
        std::os::unix::fs::symlink(root.join("one.json"), root.join("link.json")).unwrap();
        assert!(read(temporary.path(), "link").is_err());
        assert!(list(temporary.path()).is_err());
        std::fs::remove_file(root.join("link.json")).unwrap();
        std::fs::hard_link(root.join("one.json"), root.join("hard.json")).unwrap();
        assert!(read(temporary.path(), "hard").is_err());
        std::fs::remove_file(root.join("hard.json")).unwrap();
        std::fs::write(root.join("large.json"), vec![0; CAPTURE_LIMIT + 1]).unwrap();
        assert!(read(temporary.path(), "large").is_err());
        std::fs::remove_file(root.join("large.json")).unwrap();
        for i in 0..16 {
            std::fs::write(root.join(format!("id-{i}.json")), b"small").unwrap();
        }
        assert!(list(temporary.path()).is_err());
        let elsewhere = tempfile::tempdir().unwrap();
        std::fs::rename(&root, temporary.path().join("old-captures")).unwrap();
        std::os::unix::fs::symlink(elsewhere.path(), &root).unwrap();
        assert!(list(temporary.path()).is_err());
        assert!(read(temporary.path(), "one").is_err());
    }
}
