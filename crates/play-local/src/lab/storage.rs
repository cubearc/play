//! Small, private filesystem result bundles; saved history remains a separate capability.
use super::{execute::json_bytes, types::*};
use crate::config::Config;
use cap_std::fs::{Dir, OpenOptions};
use std::{
    io::{Read, Write},
    path::Path,
    sync::Arc,
};
use uuid::Uuid;

const MAX_RESULTS: usize = 64;
const MAX_TOTAL: u64 = 64 * 1024 * 1024;
const MAX_RECORD: usize = 128 * 1024;
pub struct Storage {
    root: Arc<Dir>,
    workspace: Uuid,
}

fn private(path: &Path) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path).map_err(|_| Error::storage())?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err(Error::storage());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(Error::storage());
        }
    }
    Ok(())
}
fn create(path: &Path) -> Result<()> {
    let mut builder = std::fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    match builder.create(path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(_) => return Err(Error::storage()),
    };
    private(path)
}
impl Storage {
    pub fn open(config: &Config) -> Result<Self> {
        let imports = config
            .data_dir
            .as_ref()
            .ok_or_else(Error::storage)?
            .canonicalize()
            .map_err(|_| Error::storage())?;
        let parent = imports.parent().ok_or_else(Error::storage)?;
        let artifacts = parent.join("artifacts");
        if !artifacts.exists() {
            create(&artifacts)?;
        }
        let metadata = std::fs::symlink_metadata(&artifacts).map_err(|_| Error::storage())?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(Error::storage());
        }
        let artifacts = artifacts.canonicalize().map_err(|_| Error::storage())?;
        let path = artifacts.join("lab");
        let source = config
            .source_dir
            .as_ref()
            .ok_or_else(Error::storage)?
            .canonicalize()
            .map_err(|_| Error::storage())?;
        let state = config
            .state_dir
            .clone()
            .unwrap_or_else(|| parent.join("state"));
        let state = state.canonicalize().map_err(|_| Error::storage())?;
        for root in [&source, &imports, &state] {
            if path.starts_with(root) || root.starts_with(&path) {
                return Err(Error::storage());
            }
        }
        create(&path)?;
        let storage = Self {
            root: Arc::new(
                Dir::open_ambient_dir(&path, cap_std::ambient_authority())
                    .map_err(|_| Error::storage())?,
            ),
            workspace: config.workspace.as_ref().ok_or_else(Error::storage)?.id,
        };
        storage.recover()?;
        Ok(storage)
    }
    fn recover(&self) -> Result<()> {
        for (index, entry) in self
            .root
            .entries()
            .map_err(|_| Error::storage())?
            .take(MAX_RESULTS + 1)
            .enumerate()
        {
            if index == MAX_RESULTS {
                return Err(Error::limit());
            }
            let entry = entry.map_err(|_| Error::storage())?;
            let name = entry.file_name();
            let name = name.to_str().ok_or_else(Error::storage)?;
            let Some(id) = name.strip_prefix("pending-") else {
                continue;
            };
            if Uuid::parse_str(id).is_err() {
                return Err(Error::storage());
            }
            let metadata = self
                .root
                .symlink_metadata(name)
                .map_err(|_| Error::storage())?;
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err(Error::storage());
            }
            let directory = self.root.open_dir(name).map_err(|_| Error::storage())?;
            let mut files = Vec::new();
            for entry in directory.entries().map_err(|_| Error::storage())?.take(3) {
                let entry = entry.map_err(|_| Error::storage())?;
                let file = entry.file_name();
                if !matches!(file.to_str(), Some("record.json" | "value.bin"))
                    || !directory
                        .symlink_metadata(&file)
                        .map_err(|_| Error::storage())?
                        .is_file()
                {
                    return Err(Error::storage());
                }
                files.push(file);
            }
            for file in files {
                directory.remove_file(file).map_err(|_| Error::storage())?;
            }
            self.root.remove_dir(name).map_err(|_| Error::storage())?;
        }
        Ok(())
    }
    fn directory(&self, id: Uuid) -> Result<Dir> {
        if id.get_version_num() != 4 {
            return Err(Error::invalid());
        }
        let name = id.to_string();
        let metadata = self.root.symlink_metadata(&name).map_err(|_| {
            Error::new(
                "result_missing",
                "This Lab result is unavailable in this workspace.",
            )
        })?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(Error::storage());
        }
        self.root.open_dir(name).map_err(|_| Error::storage())
    }
    fn read(directory: &Dir, name: &str, maximum: usize) -> Result<Vec<u8>> {
        let metadata = directory
            .symlink_metadata(name)
            .map_err(|_| Error::storage())?;
        if !metadata.is_file()
            || metadata.file_type().is_symlink()
            || metadata.len() > maximum as u64
        {
            return Err(Error::storage());
        }
        #[cfg(unix)]
        {
            use cap_std::fs::MetadataExt;
            if metadata.nlink() != 1 {
                return Err(Error::storage());
            }
        }
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use cap_std::fs::OpenOptionsExt;
            options.custom_flags(
                (rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits() as i32,
            );
        }
        let file = directory
            .open_with(name, &options)
            .map_err(|_| Error::storage())?;
        if !file.metadata().map_err(|_| Error::storage())?.is_file() {
            return Err(Error::storage());
        }
        let mut bytes = Vec::new();
        file.take(maximum as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Error::storage())?;
        if bytes.len() > maximum {
            return Err(Error::limit());
        }
        Ok(bytes)
    }
    pub fn load(&self, id: Uuid) -> Result<(Record, Vec<u8>)> {
        let directory = self.directory(id)?;
        let record: Record =
            serde_json::from_slice(&Self::read(&directory, "record.json", MAX_RECORD)?)
                .map_err(|_| Error::storage())?;
        if record.id != id || record.workspace_id != self.workspace || record.byte_size > MAX_OUTPUT
        {
            return Err(Error::storage());
        }
        let data = Self::read(&directory, "value.bin", record.byte_size)?;
        if data.len() != record.byte_size || cube_core::hashing::sha256(&data) != record.sha256 {
            return Err(Error::storage());
        }
        Ok((record, data))
    }
    pub fn history(&self) -> Result<Vec<Record>> {
        let mut records = Vec::new();
        for (index, entry) in self
            .root
            .entries()
            .map_err(|_| Error::storage())?
            .take(MAX_RESULTS + 1)
            .enumerate()
        {
            if index == MAX_RESULTS {
                return Err(Error::limit());
            }
            let entry = entry.map_err(|_| Error::storage())?;
            let name = entry.file_name();
            if let Some(pending) = name.to_str().and_then(|name| name.strip_prefix("pending-")) {
                let id = Uuid::parse_str(pending).map_err(|_| Error::storage())?;
                if id.get_version_num() != 4 {
                    return Err(Error::storage());
                }
                continue;
            }
            let id = Uuid::parse_str(name.to_str().ok_or_else(Error::storage)?)
                .map_err(|_| Error::storage())?;
            records.push(self.load(id)?.0);
        }
        records.sort_by_key(|r| std::cmp::Reverse(r.provenance.created_ms));
        Ok(records)
    }
    pub fn record(
        &self,
        id: Uuid,
        output: WorkerOutput,
        provenance: Provenance,
        data: &[u8],
    ) -> Result<Record> {
        Ok(Record {
            id,
            workspace_id: self.workspace,
            representation: output.data.representation,
            semantic_type: output.data.semantic_type,
            byte_size: data.len(),
            sha256: cube_core::hashing::sha256(data),
            metadata: output.data.metadata,
            provenance,
        })
    }
    pub fn save(&self, id: Uuid, output: WorkerOutput, provenance: Provenance) -> Result<Record> {
        let data = output.data.bytes(MAX_OUTPUT)?;
        let mut count = 0;
        let mut bytes = 0;
        for entry in self
            .root
            .entries()
            .map_err(|_| Error::storage())?
            .take(MAX_RESULTS + 1)
        {
            let entry = entry.map_err(|_| Error::storage())?;
            count += 1;
            let name = entry.file_name();
            let name = name.to_str().ok_or_else(Error::storage)?;
            if name.starts_with("pending-") {
                continue;
            }
            let id = Uuid::parse_str(name).map_err(|_| Error::storage())?;
            let directory = self.directory(id)?;
            bytes += directory
                .symlink_metadata("value.bin")
                .map_err(|_| Error::storage())?
                .len();
        }
        if count >= MAX_RESULTS || bytes + data.len() as u64 > MAX_TOTAL {
            return Err(Error::new(
                "resource_limit",
                "Lab storage reached 64 results or 64 MiB. Preserve results before starting a new investigation.",
            ));
        }
        let record = self.record(id, output, provenance, &data)?;
        let encoded = json_bytes(&record, MAX_RECORD)?;
        let name = id.to_string();
        if self.root.symlink_metadata(&name).is_ok() {
            return Err(Error::new(
                "invalid_input",
                "This execution ID already exists; choose a new UUID.",
            ));
        }
        let pending = format!("pending-{id}");
        self.root.create_dir(&pending).map_err(|_| {
            Error::new(
                "invalid_input",
                "This execution ID already exists; choose a new UUID.",
            )
        })?;
        let directory = self.root.open_dir(&pending).map_err(|_| Error::storage())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            directory
                .set_permissions(
                    ".",
                    cap_std::fs::Permissions::from_std(std::fs::Permissions::from_mode(0o700)),
                )
                .map_err(|_| Error::storage())?;
        }
        let saved = (|| {
            for (name, bytes) in [
                ("value.bin", data.as_slice()),
                ("record.json", encoded.as_slice()),
            ] {
                let mut options = OpenOptions::new();
                options.write(true).create_new(true);
                #[cfg(unix)]
                {
                    use cap_std::fs::OpenOptionsExt;
                    options.mode(0o600);
                }
                let mut file = directory
                    .open_with(name, &options)
                    .map_err(|_| Error::storage())?;
                file.write_all(bytes)
                    .and_then(|_| file.sync_all())
                    .map_err(|_| Error::storage())?;
            }
            self.root
                .rename(&pending, &self.root, &name)
                .map_err(|_| Error::storage())?;
            #[cfg(unix)]
            self.root
                .open(".")
                .and_then(|file| file.sync_all())
                .map_err(|_| Error::storage())?;
            Ok(record)
        })();
        if saved.is_err() && self.root.symlink_metadata(&pending).is_ok() {
            let _ = directory.remove_file("record.json");
            let _ = directory.remove_file("value.bin");
            let _ = self.root.remove_dir(pending);
        }
        saved
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::WorkspaceContext;
    use serde_json::json;
    fn fixture(root: &Path) -> Config {
        for name in ["source", "imports", "state"] {
            let path = root.join(name);
            std::fs::create_dir(&path).unwrap();
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).unwrap();
            }
        }
        let mut config = Config::parse(
            "127.0.0.1:7331",
            "http://127.0.0.1:7331",
            "unused".into(),
            false,
        )
        .unwrap();
        config.source_dir = Some(root.join("source"));
        config.data_dir = Some(root.join("imports"));
        config.workspace = Some(WorkspaceContext {
            id: Uuid::new_v4(),
            name: "Storage test".into(),
        });
        config
    }
    fn output() -> WorkerOutput {
        WorkerOutput {
            data: Data::new(Representation::Bytes, "bytes", b"\0\xff", json!(null)),
            parameters: Vec::new(),
        }
    }
    fn provenance() -> Provenance {
        Provenance {
            operation: "encoding.base64.decode".into(),
            adapter_api: 1,
            pipeline_adapter: Some(1),
            engine_api: 1,
            core_version: super::super::execute::core_version(),
            parameters: Vec::new(),
            inputs: Vec::new(),
            created_ms: 1,
        }
    }
    #[test]
    fn atomic_bundles_recovery_quotas_and_cross_workspace_integrity() {
        let root = tempfile::tempdir().unwrap();
        let mut config = fixture(root.path());
        let store = Storage::open(&config).unwrap();
        let id = Uuid::new_v4();
        let record = store.save(id, output(), provenance()).unwrap();
        assert_eq!(store.load(id).unwrap().1, b"\0\xff");
        assert!(store.save(id, output(), provenance()).is_err());
        assert_eq!(store.load(id).unwrap().0.sha256, record.sha256);
        let pending = root
            .path()
            .join(format!("artifacts/lab/pending-{}", Uuid::new_v4()));
        std::fs::create_dir(&pending).unwrap();
        std::fs::write(pending.join("value.bin"), b"interrupted").unwrap();
        assert_eq!(store.history().unwrap().len(), 1);
        drop(store);
        let store = Storage::open(&config).unwrap();
        assert!(!pending.exists());
        assert!(store.load(id).is_ok());
        for _ in 1..MAX_RESULTS {
            store.save(Uuid::new_v4(), output(), provenance()).unwrap();
        }
        assert_eq!(
            store
                .save(Uuid::new_v4(), output(), provenance())
                .unwrap_err()
                .code,
            "resource_limit"
        );
        assert!(store.load(Uuid::nil()).is_err());
        config.workspace.as_mut().unwrap().id = Uuid::new_v4();
        let other = Storage::open(&config).unwrap();
        assert!(other.load(id).is_err());
        let unsafe_root = root.path().join("artifacts/lab");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&unsafe_root, std::fs::Permissions::from_mode(0o755)).unwrap();
            assert!(Storage::open(&config).is_err());
        }
        let _ = unsafe_root;
    }
}
