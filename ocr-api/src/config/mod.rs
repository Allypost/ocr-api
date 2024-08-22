use clap::Parser;
use once_cell::sync::Lazy;
use url::Url;

use crate::helpers::timeframe::Timeframe;

pub static CONFIG: Lazy<Config> = Lazy::new(Config::new);

#[derive(Debug, Clone, Parser)]
pub struct Config {
    /// The host to bind to.
    ///
    /// Should be an IP address or a hostname.
    /// eg. `127.0.0.1` or `localhost` to bind to the local machine only.
    #[clap(short = 'H', long, default_value = "0.0.0.0", env = "HOST")]
    pub host: String,

    /// The port to bind to.
    ///
    /// Should be a valid port number (1-65535).
    /// eg. `8080` to bind to port 8080.
    /// The special value `0` will let the OS choose a port (usually at random).
    #[clap(short = 'P', long, default_value = "8000", env = "PORT")]
    pub port: u16,

    /// The URLs of the OCR APIs to use.
    ///
    /// Can be any combination of repeating this flag with comma- or space-separated lists of URLs.
    /// eg. `http://localhost:8080,http://localhost:8081` to use two APIs.
    #[clap(short = 'u', long = "base-api-url", env = "BASE_API_URLS", value_parser = value_parser_parse_absolute_urls(), required = true)]
    pub base_api_urls: std::vec::Vec<Url>,

    /// How often to check whether the APIs are reachable.
    ///
    /// Can be expressed as a human readable duration.
    /// eg. `5s` to check every 5 seconds or `1 minute` to check every minute.
    #[clap(long, value_parser = Timeframe::parse_str, default_value = "5s", env = "API_CHECK_INTERVAL")]
    pub api_check_interval: Timeframe,
}

impl Config {
    #[must_use]
    pub fn global() -> &'static Self {
        &CONFIG
    }
}

impl Config {
    fn new() -> Self {
        Self::parse()
    }
}

fn parse_absolute_url(s: &str) -> Result<Url, String> {
    let mut parsed = match Url::parse(s) {
        Ok(parsed) => parsed,
        Err(e) => return Err(format!("URL must be absolute: {e}")),
    };

    if parsed.cannot_be_a_base() {
        return Err("URL must be absolute".to_string());
    }

    parsed.set_path("/");

    Ok(parsed)
}

fn value_parser_parse_absolute_urls() -> impl clap::builder::TypedValueParser {
    move |s: &str| {
        s.split([',', ' '])
            .filter_map(|x| {
                let x = x.trim();

                if x.is_empty() {
                    return None;
                }

                Some(x)
            })
            .map(|s| parse_absolute_url(s.trim()))
            .collect::<Result<Vec<_>, _>>()
    }
}
