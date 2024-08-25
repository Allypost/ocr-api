use std::sync::Arc;

use futures::{stream::FuturesUnordered, StreamExt};
use once_cell::sync::OnceCell;
use tokio::sync::RwLock;
use tracing::{debug, info};

use super::{endpoint::EndpointId, Endpoint};
use crate::config::Config;

static ENDPOINT_WATCHER: OnceCell<Arc<EndpointWatcher>> = OnceCell::new();

#[derive(Debug)]
pub struct EndpointWatcher {
    endpoints: Arc<RwLock<Vec<Endpoint>>>,
}

impl EndpointWatcher {
    #[allow(clippy::significant_drop_tightening)]
    pub async fn check_and_update_endpoints(&self) {
        debug!("Checking TCP connections to endpoints");

        let endpoints = self.endpoints.read().await;
        let futs = endpoints.iter().map(|endpoint| async move {
            endpoint.check_and_update().await;
        });

        futs.collect::<FuturesUnordered<_>>()
            .collect::<Vec<_>>()
            .await;
    }
}

#[allow(dead_code)]
impl EndpointWatcher {
    pub async fn endpoints(&self) -> Vec<Endpoint> {
        self.endpoints.read().await.clone()
    }

    pub async fn endpoints_supporting_handler(&self, handler: &str) -> Vec<Endpoint> {
        self.endpoints()
            .await
            .into_iter()
            .filter(|endpoint| endpoint.supports_handler(handler))
            .collect()
    }

    pub async fn add_endpoint<T>(&self, endpoint: T) -> bool
    where
        T: Into<Endpoint> + Send + Sync,
    {
        let endpoint = endpoint.into();

        let exists = self
            .endpoints
            .read()
            .await
            .iter()
            .any(|e| e.url == endpoint.url);

        if exists {
            return false;
        }

        endpoint.check_and_update().await;
        self.endpoints.write().await.push(endpoint);

        true
    }

    pub async fn remove_endpoint<T>(&self, endpoint_id: T)
    where
        T: Into<EndpointId> + Send + Sync,
    {
        let id_to_delete = endpoint_id.into();
        let mut endpoints = self.endpoints.write().await;
        endpoints.retain(|endpoint| endpoint.id != id_to_delete);
    }
}

impl EndpointWatcher {
    fn from_urls<U, I>(urls: U) -> Self
    where
        U: IntoIterator<Item = I>,
        I: Into<Endpoint>,
    {
        Self {
            endpoints: Arc::new(RwLock::new(
                urls.into_iter().map(std::convert::Into::into).collect(),
            )),
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
                        watcher.check_and_update_endpoints().await;
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
        Self::from_urls(urls)
    }
}
