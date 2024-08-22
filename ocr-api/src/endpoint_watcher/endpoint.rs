use std::string::ToString;

use chrono::{prelude::*, DateTime};
use serde::{Deserialize, Serialize};
use tokio::net::TcpStream;
use tracing::{debug, trace};
use url::Url;

use crate::helpers::id::time_rand_id;

#[derive(Debug, Clone, Serialize)]
pub struct Endpoint {
    pub id: String,
    pub url: Url,
    pub status: EndpointStatus,
    info: Option<EndpointInfo>,
}

impl Endpoint {
    pub fn supports_handler(&self, handler: &str) -> bool {
        self.info
            .as_ref()
            .map_or(false, |info| info.supports_handler(handler))
    }

    pub fn handler_url(&self, handler: &str) -> Option<Url> {
        let info = self.info.as_ref()?;
        let mut handler_path = info.handler_path(handler);
        if handler_path.starts_with('/') {
            handler_path = handler_path[1..].to_string();
        }

        self.url.join(&handler_path).ok()
    }

    pub const fn is_up(&self) -> bool {
        self.status.is_up()
    }
}

impl Endpoint {
    #[tracing::instrument(skip_all, fields(url = %self.url.as_str()))]
    pub async fn check_and_update(&mut self) {
        trace!("Checking and updating endpoint");
        let was_up = self.status.is_up();
        let now_is_up = self.check_tcp().await.is_up();

        if !was_up && now_is_up {
            trace!("Endpoint is freshly up, updating metadata");
            self.update_metadata().await;
        }
    }

    #[tracing::instrument(skip_all, fields(url = %self.url.as_str()))]
    pub async fn check_tcp(&mut self) -> &mut Self {
        trace!("Checking TCP connection to endpoint");

        let addr = {
            let host = self.url.host_str().expect("URL must have a host");
            let port = self.url.port().unwrap_or_else(|| match self.url.scheme() {
                "http" => 80,
                "https" => 443,
                _ => panic!("Unsupported scheme"),
            });
            format!("{}:{}", host, port)
        };

        trace!(addr = ?addr, "Connecting to endpoint");

        let result = TcpStream::connect(addr).await;

        match result {
            Ok(_) => {
                trace!("TCP connection successful");
                self.status = EndpointStatus::up();
            }
            Err(e) => {
                trace!(error = ?e, "TCP connection failed");
                self.status = EndpointStatus::down(e);
            }
        }

        self
    }

    #[tracing::instrument(skip_all, fields(url = %self.url.as_str()))]
    pub async fn update_metadata(&mut self) -> &mut Self {
        if !self.status.is_up() {
            return self;
        }

        debug!("Updating endpoint metadata");

        let client = reqwest::Client::new();
        let response = client.get(self.url.as_str()).send().await;
        trace!(response = ?response, "Got response from endpoint");
        let response = match response {
            Ok(resp) => resp,
            Err(e) => {
                self.status =
                    EndpointStatus::down(format!("Couldn't get endpoint base info: {:?}", e));

                return self;
            }
        };

        let response = response.json::<EndpointInfo>().await;
        let response = match response {
            Ok(resp) => resp,
            Err(e) => {
                self.status =
                    EndpointStatus::down(format!("Couldn't parse endpoint base info: {:?}", e));

                return self;
            }
        };
        trace!(response = ?response, "Parsed response from endpoint");

        self.info = Some(response);

        self
    }
}

impl Endpoint {
    pub fn new(url: Url) -> Self {
        Self {
            id: time_rand_id(),
            url,
            status: EndpointStatus::unknown(),
            info: None,
        }
    }
}

impl From<Url> for Endpoint {
    fn from(url: Url) -> Self {
        Self::new(url)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointInfo {
    #[serde(alias = "handlers")]
    available_handlers: Vec<String>,
    handler_template: String,
}
impl EndpointInfo {
    pub fn supports_handler(&self, handler: &str) -> bool {
        self.available_handlers.contains(&handler.to_string())
    }

    pub fn handler_path(&self, handler: &str) -> String {
        self.handler_template.replace("{handler_name}", handler)
    }
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "snake_case", tag = "state")]
pub enum EndpointStatus {
    Up {
        checked_at: DateTime<Utc>,
    },
    Down {
        checked_at: DateTime<Utc>,
        error: String,
    },
    #[default]
    Unknown,
}
#[allow(dead_code)]
impl EndpointStatus {
    pub fn up() -> Self {
        Self::Up {
            checked_at: Utc::now(),
        }
    }

    pub const fn is_up(&self) -> bool {
        matches!(self, Self::Up { .. })
    }

    pub fn down<T>(error: T) -> Self
    where
        T: std::fmt::Display,
    {
        Self::Down {
            checked_at: Utc::now(),
            error: error.to_string(),
        }
    }

    pub const fn is_down(&self) -> bool {
        matches!(self, Self::Down { .. })
    }

    pub const fn unknown() -> Self {
        Self::Unknown
    }

    pub const fn is_unknown(&self) -> bool {
        matches!(self, Self::Unknown)
    }
}
