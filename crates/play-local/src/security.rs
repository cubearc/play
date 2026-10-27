use axum::{
    Json,
    extract::{OriginalUri, Request, State},
    http::{HeaderValue, Method, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use subtle::ConstantTimeEq;

use crate::AppState;
mod read_grants;
pub(crate) use read_grants::{ReadGrants, create as create_read_link};

#[derive(Serialize)]
struct ErrorBody {
    code: &'static str,
    message: &'static str,
}

pub fn error(status: StatusCode, code: &'static str, message: &'static str) -> Response {
    (status, Json(ErrorBody { code, message })).into_response()
}

pub fn matches_secret(candidate: &str, secret: &str) -> bool {
    candidate.as_bytes().ct_eq(secret.as_bytes()).into()
}

/// Host and browser boundaries apply to static files, readiness and API routes.
pub async fn boundary(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let headers = request.headers();
    let host_valid = headers.get_all(header::HOST).iter().count() == 1
        && headers.get(header::HOST).and_then(|v| v.to_str().ok())
            == Some(state.config.authority());
    let origin = headers.get(header::ORIGIN);
    let origin_valid = headers.get_all(header::ORIGIN).iter().count() <= 1
        && origin.is_none_or(|value| value.to_str().ok() == Some(&state.config.origin));
    let needs_origin = !matches!(*request.method(), Method::GET | Method::HEAD);
    let cross_site = headers
        .get("sec-fetch-site")
        .is_some_and(|v| v == "cross-site");
    let forwarded = ["forwarded", "x-forwarded-host", "x-forwarded-proto"]
        .iter()
        .any(|name| headers.contains_key(*name));
    let mut response = if request
        .uri()
        .path_and_query()
        .is_some_and(|value| value.as_str().len() > 16384)
    {
        error(
            StatusCode::URI_TOO_LONG,
            "request_limit",
            "The local request exceeds its path/query limit",
        )
    } else if !host_valid
        || !origin_valid
        || (needs_origin && origin.is_none())
        || cross_site
        || forwarded
    {
        error(
            StatusCode::FORBIDDEN,
            "untrusted_request",
            "Use the local Cube Play session URL",
        )
    } else {
        next.run(request).await
    };
    let policy = format!(
        "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; media-src 'self'; font-src 'self'; connect-src 'self' ws://{}; frame-ancestors 'none'; base-uri 'none'; form-action 'none'; object-src 'none'",
        state.config.authority()
    );
    response.headers_mut().insert(
        "content-security-policy",
        HeaderValue::from_str(&policy).expect("Validated loopback origin forms a header"),
    );
    for (name, value) in [
        ("x-content-type-options", "nosniff"),
        ("referrer-policy", "no-referrer"),
        ("cache-control", "no-store"),
        (
            "permissions-policy",
            "camera=(), microphone=(), geolocation=()",
        ),
    ] {
        response
            .headers_mut()
            .insert(name, HeaderValue::from_static(value));
    }
    response
}

pub async fn require_session(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Response {
    let authorized = bearer(&request).is_some_and(|value| matches_secret(value, &state.secret));
    let protocols = request
        .headers()
        .get(header::SEC_WEBSOCKET_PROTOCOL)
        .and_then(|v| v.to_str().ok());
    let websocket = !request.headers().contains_key(header::AUTHORIZATION)
        && request
            .headers()
            .get_all(header::SEC_WEBSOCKET_PROTOCOL)
            .iter()
            .count()
            == 1
        && request.uri().path() == "/terminal/ws"
        && protocols.is_some_and(|value| {
            if value.len() > 512 {
                return false;
            }
            let values = value.split(',').map(str::trim).collect::<Vec<_>>();
            values.len() == 2
                && values.contains(&"cube-play.v1")
                && values.iter().any(|v| {
                    v.strip_prefix("cube-play.auth.")
                        .is_some_and(|secret| matches_secret(secret, &state.secret))
                })
        });
    let mut read = false;
    if !authorized
        && !websocket
        && !request.headers().contains_key(header::AUTHORIZATION)
        && matches!(*request.method(), Method::GET | Method::HEAD)
    {
        let uri = request
            .extensions()
            .get::<OriginalUri>()
            .map(|v| &v.0)
            .unwrap_or_else(|| request.uri());
        if state
            .read_grants
            .authorize(&uri.to_string(), std::time::Instant::now())
            .is_some()
            && let Some((clean, _)) = read_grants::strip(&request.uri().to_string())
            && let Ok(uri) = clean.parse()
        {
            *request.uri_mut() = uri;
            read = true;
        }
    }
    if !authorized && !websocket && !read {
        return error(
            StatusCode::UNAUTHORIZED,
            "session_required",
            "Reopen the session URL printed by Cube Play",
        );
    }
    next.run(request).await
}

fn bearer(request: &Request) -> Option<&str> {
    if request
        .headers()
        .get_all(header::AUTHORIZATION)
        .iter()
        .count()
        != 1
    {
        return None;
    }
    request
        .headers()
        .get(header::AUTHORIZATION)?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")
}
/// Validate the origin-scoped browser capability and retire the old cookie.
pub async fn open_session(State(state): State<AppState>, request: Request) -> Response {
    let authorization = bearer(&request);
    if !authorization.is_some_and(|value| matches_secret(value, &state.secret)) {
        return error(
            StatusCode::UNAUTHORIZED,
            "invalid_session",
            "The local session link is missing or expired",
        );
    }
    let mut response = StatusCode::NO_CONTENT.into_response();
    let cookie = format!(
        "{}=; Max-Age=0; HttpOnly; SameSite=Strict; Path=/api",
        state.config.session_cookie(),
    );
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie).expect("UUID cookie is a valid header"),
    );
    response
}
