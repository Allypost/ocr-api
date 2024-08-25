use std::{string::ToString, sync::Arc};

use chrono::{prelude::*, DateTime};
use parking_lot::RwLock;
use serde::{ser::SerializeStruct, Deserialize, Serialize};
use tokio::net::TcpStream;
use tracing::{debug, trace};
use url::Url;

use crate::helpers::id::time_rand_id;

#[derive(Debug, Clone)]
pub struct Endpoint {
    pub id: EndpointId,
    pub url: Url,
    pub status: Arc<RwLock<EndpointStatus>>,
}

impl Endpoint {
    pub fn supports_handler(&self, handler: &str) -> bool {
        self.status
            .read()
            .info()
            .map_or(false, |info| info.supports_handler(handler))
    }

    pub fn handler_url(&self, handler: &str) -> Option<Url> {
        let mut handler_path = self.status.read().info()?.handler_path(handler);
        if handler_path.starts_with('/') {
            handler_path = handler_path[1..].to_string();
        }

        self.url.join(&handler_path).ok()
    }
}

impl Endpoint {
    #[tracing::instrument(skip_all, fields(url = %self.url.as_str()))]
    pub async fn check_and_update(&self) {
        trace!("Checking and updating endpoint");
        match self.check_connectivity().await {
            Ok(()) => {
                trace!("Endpoint is up, updating metadata");
                self.update_metadata().await;
            }
            Err(e) => {
                trace!(error = ?e, "Endpoint is down, updating metadata");
                {
                    let mut status = self.status.write();
                    *status = EndpointStatus::down(format!("Couldn't connect to endpoint: {}", e));
                }
            }
        }
    }

    #[tracing::instrument(skip_all)]
    async fn check_connectivity(&self) -> Result<(), std::io::Error> {
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
                return Ok(());
            }
            Err(e) => {
                trace!(error = ?e, "TCP connection failed");
                return Err(e);
            }
        }
    }

    async fn update_metadata(&self) -> &Self {
        let new_status = self.get_metadata().await;

        {
            let mut status = self.status.write();
            *status = new_status;
        }

        self
    }

    #[tracing::instrument(skip_all)]
    async fn get_metadata(&self) -> EndpointStatus {
        debug!("Getting endpoint metadata");

        let client = reqwest::Client::new();
        let response = client.get(self.url.as_str()).send().await;
        trace!(response = ?response, "Got response from endpoint");
        let response = match response {
            Ok(resp) => resp,
            Err(e) => {
                return EndpointStatus::down(format!("Couldn't get endpoint base info: {:?}", e));
            }
        };

        let response = response.json::<EndpointInfo>().await;
        let response = match response {
            Ok(resp) => resp,
            Err(e) => {
                return EndpointStatus::down(format!("Couldn't parse endpoint base info: {:?}", e));
            }
        };
        trace!(response = ?response, "Parsed response from endpoint");

        EndpointStatus::up(response)
    }
}

impl Endpoint {
    pub fn new(url: Url) -> Self {
        Self {
            id: EndpointId::default(),
            url,
            status: Arc::new(RwLock::new(EndpointStatus::unknown())),
        }
    }
}

impl From<Url> for Endpoint {
    fn from(url: Url) -> Self {
        Self::new(url)
    }
}

impl Serialize for Endpoint {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut state = serializer.serialize_struct("Endpoint", 3)?;
        state.serialize_field("id", &self.id)?;
        state.serialize_field("url", &self.url)?;
        state.serialize_field("status", &*self.status.read())?;
        state.end()
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
        info: EndpointInfo,
    },
    Down {
        checked_at: DateTime<Utc>,
        error: String,
    },
    #[default]
    Unknown,
}
impl EndpointStatus {
    pub const fn info(&self) -> Option<&EndpointInfo> {
        match self {
            Self::Up { info, .. } => Some(info),
            _ => None,
        }
    }
}
#[allow(dead_code)]
impl EndpointStatus {
    pub fn up<T>(info: T) -> Self
    where
        T: Into<EndpointInfo>,
    {
        Self::Up {
            checked_at: Utc::now(),
            info: info.into(),
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EndpointId(String);
impl EndpointId {
    pub fn time_random() -> Self {
        time_rand_id().into()
    }
}

impl Default for EndpointId {
    fn default() -> Self {
        Self::time_random()
    }
}

impl From<String> for EndpointId {
    fn from(id: String) -> Self {
        Self(id)
    }
}

impl From<&String> for EndpointId {
    fn from(id: &String) -> Self {
        Self(id.clone())
    }
}

impl From<&str> for EndpointId {
    fn from(id: &str) -> Self {
        Self(id.to_string())
    }
}

impl std::fmt::Display for EndpointId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for EndpointId {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}
