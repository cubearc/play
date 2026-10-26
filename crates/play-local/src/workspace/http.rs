use std::{sync::Arc, time::Duration};

use axum::{
    Json,
    body::{Body, Bytes},
    extract::{Query, Request, State, rejection::QueryRejection},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};
use futures_util::StreamExt;
use serde::Deserialize;
use tokio::{
    io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt},
    sync::OwnedSemaphorePermit,
};
use tokio_util::io::ReaderStream;

use super::{FileError, MAX_IMPORT, Result, Store, Workspace, inspect::Media};
use crate::AppState;

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    #[serde(default)]
    store: Store,
    #[serde(default)]
    path: String,
    #[serde(default)]
    offset: u64,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Import {
    name: String,
}

fn workspace(state: &AppState) -> Result<Arc<Workspace>> {
    state
        .workspace
        .as_ref()
        .map_err(|error| *error)?
        .clone()
        .ok_or(FileError(
            StatusCode::CONFLICT,
            "workspace_required",
            "Open a local directory with cube play to browse or import files.",
        ))
}
pub(crate) async fn permit(workspace: &Workspace) -> Result<OwnedSemaphorePermit> {
    let busy = || {
        FileError(
            StatusCode::TOO_MANY_REQUESTS,
            "workspace_busy",
            "Two file operations are already active. Cancel a preview or retry shortly.",
        )
    };
    tokio::time::timeout(
        Duration::from_secs(2),
        workspace.slots.clone().acquire_owned(),
    )
    .await
    .map_err(|_| busy())?
    .map_err(|_| busy())
}
fn query<T>(value: std::result::Result<Query<T>, QueryRejection>) -> Result<T> {
    value
        .map(|Query(value)| value)
        .map_err(|_| super::invalid())
}
fn failed() -> FileError {
    FileError(
        StatusCode::INTERNAL_SERVER_ERROR,
        "file_operation_failed",
        "The local file operation could not finish. Retry or reopen this workspace.",
    )
}

pub(crate) async fn files(
    State(state): State<AppState>,
    selection: std::result::Result<Query<Selection>, QueryRejection>,
) -> Result<Json<super::Listing>> {
    let selection = query(selection)?;
    let database = state.database().await?;
    let workspace = workspace(&state)?;
    let permit = permit(&workspace).await?;
    let offset = selection.offset.try_into().map_err(|_| super::invalid())?;
    let mut result = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        workspace.listing(selection.store, &selection.path, offset)
    })
    .await
    .map_err(|_| failed())??;
    if selection.store == Store::Imports
        && let Some(database) = database
    {
        for entry in &mut result.entries {
            if let Some(path) = &entry.path {
                match database.authorize_import(path).await {
                    Ok(()) => {}
                    Err(error) if error.1 == "artifact_unavailable" => entry.accessible = false,
                    Err(error) => return Err(error),
                }
            }
        }
    }
    Ok(Json(result))
}

pub(crate) async fn inspection(
    State(state): State<AppState>,
    selection: std::result::Result<Query<Selection>, QueryRejection>,
) -> Result<Json<super::inspect::Inspection>> {
    let selection = query(selection)?;
    let database = state.database().await?;
    let workspace = workspace(&state)?;
    let permit = permit(&workspace).await?;
    if selection.store == Store::Imports
        && let Some(database) = database
    {
        database.authorize_import(&selection.path).await?;
    }
    let result = tokio::task::spawn_blocking(move || {
        let _permit = permit;
        workspace.inspect(selection.store, &selection.path, selection.offset)
    })
    .await
    .map_err(|_| failed())??;
    Ok(Json(result))
}

/// One inclusive HTTP byte range, with checked arithmetic and no multipart amplification.
fn range(headers: &HeaderMap, size: u64) -> std::result::Result<Option<(u64, u64)>, ()> {
    if headers.get_all(header::RANGE).iter().count() > 1 {
        return Err(());
    }
    let Some(value) = headers.get(header::RANGE) else {
        return Ok(None);
    };
    let value = value.to_str().map_err(|_| ())?;
    if value.len() > 80 || size == 0 {
        return Err(());
    }
    let value = value.strip_prefix("bytes=").ok_or(())?;
    let (start, end) = value.split_once('-').ok_or(())?;
    let parse = |value: &str| {
        if !value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()) {
            value.parse::<u64>().map_err(|_| ())
        } else {
            Err(())
        }
    };
    if start.is_empty() {
        let suffix = parse(end)?;
        if suffix == 0 {
            return Err(());
        }
        return Ok(Some((size.saturating_sub(suffix), size - 1)));
    }
    let start = parse(start)?;
    let end = if end.is_empty() {
        size - 1
    } else {
        parse(end)?.min(size - 1)
    };
    if start >= size || end < start {
        return Err(());
    }
    Ok(Some((start, end)))
}

