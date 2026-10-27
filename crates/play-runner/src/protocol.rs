use serde::{Deserialize, Serialize};

pub const FRAME_LIMIT: usize = 32768;
pub const INPUT_FRAME: usize = 4096;
pub const INPUT_LIMIT: usize = 4 * 1024 * 1024;
pub const OUTPUT_LIMIT: usize = 8 * 1024 * 1024;
pub const SESSION_LIMIT: usize = 2;
pub const IDLE_SECONDS: u64 = 300;
pub const LIFETIME_SECONDS: u64 = 1800;
pub const COMMAND_SECONDS: u64 = 30;
pub const CAPTURE_LIMIT: usize = 128 * 1024;

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Size {
    pub cols: u16,
    pub rows: u16,
}
impl Size {
    pub fn valid(self) -> bool {
        (20..=240).contains(&self.cols) && (5..=80).contains(&self.rows)
    }
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Store {
    #[default]
    Source,
    Imports,
    Output,
}
impl Store {
    pub fn name(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Imports => "imports",
            Self::Output => "output",
        }
    }
}
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Location {
    pub store: Store,
    pub path: String,
}
impl Location {
    pub fn valid(&self) -> bool {
        valid_path(&self.path, true)
    }
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Client {
    Probe,
    Start { location: Location, size: Size },
    Input { data: String },
    Resize { size: Size },
    Close,
    CaptureList,
    CaptureRead { id: String },
}
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Server {
    Status {
        active: usize,
        limit: usize,
    },
    Ready {
        id: uuid::Uuid,
        size: Size,
    },
    Resized {
        size: Size,
    },
    Output {
        data: Vec<u8>,
    },
    End {
        reason: String,
    },
    Error {
        code: String,
    },
    Captures {
        ids: Vec<String>,
    },
    CaptureData {
        id: String,
        data: Vec<u8>,
        done: bool,
    },
}
pub fn valid_capture(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 96
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}
impl Server {
    pub fn error(code: &str) -> Self {
        Self::Error { code: code.into() }
    }
}
/// Shared filesystem syntax; authorization still resolves each path within its root.
pub fn valid_path(path: &str, allow_root: bool) -> bool {
    if path.is_empty() {
        return allow_root;
    }
    if path.len() > 4096 || path.split('/').count() > 32 {
        return false;
    }
    path.split('/').all(|name| {
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
        !name.is_empty()
            && name != "."
            && name != ".."
            && name.len() <= 255
            && !name.ends_with(['.', ' '])
            && !name.chars().any(|c| {
                c.is_control() || matches!(c, '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|')
            })
            && !device
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn typed_frames_and_shared_paths_reject_escapes_and_unbounded_dimensions() {
        for size in [
            Size { cols: 20, rows: 5 },
            Size {
                cols: 240,
                rows: 80,
            },
        ] {
            assert!(size.valid());
        }
        for size in [
            Size { cols: 0, rows: 24 },
            Size {
                cols: 241,
                rows: 24,
            },
            Size { cols: 80, rows: 81 },
        ] {
            assert!(!size.valid());
        }
        for path in ["nested/file.txt", "空/name", "a"] {
            assert!(valid_path(path, false));
        }
        for path in [
            "",
            "../escape",
            "a/../x",
            "/etc/passwd",
            "a\\b",
            "NUL.txt",
            "COM1",
            "a.",
            "a/",
            "a//b",
        ] {
            assert!(!valid_path(path, false));
        }
        assert!(valid_path("", true));
        assert!(!valid_path(&"a/".repeat(33), true));
        assert!(!valid_path(&"a".repeat(4097), true));
        assert!(!valid_path(&"a".repeat(256), true));
        assert!(
            serde_json::from_str::<Client>(
                r#"{"type":"input","data":"cube --help\r","extra":true}"#
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<Client>(r#"{"type":"resize","size":{"cols":-1,"rows":24}}"#)
                .is_err()
        );
        let frame = Server::Output {
            data: vec![0, 255, 128],
        };
        assert!(
            matches!(serde_json::from_str::<Server>(&serde_json::to_string(&frame).unwrap()).unwrap(),Server::Output{data} if data==[0,255,128])
        );
    }
}
