use config::Config;
use tokio::{net::TcpListener, signal};
use tracing::{debug, info, warn};

pub mod config;
mod endpoint_watcher;
pub mod helpers;
mod logger;
mod router;

#[tokio::main]
async fn main() {
    let dotenvy_result = dotenvy::dotenv();
    logger::init();

    match dotenvy_result {
        Ok(path) => {
            debug!(?path, "Loaded .env file");
        }
        Err(e) if e.not_found() => {
            warn!("No .env file found");
        }
        Err(e) => {
            eprintln!("Failed to load .env file: {:?}", e);
            std::process::exit(1);
        }
    }

    debug!(config = ?Config::global(), "Loaded configuration");

    // Reference the global endpoint watcher to start global init
    endpoint_watcher::EndpointWatcher::global();

    let app = router::create_router();

    let listener = {
        let bind_addr = format!(
            "{host}:{port}",
            host = Config::global().host,
            port = Config::global().port,
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

    info!("API auth key is {:?}", Config::global().auth.api_auth_key);

    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("Failed to start server!");
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
