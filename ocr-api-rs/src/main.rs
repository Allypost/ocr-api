mod helpers;
mod log;
mod ocr;

use std::{string::ToString, time::Duration};

use axum::{
    extract::{Multipart, Path},
    http::{header, HeaderValue, Request, Response, StatusCode},
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use helpers::temp_file::TempFile;
use serde::{Deserialize, Serialize};
use tokio::{io::AsyncWriteExt, net::TcpListener, signal};
use tower::ServiceBuilder;
use tower_http::{
    catch_panic::CatchPanicLayer,
    request_id::{MakeRequestId, PropagateRequestIdLayer, RequestId, SetRequestIdLayer},
    timeout::TimeoutLayer,
    trace::TraceLayer,
};
use tracing::{debug, field, info, trace, Span};

#[derive(Clone)]
struct AppMakeRequestId;
impl MakeRequestId for AppMakeRequestId {
    fn make_request_id<B>(&mut self, _request: &Request<B>) -> Option<RequestId> {
        let mut id = helpers::id::time_thread_id();
        id.make_ascii_lowercase();
        let val = HeaderValue::from_str(&id).ok()?;

        Some(RequestId::new(val))
    }
}

#[tokio::main]
async fn main() {
    log::init();

    let app = create_router();

    let listener = {
        let bind_addr = format!(
            "{host}:{port}",
            host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string()),
        );

        debug!(addr = bind_addr, "Binding to");

        TcpListener::bind(bind_addr)
            .await
            .expect("Failed to start listener!")
    };

    info!(
        "Server started on: http://{}/",
        listener.local_addr().expect("Failed to get local address!")
    );

    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("Failed to start server!");
}

fn create_router() -> Router {
    Router::new()
        .route("/", get(handler_root))
        .route("/ocr/:handler_name", post(handler_ocr_by_handler_name))
        .layer(CatchPanicLayer::new())
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
                .layer(TimeoutLayer::new(Duration::from_secs(60)))
                .layer(PropagateRequestIdLayer::x_request_id()),
        )
}

async fn handler_root() -> impl IntoResponse {
    Json(serde_json::json!({
        "available_handlers": ocr::HANDLERS.iter().map(|h| h.name()).collect::<Vec<_>>(),
        "handler_template": "/ocr/{handler_name}",
    }))
}

#[derive(Debug)]
#[allow(dead_code)]
struct UploadTempFile {
    file_name: Option<String>,
    content_type: Option<String>,
    temp_file: TempFile,
}
#[tracing::instrument(skip_all, fields(field = ?file_field_name))]
async fn upload_temp_file(
    multipart: &mut Multipart,
    file_field_name: &str,
) -> anyhow::Result<UploadTempFile> {
    debug!("Uploading file from field");
    while let Some(mut field) = multipart.next_field().await? {
        trace!(?field, "Processing field");
        let field_name = field.name().unwrap_or_default().to_string();
        trace!(?field_name, "Field name");

        if field_name != file_field_name {
            continue;
        }

        debug!("Found field");
        let content_type = field.content_type().map(ToString::to_string);
        trace!(?content_type, "Content type");

        let ext = content_type
            .as_ref()
            .and_then(mime2ext::mime2ext)
            .unwrap_or("bin");

        let mut temp_file = TempFile::with_prefix_and_extension("ocr-", ext).await?;

        trace!(?temp_file, "Created temp file");

        debug!("Writing field to temp file by chunks");
        while let Some(chunk) = field.chunk().await? {
            temp_file.file_mut().write_all(&chunk).await?;
        }
        debug!("Finished writing field to temp file");

        let file = UploadTempFile {
            file_name: field.file_name().map(ToString::to_string),
            content_type,
            temp_file,
        };

        debug!(?file, "Uploaded file");

        return Ok(file);
    }

    anyhow::bail!("Failed to find field: {}", file_field_name);
}

