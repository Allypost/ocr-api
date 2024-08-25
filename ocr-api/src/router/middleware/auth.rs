use axum::{
    extract::Request,
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use constant_time_eq::constant_time_eq;

use crate::config::Config;

pub const AUTH_HEADER: &str = "x-api-key";
pub const AUTH_COOKIE: &str = "api-key";

#[derive(Debug, Clone)]
pub struct AuthData;

pub async fn parse_auth_header(mut request: Request, next: Next) -> Result<Response, Response> {
    if request.extensions().get::<AuthData>().is_some() {
        return Ok(next.run(request).await);
    };

    let auth_value = None
        .or_else(|| {
            request
                .headers()
                .get(AUTH_HEADER)
                .and_then(|x| x.to_str().ok())
        })
        .or_else(|| {
            request
                .headers()
                .get(header::AUTHORIZATION)
                .and_then(|x| x.to_str().ok())
                .and_then(|h| {
                    let (t, val) = h.split_once(' ')?;

                    if t.to_lowercase() != "bearer" {
                        return None;
                    }

                    Some(val)
                })
        })
        .or_else(|| {
            request
                .headers()
                .get(header::COOKIE)
                .and_then(|x| x.to_str().ok())
                .and_then(|h| {
                    h.split("; ")
                        .filter_map(|x| x.split_once('='))
                        .find(|(k, _)| k.to_lowercase() == AUTH_COOKIE)
                        .map(|(_, v)| v)
                })
        });

    if let Some(auth_value) = auth_value {
        if !constant_time_eq(
            auth_value.as_bytes(),
            Config::global().auth.api_auth_key.as_bytes(),
        ) {
            return Err((StatusCode::UNAUTHORIZED, "Invalid credentials").into_response());
        }

        request.extensions_mut().insert(AuthData);
    }

    let response = next.run(request).await;

    Ok(response)
}

pub async fn require_auth(request: Request, next: Next) -> Result<Response, Response> {
    let auth_data = request.extensions().get::<AuthData>();

    if auth_data.is_none() {
        return Err((StatusCode::UNAUTHORIZED, "Not authorized").into_response());
    }

    let response = next.run(request).await;

    Ok(response)
}
