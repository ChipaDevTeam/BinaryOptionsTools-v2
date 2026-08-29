use std::time::Duration;

use binary_options_tools::config::Config;
use napi_derive::napi;
use url::Url;

use crate::error::{napi_err, BinaryErrorNode};

/// Plain-object client configuration.
///
/// Every field is optional; omitted fields keep the library default. Durations
/// are expressed in the unit named by the field so that no `Duration` type has
/// to cross the JS boundary.
#[napi(object)]
#[derive(Default)]
pub struct ClientConfig {
    /// Maximum number of reconnection loops before giving up.
    pub max_allowed_loops: Option<u32>,
    /// Delay between internal polling iterations, in milliseconds.
    pub sleep_interval_ms: Option<u32>,
    /// Delay before attempting to reconnect, in seconds.
    pub reconnect_time_secs: Option<u32>,
    /// How long the initial handshake may take, in seconds.
    pub connection_initialization_timeout_secs: Option<u32>,
    /// Default request timeout, in seconds.
    pub timeout_secs: Option<u32>,
    /// Explicit list of WebSocket endpoints to use instead of the built-in ones.
    pub urls: Option<Vec<String>>,
    /// Proxy URL used for the WebSocket connection.
    pub proxy: Option<String>,
    /// `User-Agent` header sent during the handshake.
    pub user_agent: Option<String>,
    /// `Origin` header sent during the handshake.
    pub origin: Option<String>,
    /// `Sec-WebSocket-Extensions` header sent during the handshake.
    pub sec_websocket_extensions: Option<String>,
    /// TLS cipher suites to advertise.
    pub tls_cipher_suites: Option<Vec<String>>,
    /// ALPN protocols to advertise.
    pub tls_alpn: Option<Vec<String>>,
}

impl TryFrom<ClientConfig> for Config {
    type Error = napi::Error;

    fn try_from(value: ClientConfig) -> Result<Self, Self::Error> {
        let mut config = Config::default();

        if let Some(loops) = value.max_allowed_loops {
            config.max_allowed_loops = loops;
        }
        if let Some(interval) = value.sleep_interval_ms {
            config.sleep_interval = Duration::from_millis(interval as u64);
        }
        if let Some(secs) = value.reconnect_time_secs {
            config.reconnect_time = Duration::from_secs(secs as u64);
        }
        if let Some(secs) = value.connection_initialization_timeout_secs {
            config.connection_initialization_timeout = Duration::from_secs(secs as u64);
        }
        if let Some(secs) = value.timeout_secs {
            config.timeout = Duration::from_secs(secs as u64);
        }
        if let Some(urls) = value.urls {
            let mut parsed = Vec::with_capacity(urls.len());
            let mut invalid = Vec::new();
            for url in urls {
                match Url::parse(&url) {
                    Ok(url) => parsed.push(url),
                    Err(_) => invalid.push(url),
                }
            }
            if !invalid.is_empty() {
                return Err(napi_err(BinaryErrorNode::InvalidParameter(format!(
                    "Invalid URLs provided: {}",
                    invalid.join(", ")
                ))));
            }
            config.urls = parsed;
        }
        if value.proxy.is_some() {
            config.proxy = value.proxy;
        }
        if value.user_agent.is_some() {
            config.user_agent = value.user_agent;
        }
        if value.origin.is_some() {
            config.origin = value.origin;
        }
        if value.sec_websocket_extensions.is_some() {
            config.sec_websocket_extensions = value.sec_websocket_extensions;
        }
        if value.tls_cipher_suites.is_some() {
            config.tls_cipher_suites = value.tls_cipher_suites;
        }
        if value.tls_alpn.is_some() {
            config.tls_alpn = value.tls_alpn;
        }

        Ok(config)
    }
}