#[tracing::instrument(skip_all)]
async fn handler_ocr_by_handler_name(
    Path(handler_name): Path<String>,
    mut multipart: Multipart,
) -> impl IntoResponse {
    trace!("Handling OCR request");

    let resp = OcrResponseBase::new(&handler_name);

    let ocr_handler = ocr::HANDLERS
        .iter()
        .find(|h| h.name() == handler_name)
        .cloned();

    let ocr_handler = match ocr_handler {
        Some(handler) => handler,
        None => {
            let err = format!(
                "Unknown handler. Available handlers: {}",
                ocr::HANDLERS
                    .iter()
                    .map(|h| h.name())
                    .collect::<Vec<_>>()
                    .join(", ")
            );

            return (StatusCode::NOT_FOUND, Json(resp.error(&err))).into_response();
        }
    };

    trace!(?ocr_handler, "Found handler");

    let file = match upload_temp_file(&mut multipart, "file").await {
        Ok(file) => file,
        Err(e) => {
            return resp.error(&e).into_response();
        }
    };
    trace!(?file, "Uploaded file");

    let img_path = file.temp_file.path().to_path_buf();
    let img_mime_type = file.content_type.clone();

    debug!(?img_path, ?img_mime_type, "Start image OCR task");
    let ocr_task_result = {
        let cur_span = Span::current();

        tokio::task::spawn_blocking(move || {
            let _entered = cur_span.enter();
            ocr_handler.ocr(&img_path, img_mime_type.as_deref())
        })
        .await
    };
    debug!(success = ocr_task_result.is_ok(), "Image OCR task done");

    let ocr_result = match ocr_task_result {
        Ok(ocr_result) => ocr_result,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(resp.error(&e))).into_response();
        }
    };

    let ocr_result = match ocr_result {
        Ok(ocr_result) => ocr_result,
        Err(e) => {
            return resp.error(&e).into_response();
        }
    };

    resp.data(ocr_result.matches).into_response()
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum OcrResult {
    Error(String),
    Data(Vec<serde_json::Value>),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
struct OcrResponse {
    pub engine: String,
    #[serde(flatten)]
    pub result: OcrResult,
}
impl IntoResponse for OcrResponse {
    fn into_response(self) -> Response<axum::body::Body> {
        match self.result {
            OcrResult::Error(_) => (StatusCode::BAD_REQUEST, Json(self)).into_response(),
            OcrResult::Data(_) => (StatusCode::OK, Json(self)).into_response(),
        }
    }
}

#[derive(Debug)]
struct OcrResponseBase {
    engine: String,
}
impl OcrResponseBase {
    fn new<T>(engine: T) -> Self
    where
        T: Into<String>,
    {
        Self {
            engine: engine.into(),
        }
    }

    fn error<E>(&self, error: &E) -> OcrResponse
    where
        E: ToString,
    {
        OcrResponse {
            engine: self.engine.clone(),
            result: OcrResult::Error(error.to_string()),
        }
    }

    fn data<D, I>(&self, data: D) -> OcrResponse
    where
        D: IntoIterator<Item = I>,
        I: Into<serde_json::Value>,
    {
        OcrResponse {
            engine: self.engine.clone(),
            result: OcrResult::Data(data.into_iter().map(Into::into).collect()),
        }
    }
}

async fn shutdown_signal() {
    // Listen for a SIGINT (Ctrl+C) or SIGTERM signal
    let ctrl_c = signal::ctrl_c();

    #[cfg(unix)]
    let mut handler = signal::unix::signal(signal::unix::SignalKind::terminate())
        .expect("Failed to install signal handler");
    #[cfg(unix)]
    let terminate = { handler.recv() };

    #[cfg(not(unix))]
    let terminate = std::future::pending();

    #[allow(clippy::redundant_pub_crate)]
    {
        tokio::select! {
            _ = ctrl_c => {
                println!("Received Ctrl+C, shutting down");
            }
            _ = terminate => {
                println!("Received SIGTERM, shutting down");
            }
        }
    }
}
