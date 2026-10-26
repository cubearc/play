//! Authorized filesystem handles, bounded discovery and explicit managed imports.
mod http;
mod inspect;

use std::{io, path::Path, sync::Arc, time::UNIX_EPOCH};

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use cap_std::fs::{Dir, OpenOptions};
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;

use crate::{config::Config, security};

pub(crate) use http::{files, import, inspection, media};
pub const MAX_ENTRIES: usize = 4096;
pub const PAGE_SIZE: usize = 128;
pub const MAX_IMPORT: u64 = 32 * 1024 * 1024;
pub const MAX_MEDIA: u64 = 256 * 1024 * 1024;
pub const MAX_IMAGE: u64 = 16 * 1024 * 1024;
const TEMP_PREFIX: &str = ".cube-play-import-";

#[derive(Clone, Copy, Debug)]
pub struct FileError(pub StatusCode, pub &'static str, pub &'static str);
impl IntoResponse for FileError {
    fn into_response(self) -> Response {
        security::error(self.0, self.1, self.2)
    }
}
impl From<io::Error> for FileError {
    fn from(error: io::Error) -> Self {
        match error.kind() {
            io::ErrorKind::NotFound => Self(
                StatusCode::NOT_FOUND,
                "file_missing",
                "The selected file or directory is no longer available. Refresh the explorer.",
            ),
            io::ErrorKind::PermissionDenied => Self(
                StatusCode::FORBIDDEN,
                "path_denied",
                "The selected path is inaccessible or leaves the authorized workspace.",
            ),
            io::ErrorKind::AlreadyExists => Self(
                StatusCode::CONFLICT,
                "file_exists",
                "An imported file already uses that name. Choose another name; existing data was preserved.",
            ),
            _ => Self(
                StatusCode::BAD_REQUEST,
                "file_unavailable",
                "The selected file cannot be accessed safely. Source files were not changed.",
            ),
        }
    }
}
pub type Result<T> = std::result::Result<T, FileError>;
pub fn invalid() -> FileError {
    FileError(
        StatusCode::BAD_REQUEST,
        "invalid_path",
        "Use a relative workspace path with at most 32 normal components; traversal, device names and reserved names are not allowed.",
    )
}
pub fn limit() -> FileError {
    FileError(
        StatusCode::PAYLOAD_TOO_LARGE,
        "file_limit",
        "The selected content exceeds the preview or import limit. Metadata and bounded byte inspection remain available.",
    )
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Store {
    #[default]
    Source,
    Imports,
}

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Image,
    Video,
    Audio,
    Text,
    Binary,
    Unknown,
}

/// Filename categories are UI hints, never claims about structural validity.
fn hint(name: &str) -> (Category, &'static str) {
    let extension = Path::new(name)
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match extension.as_str() {
        "png" => (Category::Image, "image/png"),
        "jpg" | "jpeg" => (Category::Image, "image/jpeg"),
        "gif" => (Category::Image, "image/gif"),
        "bmp" => (Category::Image, "image/bmp"),
        "webp" => (Category::Image, "image/webp"),
        "mp4" | "m4v" => (Category::Video, "video/mp4"),
        "mov" => (Category::Video, "video/quicktime"),
        "webm" => (Category::Video, "video/webm"),
        "wav" | "wave" => (Category::Audio, "audio/wav"),
        "mp3" => (Category::Audio, "audio/mpeg"),
        "ogg" | "oga" => (Category::Audio, "audio/ogg"),
        "m4a" => (Category::Audio, "audio/mp4"),
        "txt" | "md" | "json" | "csv" | "log" | "toml" | "yaml" | "yml" | "xml" | "html"
        | "htm" | "js" | "ts" | "css" | "rs" | "py" | "cube" | "svg" => {
            (Category::Text, "text/plain")
        }
        "bin" | "exe" | "dll" | "so" | "zip" | "gz" | "pdf" => {
            (Category::Binary, "application/octet-stream")
        }
        _ => (Category::Unknown, "application/octet-stream"),
    }
}

