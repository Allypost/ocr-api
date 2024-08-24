use std::sync::Arc;

use futures::{stream::FuturesUnordered, StreamExt};
use once_cell::sync::OnceCell;
use tracing::{debug, info};

use super::Endpoint;
use crate::config::Config;

static ENDPOINT_WATCHER: OnceCell<Arc<EndpointWatcher>> = OnceCell::new();

#[derive(Debug)]
pub struct EndpointWatcher {
    pub endpoints: Vec<Endpoint>,
}

impl EndpointWatcher {
    pub async fn check_and_update_endpoints(&self) {
        debug!("Checking TCP connections to endpoints");

        self.endpoints
            .iter()
            .map(|endpoint| async move {
                endpoint.check_and_update().await;
            })
            .collect::<FuturesUnordered<_>>()
            .collect::<Vec<_>>()
            .await;
    }
}

impl EndpointWatcher {
    pub const fn endpoints(&self) -> &Vec<Endpoint> {
        &self.endpoints
    }

    pub fn endpoints_supporting_handler(&self, handler: &str) -> Vec<&Endpoint> {
        self.endpoints()
            .iter()
            .filter(|endpoint| {
                if !endpoint.is_up() {
                    return false;
                }

                if !endpoint.supports_handler(handler) {
                    return false;
                }

                true
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
            endpoints: urls.into_iter().map(std::convert::Into::into).collect(),
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
