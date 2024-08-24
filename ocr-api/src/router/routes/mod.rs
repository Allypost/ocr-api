use axum::{
    body::Body,
    extract::Path,
    http::{HeaderMap, HeaderValue},
    response::{IntoResponse, Response},
    Json,
};
use rand::prelude::*;
use reqwest::{Method, StatusCode};
use tracing::{debug, trace};

use crate::endpoint_watcher::EndpointWatcher;

pub async fn get_root() -> impl IntoResponse {
    Json("OCR API Gateway".to_string())
}

pub async fn get_endpoints() -> impl IntoResponse {
    let endpoints = &EndpointWatcher::global().endpoints;

    Json(endpoints)
}

pub async fn get_endpoints_supporting_handler(Path(handler): Path<String>) -> impl IntoResponse {
    let endpoints = EndpointWatcher::global().endpoints_supporting_handler(&handler);

    Json(endpoints)
}

pub async fn get_endpoint_supporting_handler(Path(handler): Path<String>) -> impl IntoResponse {
    let endpoints = EndpointWatcher::global().endpoints_supporting_handler(&handler);

    let endpoint = endpoints.choose(&mut rand::thread_rng());

    let endpoint = match endpoint {
        Some(endpoint) => endpoint,
        None => {
            return (
                StatusCode::NOT_FOUND,
                "No live endpoints found supporting that handler".to_string(),
            )
                .into_response()
        }
    };

    Json(endpoint).into_response()
}

#[tracing::instrument]
pub async fn any_endpoint_proxy_handler(
    Path(handler): Path<String>,
    method: Method,
    headers: HeaderMap,
    body: Body,
) -> impl IntoResponse {
    debug!(?handler, "Proxying request");

    let endpoints = EndpointWatcher::global().endpoints_supporting_handler(&handler);

    let endpoint = endpoints.choose(&mut rand::thread_rng());

    trace!(?endpoint, "Chose endpoint");

    let endpoint = match endpoint {
        Some(endpoint) => endpoint,
        None => {
            return (
                StatusCode::NOT_FOUND,
                "No live endpoints found supporting that handler".to_string(),
            )
                .into_response()
        }
    };

    let handler_url = match endpoint.handler_url(&handler) {
        Some(url) => url,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Endpoint info not available".to_string(),
            )
                .into_response()
        }
    };

    trace!(?handler_url, "Got handler url");

    trace!("Forwarding request to endpoint");
    let endpoint_response = {
        let client = reqwest::Client::new();

        let mut request_builder = client.request(method, handler_url);

        request_builder = request_builder.headers({
            let mut headers = headers.clone();

            headers.insert(
                "Host",
                HeaderValue::from_str(endpoint.url.host_str().unwrap_or_default())
                    .expect("Invalid host"),
            );

            headers
        });

        request_builder = request_builder.body(reqwest::Body::wrap_stream(body.into_data_stream()));

        request_builder.send().await
    };

    trace!(?endpoint_response, "Got response from endpoint");

    match endpoint_response {
        Ok(endpoint_response) => {
            trace!("Forwarding response from endpoint");
            let mut response_builder = Response::builder().status(endpoint_response.status());
            *response_builder
                .headers_mut()
                .expect("Failed to get headers") = endpoint_response.headers().clone();
            response_builder
                .body(Body::from_stream(endpoint_response.bytes_stream()))
                .expect("Failed to build response")
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to proxy request: {:?}", e),
        )
            .into_response(),
    }
}