pub fn validate(path: &str, allow_root: bool) -> Result<()> {
    if path.is_empty() {
        return if allow_root { Ok(()) } else { Err(invalid()) };
    }
    if path.len() > 4096 || path.split('/').count() > 32 {
        return Err(invalid());
    }
    for name in path.split('/') {
        let stem = name.split('.').next().unwrap_or("").to_uppercase();
        let device = matches!(
            stem.as_str(),
            "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
        ) || ["COM", "LPT"].iter().any(|prefix| {
            stem.strip_prefix(prefix).is_some_and(|suffix| {
                matches!(
                    suffix,
                    "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            })
        });
        if name.is_empty()
            || name == "."
            || name == ".."
            || name.len() > 255
            || name.ends_with(['.', ' '])
            || name.chars().any(|c| {
                c.is_control() || matches!(c, '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
            })
            || device
        {
            return Err(invalid());
        }
    }
    Ok(())
}

pub struct Workspace {
    source: Arc<Dir>,
    imports: Option<Arc<Dir>>,
    pub slots: Arc<Semaphore>,
}

impl Workspace {
    pub fn open(config: &Config) -> Result<Option<Arc<Self>>> {
        let Some(source) = &config.source_dir else {
            return Ok(None);
        };
        let source = source.canonicalize()?;
        let root = Dir::open_ambient_dir(&source, cap_std::ambient_authority())?;
        let imports = if let Some(path) = &config.data_dir {
            if path.starts_with(&source) || source.starts_with(path) {
                return Err(invalid());
            }
            let mut builder = std::fs::DirBuilder::new();
            builder.recursive(false);
            #[cfg(unix)]
            {
                use std::os::unix::fs::DirBuilderExt;
                builder.mode(0o700);
            }
            let parent = path.parent().ok_or_else(invalid)?.canonicalize()?;
            let expected = parent.join(path.file_name().ok_or_else(invalid)?);
            if expected.starts_with(&source) || source.starts_with(&expected) {
                return Err(invalid());
            }
            match std::fs::symlink_metadata(path) {
                Ok(metadata) if metadata.file_type().is_symlink() => return Err(invalid()),
                Ok(_) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => builder.create(path)?,
                Err(error) => return Err(error.into()),
            }
            let metadata = std::fs::symlink_metadata(path)?;
            if !metadata.is_dir() || metadata.file_type().is_symlink() {
                return Err(invalid());
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if metadata.permissions().mode() & 0o077 != 0 {
                    return Err(FileError(
                        StatusCode::FORBIDDEN,
                        "unsafe_storage",
                        "Managed import storage must be private to the runtime user.",
                    ));
                }
            }
            let canonical = path.canonicalize()?;
            if canonical.starts_with(&source) || source.starts_with(&canonical) {
                return Err(invalid());
            }
            Some(Arc::new(Dir::open_ambient_dir(
                canonical,
                cap_std::ambient_authority(),
            )?))
        } else {
            None
        };
        Ok(Some(Arc::new(Self {
            source: Arc::new(root),
            imports,
            slots: Arc::new(Semaphore::new(2)),
        })))
    }

    pub fn root(&self, store: Store) -> Result<&Arc<Dir>> {
        match store {
            Store::Source => Ok(&self.source),
            Store::Imports => self.imports.as_ref().ok_or(FileError(
                StatusCode::CONFLICT,
                "imports_unavailable",
                "Managed import storage is not configured. Open this workspace through cube play.",
            )),
        }
    }

    fn check(&self, store: Store, path: &str) -> Result<()> {
        validate(path, true)?;
        let root = self.root(store)?;
        let mut prefix = std::path::PathBuf::new();
        for component in Path::new(path).components() {
            prefix.push(component);
            if root.symlink_metadata(&prefix)?.file_type().is_symlink() {
                return Err(FileError(
                    StatusCode::FORBIDDEN,
                    "symlink_denied",
                    "Symbolic links are shown but not opened. Select a regular file inside the authorized workspace.",
                ));
            }
        }
        Ok(())
    }

    pub fn file(&self, store: Store, path: &str) -> Result<std::fs::File> {
        validate(path, false)?;
        self.check(store, path)?;
        let root = self.root(store)?;
        if !root.symlink_metadata(path)?.is_file() {
            return Err(invalid());
        }
        let mut options = OpenOptions::new();
        options.read(true);
        #[cfg(unix)]
        {
            use cap_std::fs::OpenOptionsExt;
            options.custom_flags(
                (rustix::fs::OFlags::NONBLOCK | rustix::fs::OFlags::NOFOLLOW).bits() as i32,
            );
        }
        let file = root.open_with(path, &options)?.into_std();
        if !file.metadata()?.is_file() {
            return Err(invalid());
        }
        Ok(file)
    }

    pub fn listing(&self, store: Store, path: &str, offset: usize) -> Result<Listing> {
        self.check(store, path)?;
        if offset >= MAX_ENTRIES || !offset.is_multiple_of(PAGE_SIZE) {
            return Err(invalid());
        }
        let root = self.root(store)?;
        let directory = root.open_dir(if path.is_empty() { "." } else { path })?;
        let mut names = Vec::new();
        let mut truncated = false;
        for (index, entry) in directory.entries()?.take(MAX_ENTRIES + 1).enumerate() {
            let entry = entry?;
            if index == MAX_ENTRIES {
                truncated = true;
                break;
            }
            let name = entry.file_name();
            if store == Store::Imports && name.to_string_lossy().starts_with(TEMP_PREFIX) {
                continue;
            }
            names.push(name);
        }
        names.sort();
        let total = names.len();
        let mut entries = Vec::new();
        for name in names.into_iter().skip(offset).take(PAGE_SIZE) {
            let display = name.to_string_lossy().into_owned();
            let relative = if path.is_empty() {
                display.clone()
            } else {
                format!("{path}/{display}")
            };
            let valid = name.to_str().is_some() && validate(&relative, false).is_ok();
            let metadata = directory.symlink_metadata(&name).ok();
            let kind = metadata
                .as_ref()
                .map(|m| {
                    if m.file_type().is_symlink() {
                        "symlink"
                    } else if m.is_dir() {
                        "directory"
                    } else if m.is_file() {
                        "file"
                    } else {
                        "special"
                    }
                })
                .unwrap_or("unavailable");
            entries.push(Entry {
                name: display.clone(),
                path: valid.then_some(relative),
                kind,
                category: hint(&display).0,
                size: metadata.as_ref().filter(|m| m.is_file()).map(|m| m.len()),
                modified_ms: metadata
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.into_std().duration_since(UNIX_EPOCH).ok())
                    .and_then(|d| d.as_millis().try_into().ok()),
                accessible: valid && matches!(kind, "directory" | "file"),
            });
        }
        Ok(Listing {
            store,
            path: path.into(),
            entries,
            next_offset: (offset + PAGE_SIZE < total).then_some(offset + PAGE_SIZE),
            truncated,
            imports_available: self.imports.is_some(),
            page_size: PAGE_SIZE,
            entry_limit: MAX_ENTRIES,
        })
    }

    pub fn begin_import(&self, name: &str) -> Result<(std::fs::File, PendingImport)> {
        validate(name, false)?;
        if name.contains('/') || name.starts_with(TEMP_PREFIX) {
            return Err(invalid());
        }
        let root = self.root(Store::Imports)?.clone();
        let temporary = format!("{TEMP_PREFIX}{}", uuid::Uuid::new_v4());
        let mut options = OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use cap_std::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let file = root.open_with(&temporary, &options)?.into_std();
        Ok((
            file,
            PendingImport {
                root,
                temporary,
                target: name.into(),
            },
        ))
    }
}

pub struct PendingImport {
    root: Arc<Dir>,
    temporary: String,
    target: String,
}
impl PendingImport {
    pub fn commit(&self) -> Result<()> {
        self.root
            .hard_link(&self.temporary, &self.root, &self.target)?;
        Ok(())
    }
}
impl Drop for PendingImport {
    fn drop(&mut self) {
        let _ = self.root.remove_file(&self.temporary);
    }
}

#[derive(Serialize)]
pub struct Entry {
    pub name: String,
    pub path: Option<String>,
    pub kind: &'static str,
    pub category: Category,
    pub size: Option<u64>,
    pub modified_ms: Option<u64>,
    pub accessible: bool,
}
#[derive(Serialize)]
pub struct Listing {
    store: Store,
    path: String,
    entries: Vec<Entry>,
    next_offset: Option<usize>,
    truncated: bool,
    imports_available: bool,
    page_size: usize,
    entry_limit: usize,
}
