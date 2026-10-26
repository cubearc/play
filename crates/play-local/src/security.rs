use axum::{
    Json,
    extract::{Request, State},
    http::{HeaderValue, Method, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use subtle::ConstantTimeEq;

use crate::AppState;

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
    for (name, value) in [
        (
            "content-security-policy",
            "default-src 'none'; script-src 'self'; style-src 'self'; img-src 'self' data:; media-src 'self'; font-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'; object-src 'none'",
        ),
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
    request: Request,
    next: Next,
) -> Response {
    let cookies: Vec<_> = request
        .headers()
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|cookie| cookie.trim().split_once('='))
        .filter(|(name, _)| *name == state.config.session_cookie())
        .collect();
    if cookies.len() != 1 || !matches_secret(cookies[0].1, &state.secret) {
        return error(
            StatusCode::UNAUTHORIZED,
            "session_required",
            "Reopen the session URL printed by Cube Play",
        );
    }
    next.run(request).await
}

/// Exchange the URL-fragment capability for an HttpOnly, same-site session cookie.
pub async fn open_session(State(state): State<AppState>, request: Request) -> Response {
    let authorization = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    if !authorization.is_some_and(|value| matches_secret(value, &state.secret)) {
        return error(
            StatusCode::UNAUTHORIZED,
            "invalid_session",
            "The local session link is missing or expired",
        );
    }
    let mut response = StatusCode::NO_CONTENT.into_response();
    let cookie = format!(
        "{}={}; HttpOnly; SameSite=Strict; Path=/api",
        state.config.session_cookie(),
        state.secret
    );
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_str(&cookie).expect("UUID cookie is a valid header"),
    );
    response
}
