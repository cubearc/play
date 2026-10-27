//! Short-lived capabilities authorize one media/export URL, never a whole API.
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

use axum::{
    Json,
    extract::{Request, State},
    http::Uri,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::matches_secret;
use crate::{AppState, workspace::FileError};

const MAX_GRANTS: usize = 256;
const LIFETIME: Duration = Duration::from_secs(3600);
struct Grant {
    token: String,
    uri: String,
    expires: Instant,
}
#[derive(Default)]
pub(crate) struct ReadGrants(Mutex<Vec<Grant>>);
fn invalid() -> FileError {
    FileError(
        axum::http::StatusCode::BAD_REQUEST,
        "invalid_read_link",
        "Select a local media or artifact content URL.",
    )
}
fn unavailable() -> FileError {
    FileError(
        axum::http::StatusCode::SERVICE_UNAVAILABLE,
        "read_link_unavailable",
        "The private media link is unavailable. Reopen the workspace.",
    )
}
fn allowed(value: &str) -> bool {
    let Ok(uri) = value.parse::<Uri>() else {
        return false;
    };
    if value.len() > 16 * 1024 - 128 || uri.scheme().is_some() || uri.authority().is_some() {
        return false;
    }
    if uri.query().is_some_and(|q| {
        q.split('&')
            .any(|p| p.split('=').next() == Some("read_cap"))
    }) {
        return false;
    }
    if uri.path() == "/api/workspace/media" {
        return true;
    }
    let Some(path) = uri.path().strip_prefix("/api/lab/results/") else {
        return false;
    };
    let Some((id, kind)) = path.split_once('/') else {
        return false;
    };
    matches!(kind, "content" | "export")
        && id.len() == 36
        && Uuid::parse_str(id).is_ok_and(|id| id.get_version_num() == 4)
}
impl ReadGrants {
    fn mint_at(&self, uri: &str, now: Instant) -> Result<String, FileError> {
        if !allowed(uri) {
            return Err(invalid());
        }
        let mut grants = self.0.lock().map_err(|_| unavailable())?;
        grants.retain(|grant| grant.expires > now);
        let token = if let Some(grant) = grants.iter().find(|grant| grant.uri == uri) {
            grant.token.clone()
        } else {
            if grants.len() >= MAX_GRANTS {
                return Err(FileError(
                    axum::http::StatusCode::TOO_MANY_REQUESTS,
                    "read_link_limit",
                    "The workspace has too many active media links. Retry when older links expire.",
                ));
            }
            let token = Uuid::new_v4().to_string();
            grants.push(Grant {
                token: token.clone(),
                uri: uri.into(),
                expires: now + LIFETIME,
            });
            token
        };
        Ok(format!(
            "{uri}{}read_cap={token}",
            if uri.contains('?') { '&' } else { '?' }
        ))
    }
    pub(super) fn authorize(&self, uri: &str, now: Instant) -> Option<String> {
        let (original, token) = strip(uri)?;
        let grants = self.0.lock().ok()?;
        grants
            .iter()
            .any(|grant| {
                grant.expires > now && grant.uri == original && matches_secret(token, &grant.token)
            })
            .then(|| original.to_owned())
    }
}
/// Remove only a final, canonical capability. The original query stays exact.
pub(super) fn strip(uri: &str) -> Option<(&str, &str)> {
    let (uri, token) = uri
        .rsplit_once("&read_cap=")
        .or_else(|| uri.rsplit_once("?read_cap="))?;
    if token.len() != 36 || !Uuid::parse_str(token).is_ok_and(|id| id.get_version_num() == 4) {
        return None;
    }
    Some((uri, token))
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct LinkRequest {
    uri: String,
}
#[derive(Serialize)]
pub(crate) struct Link {
    url: String,
    expires_in: u64,
}
pub(crate) async fn create(
    State(state): State<AppState>,
    request: Request,
) -> Result<Json<Link>, FileError> {
    let value = crate::lab::request_json(request)
        .await
        .map_err(|_| invalid())?;
    let request: LinkRequest = serde_json::from_value(value).map_err(|_| invalid())?;
    Ok(Json(Link {
        url: state.read_grants.mint_at(&request.uri, Instant::now())?,
        expires_in: LIFETIME.as_secs(),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scoped_links_cannot_authorize_another_path_query_or_expired_session() {
        let grants = ReadGrants::default();
        let now = Instant::now();
        let uri = "/api/workspace/media?store=source&path=sample.mp4&offset=0";
        let link = grants.mint_at(uri, now).unwrap();
        assert_eq!(grants.authorize(&link, now).as_deref(), Some(uri));
        assert_eq!(grants.mint_at(uri, now).unwrap(), link);
        assert!(
            grants
                .authorize(&link.replace("sample.mp4", "other.mp4"), now)
                .is_none()
        );
        assert!(grants.authorize(&link, now + LIFETIME).is_none());
        assert!(ReadGrants::default().authorize(&link, now).is_none());
        for uri in [
            "https://outside.invalid/api/workspace/media",
            "//outside.invalid/api/workspace/media",
            "/api/health",
            "/api/workspace/files",
            "/api/workspace/media?read_cap=other",
        ] {
            assert!(grants.mint_at(uri, now).is_err());
        }
    }
    #[test]
    fn quotas_are_bounded_and_expired_links_release_capacity() {
        let grants = ReadGrants::default();
        let now = Instant::now();
        for i in 0..MAX_GRANTS {
            grants
                .mint_at(&format!("/api/workspace/media?path={i}"), now)
                .unwrap();
        }
        assert!(
            grants
                .mint_at("/api/workspace/media?path=next", now)
                .is_err()
        );
        assert!(
            grants
                .mint_at("/api/workspace/media?path=next", now + LIFETIME)
                .is_ok()
        );
        let id = Uuid::new_v4();
        for kind in ["content", "export"] {
            assert!(allowed(&format!("/api/lab/results/{id}/{kind}")));
        }
        assert!(!allowed(&format!("/api/lab/results/{id}/other")));
    }
}