pub(crate) async fn media(
    State(state): State<AppState>,
    selection: std::result::Result<Query<Selection>, QueryRejection>,
    headers: HeaderMap,
) -> Result<Response> {
    let selection = query(selection)?;
    if selection.offset != 0 {
        return Err(super::invalid());
    }
    let database = state.database().await?;
    let workspace = workspace(&state)?;
    let permit = permit(&workspace).await?;
    if selection.store == Store::Imports
        && let Some(database) = database
    {
        database.authorize_import(&selection.path).await?;
    }
    let prepared =
        tokio::task::spawn_blocking(move || workspace.media(selection.store, &selection.path))
            .await
            .map_err(|_| failed())??;
    let (size, mime) = match &prepared {
        Media::Image { bytes, mime } => (bytes.len() as u64, *mime),
        Media::File { size, mime, .. } => (*size, *mime),
    };
    let selected = match range(&headers, size) {
        Ok(value) => value,
        Err(()) => {
            let mut response = StatusCode::RANGE_NOT_SATISFIABLE.into_response();
            response.headers_mut().insert(
                header::CONTENT_RANGE,
                format!("bytes */{size}").parse().map_err(|_| failed())?,
            );
            return Ok(response);
        }
    };
    let (start, end) = selected.unwrap_or((0, size.saturating_sub(1)));
    let length = if size == 0 { 0 } else { end - start + 1 };
    let body = match prepared {
        Media::Image { bytes, .. } => {
            let bytes = Bytes::from(bytes).slice(start as usize..(start + length) as usize);
            Body::from_stream(
                futures_util::stream::iter([Ok::<_, std::io::Error>(bytes)]).map(move |chunk| {
                    let _ = &permit;
                    chunk
                }),
            )
        }
        Media::File { file, .. } => {
            let mut file = tokio::fs::File::from_std(file);
            file.seek(std::io::SeekFrom::Start(start)).await?;
            Body::from_stream(
                ReaderStream::with_capacity(file.take(length), 64 * 1024).map(move |chunk| {
                    let _ = &permit;
                    chunk
                }),
            )
        }
    };
    let mut response = (
        if selected.is_some() {
            StatusCode::PARTIAL_CONTENT
        } else {
            StatusCode::OK
        },
        body,
    )
        .into_response();
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, mime.parse().map_err(|_| failed())?);
    headers.insert(
        header::CONTENT_LENGTH,
        length.to_string().parse().map_err(|_| failed())?,
    );
    headers.insert(
        header::CONTENT_DISPOSITION,
        axum::http::HeaderValue::from_static("inline"),
    );
    headers.insert(
        header::ACCEPT_RANGES,
        axum::http::HeaderValue::from_static("bytes"),
    );
    if selected.is_some() {
        headers.insert(
            header::CONTENT_RANGE,
            format!("bytes {start}-{end}/{size}")
                .parse()
                .map_err(|_| failed())?,
        );
    }
    Ok(response)
}

pub(crate) async fn import(
    State(state): State<AppState>,
    selection: std::result::Result<Query<Import>, QueryRejection>,
    request: Request,
) -> Result<Response> {
    let selection = query(selection)?;
    let database = state.database().await?;
    let workspace = workspace(&state)?;
    let permit = permit(&workspace).await?;
    let import_workspace = workspace.clone();
    let name = selection.name.clone();
    let (file, pending) = tokio::task::spawn_blocking(move || import_workspace.begin_import(&name))
        .await
        .map_err(|_| failed())??;
    let mut file = tokio::fs::File::from_std(file);
    let mut body = request.into_body().into_data_stream();
    let length=tokio::time::timeout(Duration::from_secs(30),async {
        let mut length=0u64;
        while let Some(chunk)=body.next().await {
            let chunk=chunk.map_err(|_|FileError(StatusCode::BAD_REQUEST,"upload_interrupted","The import was interrupted or exceeded its body limit; no completed file was created."))?;
            length=length.checked_add(chunk.len() as u64).ok_or_else(super::limit)?;
            if length>MAX_IMPORT{return Err(super::limit())}
            file.write_all(&chunk).await?;
        }
        file.sync_all().await?;
        Ok::<_,FileError>(length)
    }).await.map_err(|_|FileError(StatusCode::REQUEST_TIMEOUT,"upload_timeout","The import exceeded 30 seconds. Its temporary file was removed; retry with a smaller file."))??;
    drop(file);
    let reservation = if let Some(database) = &database {
        Some(
            database
                .reserve_import(&selection.name, &pending.temporary, length)
                .await?,
        )
    } else {
        None
    };
    // Finish filesystem/DB publication even if the HTTP client disconnects.
    tokio::spawn(async move {
        let committed = tokio::task::spawn_blocking(move || {
            pending.commit()?;
            drop(pending);
            Ok::<_, FileError>(())
        })
        .await
        .map_err(|_| failed())?;
        if let (Some(database), Some(id)) = (&database, &reservation) {
            if committed.is_ok() {
                database.finish_import(id).await?;
            } else {
                database.cancel_import(id).await?;
            }
        }
        committed?;
        drop(permit);
        Ok::<_, FileError>(())
    })
    .await
    .map_err(|_| failed())??;
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({"store":"imports","path":selection.name})),
    )
        .into_response())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn byte_ranges_are_exact_and_bounded() {
        for (value, expected) in [
            ("bytes=0-2", Some((0, 2))),
            ("bytes=2-", Some((2, 9))),
            ("bytes=-3", Some((7, 9))),
            ("bytes=0-999", Some((0, 9))),
            ("bytes=-999", Some((0, 9))),
        ] {
            let mut headers = HeaderMap::new();
            headers.insert(header::RANGE, value.parse().unwrap());
            assert_eq!(range(&headers, 10), Ok(expected));
        }
        for value in [
            "bytes=10-",
            "bytes=3-2",
            "bytes=-0",
            "bytes=0-1,2-3",
            "items=0-1",
            "bytes=+0-1",
            "bytes=0-18446744073709551616",
        ] {
            let mut headers = HeaderMap::new();
            headers.insert(header::RANGE, value.parse().unwrap());
            assert!(range(&headers, 10).is_err());
            assert!(range(&headers, 0).is_err());
        }
    }
}
