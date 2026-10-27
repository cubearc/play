use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{Error, Result};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Workspace {
    pub id: Uuid,
    pub root: PathBuf,
    pub name: String,
}

pub fn contains_path(root: &Path, path: &Path) -> bool {
    path.ancestors().any(|ancestor| {
        ancestor == root || same_file::is_same_file(root, ancestor).unwrap_or(false)
    })
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Registry {
    version: u32,
    workspaces: Vec<Workspace>,
}

pub struct Lock(File);
impl Drop for Lock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

pub fn private_directory(path: &Path) -> Result<()> {
    let mut builder = fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path).map_err(Error::io)?;
    let metadata = fs::symlink_metadata(path).map_err(Error::io)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(Error::new(
            "unsafe_state",
            "Launcher state must be a private directory, not a symlink",
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(Error::new(
                "unsafe_state",
                "Launcher state directory must be private to its owner; choose a private directory rather than shared storage",
            ));
        }
    }
    Ok(())
}

pub fn lock(path: &Path, cancelled: &Arc<AtomicBool>) -> Result<Lock> {
    if fs::symlink_metadata(path)
        .is_ok_and(|metadata| !metadata.is_file() || metadata.file_type().is_symlink())
    {
        return Err(Error::new(
            "unsafe_state",
            "Launcher lock must be a regular private file",
        ));
    }
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let file = options.open(path).map_err(Error::io)?;
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        match file.try_lock() {
            Ok(()) => return Ok(Lock(file)),
            Err(std::fs::TryLockError::WouldBlock) => {}
            Err(std::fs::TryLockError::Error(error)) => return Err(Error::io(error)),
        }
        if cancelled.load(Ordering::Relaxed) {
            return Err(Error::cancelled());
        }
        if Instant::now() >= deadline {
            return Err(Error::new(
                "workspace_busy",
                "Another Cube Play startup is in progress; wait for it to finish and retry",
            ));
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

pub fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let metadata = fs::symlink_metadata(path).map_err(Error::io)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 1024 * 1024 {
        return Err(Error::new(
            "unsafe_state",
            "Launcher state is not a bounded regular file",
        ));
    }
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(Error::io)?
        .take(1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(Error::io)?;
    if bytes.len() > 1024 * 1024 {
        return Err(Error::new(
            "resource_limit",
            "Launcher state exceeds its size limit",
        ));
    }
    serde_json::from_slice(&bytes).map_err(|_|Error::new("invalid_state", "Launcher state is invalid or incompatible; preserve it for recovery rather than deleting investigation data"))
}

pub fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let bytes = serde_json::to_vec(value)
        .map_err(|_| Error::new("invalid_state", "Could not serialize launcher state"))?;
    if bytes.len() > 1024 * 1024 {
        return Err(Error::new(
            "resource_limit",
            "Launcher state exceeds its size limit; existing state was preserved",
        ));
    }
    let parent = path.parent().ok_or_else(|| {
        Error::new(
            "invalid_state",
            "Launcher state requires a parent directory",
        )
    })?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(Error::io)?;
    file.write_all(&bytes).map_err(Error::io)?;
    file.write_all(b"\n").map_err(Error::io)?;
    file.as_file().sync_all().map_err(Error::io)?;
    file.persist(path).map_err(|error| Error::io(error.error))?;
    Ok(())
}

pub fn resolve(
    path: Option<&Path>,
    cwd: &Path,
) -> Result<(PathBuf, Option<(String, &'static str)>)> {
    let input = path
        .map(|path| {
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                cwd.join(path)
            }
        })
        .unwrap_or_else(|| cwd.to_path_buf());
    let canonical = input.canonicalize().map_err(|_|Error::new("invalid_path", "Selected path is missing or inaccessible; choose an existing local directory or regular file"))?;
    let metadata = canonical.metadata().map_err(Error::io)?;
    let (root, selection) = if metadata.is_dir() {
        (canonical, None)
    } else if metadata.is_file() {
        let name = canonical
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| name.len() <= 4096)
            .ok_or_else(|| {
                Error::new(
                    "unsupported_path",
                    "Docker workspace paths require bounded Unicode filenames",
                )
            })?
            .to_string();
        let intent = if canonical
            .extension()
            .is_some_and(|extension| extension.eq_ignore_ascii_case("cube"))
        {
            "challenge"
        } else {
            "file"
        };
        (
            canonical
                .parent()
                .ok_or_else(|| {
                    Error::new("invalid_path", "Selected file has no workspace directory")
                })?
                .to_path_buf(),
            Some((name, intent)),
        )
    } else {
        return Err(Error::new(
            "invalid_path",
            "Devices, sockets and special files cannot be opened as workspaces",
        ));
    };
    if root.parent().is_none() {
        return Err(Error::new(
            "unsafe_root",
            "Choose an investigation directory rather than an entire filesystem root",
        ));
    }
    docker_path(&root)?;
    fs::read_dir(&root).map_err(|_| {
        Error::new(
            "invalid_path",
            "Selected source directory is not readable by the current user",
        )
    })?;
    Ok((root, selection))
}

