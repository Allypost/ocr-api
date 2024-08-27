use axum::{
    body::Body,
    extract::Path,
    http::{HeaderMap, HeaderValue},
    response::{IntoResponse, Response},
    Json,
};
use chrono::{prelude::*, DateTime};
use rand::prelude::*;
use reqwest::{Method, StatusCode};
use serde::{Deserialize, Serialize};
use tracing::{debug, trace};
use url::Url;

use crate::endpoint_watcher::{
    endpoint::{EndpointId, EndpointStatus},
    Endpoint, EndpointWatcher,
};

pub async fn get_root() -> impl IntoResponse {
    Json("OCR API Gateway".to_string())
}

#[derive(Debug, Serialize)]
pub struct EndpointPublic {
    pub id: EndpointId,
    pub status: EndpointStatusPublic,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum EndpointStatusPublic {
    Up {
        checked_at: DateTime<Utc>,
        available_handlers: Vec<String>,
    },
    Down {
        checked_at: DateTime<Utc>,
        error: String,
    },
    #[default]
    Unknown,
}

impl From<EndpointStatus> for EndpointStatusPublic {
    fn from(status: EndpointStatus) -> Self {
        match status {
            EndpointStatus::Up { checked_at, info } => Self::Up {
                checked_at,
                available_handlers: info.available_handlers,
            },
            EndpointStatus::Down { checked_at, error } => Self::Down { checked_at, error },
            EndpointStatus::Unknown => Self::Unknown,
        }
    }
}

impl From<Endpoint> for EndpointPublic {
    fn from(endpoint: Endpoint) -> Self {
        Self {
            id: endpoint.id,
            status: endpoint.status.read().clone().into(),
        }
    }
}
impl From<&Endpoint> for EndpointPublic {
    fn from(endpoint: &Endpoint) -> Self {
        Self {
            id: endpoint.id.clone(),
            status: endpoint.status.read().clone().into(),
        }
    }
}

pub async fn get_endpoints_public() -> impl IntoResponse {
    let endpoints = EndpointWatcher::global()
        .endpoints()
        .await
        .into_iter()
        .map(EndpointPublic::from)
        .collect::<Vec<_>>();

    Json(endpoints)
}

pub async fn get_endpoints_supporting_handler_public(
    Path(handler): Path<String>,
) -> impl IntoResponse {
    let endpoints = EndpointWatcher::global()
        .endpoints_supporting_handler(&handler)
        .await
        .into_iter()
        .map(EndpointPublic::from)
        .collect::<Vec<_>>();

    Json(endpoints)
}

pub async fn get_endpoint_supporting_handler_public(
    Path(handler): Path<String>,
) -> impl IntoResponse {
    let endpoints = EndpointWatcher::global()
        .endpoints_supporting_handler(&handler)
        .await;

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

    Json(EndpointPublic::from(endpoint)).into_response()
}

pub async fn get_endpoints() -> impl IntoResponse {
    let endpoints = EndpointWatcher::global().endpoints().await;

    Json(endpoints)
}

#[tracing::instrument]
pub async fn any_endpoint_proxy_handler(
    Path(handler): Path<String>,
    method: Method,
    headers: HeaderMap,
    body: Body,
) -> impl IntoResponse {
    debug!(?handler, "Proxying request");

    let endpoints = EndpointWatcher::global()
        .endpoints_supporting_handler(&handler)
        .await;

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

#[derive(Debug, Deserialize)]
pub struct PayloadAddEndpoint {
    url: Url,
}
pub async fn any_add_endpoint(
    axum::extract::Json(endpoint_payload): axum::extract::Json<PayloadAddEndpoint>,
) -> impl IntoResponse {
    let url = endpoint_payload.url.to_string();

    let added = EndpointWatcher::global()
        .add_endpoint(endpoint_payload.url)
        .await;

    if !added {
        return Json(serde_json::json!({
            "success": false,
            "message": "Endpoint already exists",
            "url": url,
        }));
    }

    Json(serde_json::json!({
        "success": true,
        "message": "Added endpoint",
        "url": url,
    }))
}

pub async fn delete_remove_endpoint(Path(id): Path<String>) -> impl IntoResponse {
    EndpointWatcher::global().remove_endpoint(&id).await;

    Json(serde_json::json!({
        "success": true,
        "message": "Removed endpoint",
        "id": id,
    }))
    .into_response()
}

pub async fn any_disable_endpoint(Path(id): Path<String>) -> impl IntoResponse {
    let endpoint = EndpointWatcher::global().endpoint(&id).await;

    match endpoint {
        Some(endpoint) => {
            endpoint.set_disabled(true);
        }
        None => {
            return Json(serde_json::json!({
                "success": false,
                "message": "Endpoint not found",
                "id": id,
            }))
            .into_response();
        }
    };

    Json(serde_json::json!({
        "success": true,
        "message": "Disabled endpoint",
        "id": id,
    }))
    .into_response()
}

pub async fn any_enable_endpoint(Path(id): Path<String>) -> impl IntoResponse {
    let endpoint = EndpointWatcher::global().endpoint(&id).await;

    match endpoint {
        Some(endpoint) => {
            endpoint.set_disabled(false);
        }
        None => {
            return Json(serde_json::json!({
                "success": false,
                "message": "Endpoint not found",
                "id": id,
            }))
            .into_response();
        }
    };

    Json(serde_json::json!({
        "success": true,
        "message": "Enabled endpoint",
        "id": id,
    }))
    .into_response()
}
