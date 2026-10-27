use super::failed;
use crate::workspace::Result;
use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
};

/// Configuration is trusted, but storage must never alias source or imported data.
pub(super) fn prepare(path: &Path, source: &Path, imports: &Path) -> Result<(PathBuf, File)> {
    let source = source.canonicalize().map_err(|_| failed())?;
    let imports = imports.canonicalize().map_err(|_| failed())?;
    let parent = path
        .parent()
        .ok_or_else(failed)?
        .canonicalize()
        .map_err(|_| failed())?;
    let expected = parent.join(path.file_name().ok_or_else(failed)?);
    if !path.is_absolute()
        || [&source, &imports]
            .iter()
            .any(|root| expected.starts_with(root) || root.starts_with(&expected))
    {
        return Err(failed());
    }
    let builder = fs::DirBuilder::new();
    #[cfg(unix)]
    let mut builder = builder;
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.is_dir() || metadata.file_type().is_symlink() => {
            return Err(failed());
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            builder.create(path).map_err(|_| failed())?
        }
        Err(_) => return Err(failed()),
    }
    let canonical = path.canonicalize().map_err(|_| failed())?;
    if canonical != expected {
        return Err(failed());
    }
    private(&canonical, true)?;
    for name in [
        "investigation.sqlite3",
        "investigation.sqlite3-wal",
        "investigation.sqlite3-shm",
        "investigation.sqlite3-journal",
        "owner.lock",
    ] {
        let file = canonical.join(name);
        if fs::symlink_metadata(&file).is_ok() {
            private(&file, false)?;
        }
    }
    let lock = open(&canonical.join("owner.lock"))?;
    lock.try_lock().map_err(|_|super::FileError(axum::http::StatusCode::CONFLICT,"storage_locked","Another Cube Play process owns this investigation. Stop that instance before reopening the same storage."))?;
    let database = canonical.join("investigation.sqlite3");
    let file = open(&database)?;
    if file.metadata().map_err(|_| failed())?.len() > 128 * 1024 * 1024 {
        return Err(failed());
    }
    drop(file);
    Ok((database, lock))
}
fn open(path: &Path) -> Result<File> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options
            .mode(0o600)
            .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32);
    }
    let file = options.open(path).map_err(|_| failed())?;
    private(path, false)?;
    Ok(file)
}
fn private(path: &Path, directory: bool) -> Result<()> {
    let metadata = fs::symlink_metadata(path).map_err(|_| failed())?;
    if metadata.file_type().is_symlink()
        || (directory && !metadata.is_dir())
        || (!directory && !metadata.is_file())
    {
        return Err(failed());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        if metadata.permissions().mode() & 0o077 != 0 || (!directory && metadata.nlink() != 1) {
            return Err(failed());
        }
    }
    Ok(())
}
