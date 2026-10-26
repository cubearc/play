use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
};

use cube_core::forensics::{
    ByteRange,
    ranges::{self, HexRow},
    signatures::{self, Signature},
};
use serde::Serialize;

use super::{
    Category, Entry, FileError, MAX_IMAGE, MAX_MEDIA, Result, Store, Workspace, hint, limit,
};

pub const TEXT_BYTES: usize = 64 * 1024;
pub const HEX_BYTES: u64 = 4096;

#[derive(Serialize)]
pub struct Inspection {
    pub store: Store,
    pub path: String,
    pub entry: Entry,
    pub signatures: Vec<Signature>,
    pub extension_mismatch: bool,
    pub preview: Preview,
    pub text: Option<Text>,
    pub rows: Vec<HexRow>,
    pub offset: u64,
    pub next_offset: Option<u64>,
    pub notice: Option<&'static str>,
    pub image: Option<Dimensions>,
    pub media: Option<cube_core::media::video::Info>,
}
#[derive(Serialize)]
pub struct Preview {
    pub kind: Category,
    pub available: bool,
    pub mime: &'static str,
}
#[derive(Serialize)]
pub struct Text {
    pub value: String,
    pub bytes: usize,
    pub truncated: bool,
}
#[derive(Serialize)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

pub fn bounded_image(file: &mut File) -> Result<(Vec<u8>, Dimensions)> {
    file.seek(SeekFrom::Start(0))?;
    if file.metadata()?.len() > MAX_IMAGE {
        return Err(limit());
    }
    let mut bytes = Vec::new();
    (&mut *file).take(MAX_IMAGE + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_IMAGE {
        return Err(limit());
    }
    let image=cube_core::image::Image::decode(&bytes).map_err(|_|FileError(axum::http::StatusCode::UNPROCESSABLE_ENTITY,"image_unsupported","This image is malformed or outside Cube Core's supported encoded/pixel limits. Bounded byte inspection remains available."))?;
    let info = image.info();
    Ok((
        bytes,
        Dimensions {
            width: info.width,
            height: info.height,
        },
    ))
}

fn classification(
    file: &mut File,
    name: &str,
) -> Result<(Category, &'static str, Vec<Signature>, bool)> {
    file.seek(SeekFrom::Start(0))?;
    let mut header = Vec::new();
    (&mut *file).take(12).read_to_end(&mut header)?;
    let signatures = signatures::detect(&header);
    let (declared, mime) = hint(name);
    let observed = signatures.first().map(|s| s.mime);
    let mismatch = observed.is_some_and(|actual| actual != mime);
    let (kind, mime) = match observed {
        Some(mime) if mime.starts_with("image/") => (Category::Image, mime),
        Some(mime) if mime.starts_with("audio/") => (Category::Audio, mime),
        Some(_) => (Category::Binary, "application/octet-stream"),
        None => (declared, mime),
    };
    Ok((kind, mime, signatures, mismatch))
}

impl Workspace {
    pub fn inspect(&self, store: Store, path: &str, offset: u64) -> Result<Inspection> {
        let mut file = self.file(store, path)?;
        let metadata = file.metadata()?;
        let size = metadata.len();
        if offset > size {
            return Err(super::invalid());
        }
        let name = path.rsplit('/').next().unwrap_or(path).to_string();
        let (kind, mime, signatures, mismatch) = classification(&mut file, &name)?;
        let length = (size - offset).min(HEX_BYTES);
        let rows = ranges::hex_rows(
            &mut file,
            ByteRange {
                offset,
                length: Some(length),
            },
            16,
        )
        .map_err(|_| {
            FileError(
                axum::http::StatusCode::CONFLICT,
                "file_changed",
                "The file changed during inspection. Refresh and try again.",
            )
        })?;
        let filename_category = hint(&name).0;
        let mut inspection = Inspection {
            store,
            path: path.into(),
            entry: Entry {
                name,
                path: Some(path.into()),
                kind: "file",
                category: filename_category,
                size: Some(size),
                modified_ms: metadata
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .and_then(|d| d.as_millis().try_into().ok()),
                accessible: true,
            },
            signatures,
            extension_mismatch: mismatch,
            preview: Preview {
                kind,
                available: false,
                mime,
            },
            text: None,
            rows,
            offset,
            next_offset: (offset + length < size).then_some(offset + length),
            notice: None,
            image: None,
            media: None,
        };
        match kind {
            Category::Image => match bounded_image(&mut file) {
                Ok((_, dimensions)) => {
                    inspection.image = Some(dimensions);
                    inspection.preview.available = true;
                }
                Err(error) => inspection.notice = Some(error.2),
            },
            Category::Video => {
                if size > MAX_MEDIA {
                    inspection.notice = Some(limit().2);
                } else {
                    match cube_core::media::video::inspect(&mut file) {
                        Ok(info) => {
                            inspection.media = Some(info);
                            inspection.preview.available = true;
                            inspection.notice = Some(
                                "Playback depends on browser codec support. Container metadata does not prove packet validity.",
                            );
                        }
                        Err(_) => {
                            inspection.notice = Some(
                                "This container is malformed or outside the supported Cube metadata profiles. File metadata and bounded byte inspection remain available.",
                            )
                        }
                    }
                }
            }
            Category::Audio => {
                inspection.preview.available = size <= MAX_MEDIA;
                inspection.notice = Some(if size > MAX_MEDIA {
                    limit().2
                } else {
                    "Playback depends on browser codec support. Use byte inspection if the browser cannot decode this audio."
                });
            }
            Category::Text | Category::Unknown => {
                file.seek(SeekFrom::Start(0))?;
                let mut bytes = Vec::new();
                (&mut file)
                    .take(TEXT_BYTES as u64)
                    .read_to_end(&mut bytes)?;
                let truncated = size > bytes.len() as u64;
                let utf8 = match std::str::from_utf8(&bytes) {
                    Ok(text) => Some(text),
                    Err(error) if error.error_len().is_none() && truncated => {
                        std::str::from_utf8(&bytes[..error.valid_up_to()]).ok()
                    }
                    Err(_) => None,
                };
                if let Some(text) = utf8.filter(|text| {
                    !text
                        .chars()
                        .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
                }) {
                    inspection.preview = Preview {
                        kind: Category::Text,
                        available: true,
                        mime: "text/plain",
                    };
                    inspection.text = Some(Text {
                        value: text.into(),
                        bytes: text.len(),
                        truncated: size > text.len() as u64,
                    });
                } else {
                    inspection.notice = Some(
                        "This content is not a supported UTF-8 text preview. Original bytes are preserved below.",
                    );
                }
            }
            Category::Binary => {
                inspection.notice = Some(
                    "No inline decoder is selected for this format. Inspect its original bytes below.",
                )
            }
        }
        Ok(inspection)
    }

    pub fn media(&self, store: Store, path: &str) -> Result<Media> {
        let mut file = self.file(store, path)?;
        let name = path.rsplit('/').next().unwrap_or(path);
        let (kind, mime, _, _) = classification(&mut file, name)?;
        match kind {
            Category::Image
                if matches!(
                    mime,
                    "image/png" | "image/jpeg" | "image/gif" | "image/bmp" | "image/webp"
                ) =>
            {
                let (bytes, _) = bounded_image(&mut file)?;
                Ok(Media::Image { bytes, mime })
            }
            Category::Audio | Category::Video => {
                let size = file.metadata()?.len();
                if size > MAX_MEDIA {
                    return Err(limit());
                }
                if kind == Category::Video {
                    cube_core::media::video::inspect(&mut file).map_err(|_|FileError(axum::http::StatusCode::UNPROCESSABLE_ENTITY,"media_unsupported","Container inspection is unavailable. Use the file metadata and byte inspector."))?;
                }
                Ok(Media::File { file, size, mime })
            }
            _ => Err(FileError(
                axum::http::StatusCode::UNSUPPORTED_MEDIA_TYPE,
                "preview_unsupported",
                "This format is available through inert text or byte inspection, not inline media serving.",
            )),
        }
    }
}

pub enum Media {
    Image {
        bytes: Vec<u8>,
        mime: &'static str,
    },
    File {
        file: File,
        size: u64,
        mime: &'static str,
    },
}
