mod middleware;
mod routes;

use std::time::Duration;

use axum::{
    extract::DefaultBodyLimit,
    http::{HeaderValue, Request, Response},
    routing::{delete, get},
    Router,
};
use reqwest::header;
use tower::ServiceBuilder;
use tower_http::{
    catch_panic::CatchPanicLayer,
    request_id::{MakeRequestId, PropagateRequestIdLayer, RequestId, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::{debug, field, info, Span};

use crate::helpers::id::time_thread_id;

pub const MAX_BODY_SIZE: usize = {
    const KB: usize = 1024;
    const MB: usize = 1024 * KB;

    512 * MB
};

pub fn create_router() -> Router {
    Router::new()
        .route("/", get(routes::get_root))
        .route("/endpoints", get(routes::get_endpoints))
        .route(
            "/endpoints/supporting/:handler",
            get(routes::get_endpoints_supporting_handler),
        )
        .route(
            "/ocr/:handler",
            get(routes::get_endpoint_supporting_handler).post(routes::any_endpoint_proxy_handler),
        )
        .nest(
            "/admin",
            Router::new()
                .route(
                    "/endpoints",
                    get(routes::get_endpoints)
                        .post(routes::any_add_endpoint)
                        .put(routes::any_add_endpoint),
                )
                .route("/endpoints/:id", delete(routes::delete_remove_endpoint))
                .layer(axum::middleware::from_fn(middleware::auth::require_auth))
                .layer(axum::middleware::from_fn(
                    middleware::auth::parse_auth_header,
                )),
        )
        .layer(CatchPanicLayer::new())
        .layer(DefaultBodyLimit::max(MAX_BODY_SIZE))
        .layer(
            ServiceBuilder::new()
                .layer(SetRequestIdLayer::x_request_id(AppMakeRequestId))
                .layer(
                    TraceLayer::new_for_http()
                        .make_span_with(|request: &Request<_>| {
                            let m = request.method();
                            let p = request.uri().path();
                            let id = request
                                .extensions()
                                .get::<RequestId>()
                                .and_then(|id| id.header_value().to_str().ok())
                                .unwrap_or("-");
                            let dur = field::Empty;

                            tracing::info_span!("request", %id, %m, ?p, dur)
                        })
                        .on_request(|request: &Request<_>, _span: &Span| {
                            let headers = request.headers();
                            info!(
                                target: "request",
                                "START \"{method} {uri} {http_type:?}\" {user_agent:?} {ip:?}",
                                http_type = request.version(),
                                method = request.method(),
                                uri = request.uri(),
                                user_agent = headers
                                    .get(header::USER_AGENT)
                                    .map_or("-", |x| x.to_str().unwrap_or("-")),
                                ip = headers
                                    .get("x-forwarded-for")
                                    .map_or("-", |x| x.to_str().unwrap_or("-")),
                            );
                        })
                        .on_response(|response: &Response<_>, latency, span: &Span| {
                            span.record("dur", field::debug(latency));
                            debug!(
                                target: "request",
                                "END {status}",
                                status = response.status().as_u16(),
                            );
                        })
                        .on_body_chunk(())
                        .on_eos(|_trailers: Option<&_>, stream_duration, span: &Span| {
                            span.record("dur", field::debug(stream_duration));
                            debug!(target: "request", "ERR: stream closed unexpectedly",);
                        })
                        .on_failure(|error, latency, span: &Span| {
                            span.record("dur", field::debug(latency));
                            debug!(
                                target: "request",
                                err = ?error,
                                "ERR: something went wrong",
                            );
                        }),
                )
                .layer(TimeoutLayer::new(Duration::from_secs(300)))
                .layer(PropagateRequestIdLayer::x_request_id()),
        )
}

#[derive(Clone)]
struct AppMakeRequestId;
impl MakeRequestId for AppMakeRequestId {
    fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
        let mut id = time_thread_id();
        id.make_ascii_lowercase();
        let val = HeaderValue::from_str(&id).ok()?;

        Some(RequestId::new(val))
    }
}
