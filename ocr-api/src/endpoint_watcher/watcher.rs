use std::sync::Arc;

use futures::{stream::FuturesUnordered, StreamExt};
use once_cell::sync::OnceCell;
use tokio::sync::RwLock;
use tracing::{debug, info};

use super::Endpoint;
use crate::config::Config;

static ENDPOINT_WATCHER: OnceCell<Arc<EndpointWatcher>> = OnceCell::new();

#[derive(Debug)]
pub struct EndpointWatcher {
    pub endpoints: Arc<Vec<RwLock<Endpoint>>>,
}

impl EndpointWatcher {
    pub async fn check_endpoints_tcp(&self) {
        debug!("Checking TCP connections to endpoints");

        self.endpoints
            .iter()
            .map(|endpoint| async move {
                endpoint.write().await.check_and_update().await;
            })
            .collect::<FuturesUnordered<_>>()
            .collect::<Vec<_>>()
            .await;
    }
}

impl EndpointWatcher {
    pub async fn endpoints(&self) -> Vec<Endpoint> {
        self.endpoints
            .iter()
            .map(|e| async move { e.read().await.clone() })
            .collect::<FuturesUnordered<_>>()
            .collect::<Vec<_>>()
            .await
    }

    pub async fn endpoints_supporting_handler(&self, handler: &str) -> Vec<Endpoint> {
        self.endpoints()
            .await
            .into_iter()
            .filter_map(|endpoint| {
                if !endpoint.is_up() {
                    return None;
                }

                if !endpoint.supports_handler(handler) {
                    return None;
                }

                Some(endpoint)
            })
            .collect()
    }
}

impl EndpointWatcher {
    fn from_urls<U, I>(urls: U) -> Self
    where
        U: IntoIterator<Item = I>,
        I: Into<Endpoint>,
    {
        Self {
            endpoints: Arc::new(
                urls.into_iter()
                    .map(|url| RwLock::new(url.into()))
                    .collect(),
            ),
        }
    }

    pub fn global() -> &'static Self {
        ENDPOINT_WATCHER.get_or_init(|| {
            info!("Creating global EndpointWatcher");

            let endpoints = Config::global().base_api_urls.clone();

            let watcher = Arc::new(Self::from_urls(endpoints));

            tokio::spawn({
                debug!("Starting endpoint watcher task");
                let watcher = watcher.clone();
                async move {
                    loop {
                        watcher.check_endpoints_tcp().await;
                        tokio::time::sleep(Config::global().api_check_interval.into()).await;
                    }
                }
            });

            watcher
        })
    }
}

impl<T> From<Vec<T>> for EndpointWatcher
where
    T: Into<Endpoint>,
{
    fn from(urls: Vec<T>) -> Self {
        Self {
            endpoints: Arc::new(
                urls.into_iter()
                    .map(|url| RwLock::new(url.into()))
                    .collect(),
            ),
        }
    }
}