pub fn docker_path(path: &Path) -> Result<String> {
    let text = path
        .to_str()
        .filter(|text| text.len() <= 16384)
        .ok_or_else(|| {
            Error::new(
                "unsupported_path",
                "Docker paths must be bounded Unicode paths",
            )
        })?;
    #[cfg(windows)]
    {
        if text.starts_with(r"\\?\UNC\") || text.starts_with(r"\\") && !text.starts_with(r"\\?\") {
            return Err(Error::new(
                "unsupported_path",
                "Use a local Windows drive with Docker Desktop file sharing; UNC workspace mounts are unsupported",
            ));
        }
        Ok(text.strip_prefix(r"\\?\").unwrap_or(text).to_string())
    }
    #[cfg(not(windows))]
    {
        Ok(text.to_string())
    }
}

fn registry(home: &Path) -> Result<Registry> {
    let path = home.join("workspaces.json");
    let registry = if path.exists() {
        read_json::<Registry>(&path)?
    } else {
        Registry {
            version: 1,
            workspaces: Vec::new(),
        }
    };
    let mut ids = std::collections::HashSet::new();
    let mut roots = std::collections::HashSet::new();
    if registry.version != 1
        || registry.workspaces.len() > 256
        || registry.workspaces.iter().any(|workspace| {
            workspace.id.get_version_num() != 4
                || !workspace.root.is_absolute()
                || workspace.root.parent().is_none()
                || workspace.name.is_empty()
                || workspace.name.contains('\0')
                || workspace.name.len() > 4096
                || !ids.insert(workspace.id)
                || !roots.insert(&workspace.root)
        })
    {
        return Err(Error::new(
            "invalid_state",
            "Launcher workspace registry is incompatible or inconsistent; preserve it for recovery",
        ));
    }
    Ok(registry)
}

pub fn recent_workspaces(home: &Path, cancelled: &Arc<AtomicBool>) -> Result<Vec<Workspace>> {
    let _lock = lock(&home.join("registry.lock"), cancelled)?;
    Ok(registry(home)?.workspaces.into_iter().rev().collect())
}

pub fn workspace(
    home: &Path,
    root: &Path,
    create: bool,
    cancelled: &Arc<AtomicBool>,
) -> Result<Workspace> {
    let _lock = lock(&home.join("registry.lock"), cancelled)?;
    let path = home.join("workspaces.json");
    let mut registry = registry(home)?;
    if let Some(index) = registry.workspaces.iter().position(|workspace| {
        workspace.root == root
            || (workspace
                .root
                .canonicalize()
                .is_ok_and(|current| current == workspace.root)
                && same_file::is_same_file(&workspace.root, root).unwrap_or(false))
    }) {
        let workspace = registry.workspaces[index].clone();
        if create && index + 1 != registry.workspaces.len() {
            registry.workspaces.remove(index);
            registry.workspaces.push(workspace.clone());
            write_json(&path, &registry)?;
        }
        return Ok(workspace);
    }
    if !create {
        return Err(Error::new(
            "workspace_missing",
            "No Cube Play runtime is registered for this directory",
        ));
    }
    if registry.workspaces.len() == 256 {
        return Err(Error::new(
            "resource_limit",
            "The local launcher registry has reached its workspace limit",
        ));
    }
    let workspace = Workspace {
        id: Uuid::new_v4(),
        root: root.to_path_buf(),
        name: root
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Workspace")
            .to_string(),
    };
    registry.workspaces.push(workspace.clone());
    write_json(&path, &registry)?;
    Ok(workspace)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_directories_files_and_registry_round_trip() {
        let directory = tempfile::tempdir().unwrap();
        let cwd = directory.path().canonicalize().unwrap();
        let (root, selected) = resolve(None, &cwd).unwrap();
        assert_eq!(root, cwd);
        assert!(selected.is_none());
        for file in ["sample 空.mp4", "rabbit.cube", "note $test.txt"] {
            fs::write(cwd.join(file), b"original").unwrap();
            let (root, selected) = resolve(Some(Path::new(file)), &cwd).unwrap();
            assert_eq!(root, cwd);
            assert_eq!(
                selected.unwrap(),
                (
                    file.to_string(),
                    if file.ends_with(".cube") {
                        "challenge"
                    } else {
                        "file"
                    }
                )
            );
        }
        assert!(resolve(Some(Path::new("missing")), &cwd).is_err());
        let home = cwd.join("private");
        private_directory(&home).unwrap();
        let cancelled = Arc::new(AtomicBool::new(false));
        let first = workspace(&home, &cwd, true, &cancelled).unwrap();
        let again = workspace(&home, &cwd, false, &cancelled).unwrap();
        assert_eq!(first.id, again.id);
        write_json(
            &home.join("workspaces.json"),
            &serde_json::json!({"version":99,"workspaces":[]}),
        )
        .unwrap();
        assert!(workspace(&home, &cwd, true, &cancelled).is_err());
    }
    #[cfg(unix)]
    #[test]
    fn refuses_special_paths_and_state_symlinks() {
        use std::os::unix::{fs::symlink, net::UnixListener};
        let directory = tempfile::tempdir().unwrap();
        let cwd = directory.path().canonicalize().unwrap();
        let _socket = UnixListener::bind(cwd.join("socket")).unwrap();
        assert!(resolve(Some(Path::new("socket")), &cwd).is_err());
        assert!(resolve(Some(Path::new("/")), &cwd).is_err());
        fs::write(cwd.join("data"), b"{}").unwrap();
        symlink(cwd.join("data"), cwd.join("alias")).unwrap();
        assert!(read_json::<serde_json::Value>(&cwd.join("alias")).is_err());
        assert!(lock(&cwd.join("alias"), &Arc::new(AtomicBool::new(false))).is_err());
    }
}
