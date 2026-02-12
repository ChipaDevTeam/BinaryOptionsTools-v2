use binary_options_tools_core_pre::connector::{ConnectorError, ConnectorResult};
use binary_options_tools_core_pre::reimports::{
    connect_async_tls_with_config, generate_key, Connector, MaybeTlsStream, Request,
    WebSocketStream,
};
use chrono::{Duration, Utc};
use rand::Rng;
use std::time::Duration as StdDuration;

use crate::pocketoption::{
    error::{PocketError, PocketResult},
    ssid::Ssid,
};
use crate::utils::init_crypto_provider;
use serde_json::Value;
use tokio::net::TcpStream;

use url::Url;

const IP_PROVIDERS: &[&str] = &[
    "https://i.pn/json/",
    "https://ip.pn/json/",
    "https://ipv4.myip.coffee",
    "https://api.ipify.org?format=json",
    "https://httpbin.org/ip",
    "https://ifconfig.co/json",
    "https://ipapi.co/",
    "https://ipwho.is/",
];
const EARTH_RADIUS_KM: f64 = 6371.0;

pub fn get_index() -> PocketResult<u64> {
    let mut rng = rand::thread_rng();

    let rand = rng.gen_range(10..99);
    let time = (Utc::now() + Duration::hours(2)).timestamp();
    format!("{time}{rand}")
        .parse::<u64>()
        .map_err(|e| PocketError::General(e.to_string()))
}

pub async fn get_user_location(ip_address: &str) -> PocketResult<(f64, f64)> {
    let client = reqwest::Client::builder()
        .timeout(StdDuration::from_secs(2))
        .build()
        .map_err(|e| PocketError::General(format!("Failed to build HTTP client: {e}")))?;

    // Try providers that give geolocation data
    for url in IP_PROVIDERS {
        let target = if url.contains("ipapi.co") {
            format!("{}{}/json/", url, ip_address)
        } else if url.contains("ipwho.is") || url.contains("i.pn") || url.contains("ip.pn") {
            format!("{}{}", url, ip_address)
        } else {
            continue;
        };

        tracing::debug!(target: "PocketUtils", "Trying geo provider: {}", target);
        if let Ok(response) = client.get(&target).send().await {
            if let Ok(json) = response.json::<Value>().await {
                let lat = json["lat"].as_f64().or_else(|| json["latitude"].as_f64());
                let lon = json["lon"].as_f64().or_else(|| json["longitude"].as_f64());

                if let (Some(lat), Some(lon)) = (lat, lon) {
                    tracing::debug!(target: "PocketUtils", "Found location via {}: {}, {}", target, lat, lon);
                    return Ok((lat, lon));
                }
            }
        }
    }

    tracing::warn!(target: "PocketUtils", "All geo providers failed for IP {}. Using fallback location.", ip_address);
    // Default or fallback location (e.g. US Central) if all fail
    Ok((37.0902, -95.7129))
}

pub fn calculate_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    // Haversine formula to calculate distance between two coordinates
    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();

    let lat1 = lat1.to_radians();
    let lat2 = lat2.to_radians();

    let a = dlat.sin().powi(2) + lat1.cos() * lat2.cos() * dlon.sin().powi(2);
    let c = 2.0 * a.sqrt().asin();

    EARTH_RADIUS_KM * c
}

pub async fn get_public_ip() -> PocketResult<String> {
    let client = reqwest::Client::builder()
        .timeout(StdDuration::from_secs(2))
        .build()
        .map_err(|e| PocketError::General(format!("Failed to build HTTP client: {e}")))?;

    for url in IP_PROVIDERS {
        let target = url.to_string();
        tracing::debug!(target: "PocketUtils", "Trying IP provider: {}", target);
        match client.get(&target).send().await {
            Ok(response) => {
                if let Ok(json) = response.json::<Value>().await {
                    if let Some(ip) = json["ip"]
                        .as_str()
                        .or_else(|| json["query"].as_str())
                        .or_else(|| json["origin"].as_str())
                    {
                        tracing::debug!(target: "PocketUtils", "Found public IP via {}: {}", target, ip);
                        return Ok(ip.to_string());
                    }
                }
            }
            Err(e) => {
                tracing::debug!(target: "PocketUtils", "Provider {} failed: {}", target, e);
                continue;
            }
        }
    }

    Err(PocketError::General(
        "Failed to retrieve public IP from any provider".into(),
    ))
}

pub async fn try_connect(
    ssid: Ssid,
    url: String,
) -> ConnectorResult<WebSocketStream<MaybeTlsStream<TcpStream>>> {
    init_crypto_provider();
    let mut root_store = rustls::RootCertStore::empty();
    let certs = rustls_native_certs::load_native_certs().certs;
    if certs.is_empty() {
        return Err(ConnectorError::Custom(
            "Could not load any native certificates".to_string(),
        ));
    }
    for cert in certs {
        root_store.add(cert).ok();
    }
    let tls_config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    let connector = Connector::Rustls(std::sync::Arc::new(tls_config));

    let user_agent = ssid.user_agent();

    // Log public IP to help debug 41 rejections (which often happen due to IP mismatch)
    if let Ok(ip) = get_public_ip().await {
        let redacted_ip = if let Some(idx) = ip.rfind('.') {
            format!("{}.xxx", &ip[..idx])
        } else {
            "REDACTED".to_string()
        };
        tracing::info!(target: "PocketConnect", "Connecting from IP: {}", redacted_ip);
    }

    let t_url = Url::parse(&url).map_err(|e| ConnectorError::UrlParsing(e.to_string()))?;
    let host = t_url
        .host_str()
        .ok_or(ConnectorError::UrlParsing("Host not found".into()))?;

    tracing::info!(target: "PocketConnect", "Connecting to {} with UA: {} and Origin: https://pocketoption.com", host, user_agent);

    let request = Request::builder()
        .uri(t_url.to_string())
        .header("Host", host)
        .header("User-Agent", user_agent)
        .header("Origin", "https://pocketoption.com")
        .header("Upgrade", "websocket")
        .header("Connection", "upgrade")
        .header("Sec-Websocket-Key", generate_key())
        .header("Sec-Websocket-Version", "13")
        .body(())
        .map_err(|e| ConnectorError::HttpRequestBuild(e.to_string()))?;

    let (ws, _) = tokio::time::timeout(
        StdDuration::from_secs(15),
        connect_async_tls_with_config(request, None, false, Some(connector)),
    )
    .await
    .map_err(|_| ConnectorError::Timeout)?
    .map_err(|e| ConnectorError::Custom(e.to_string()))?;
    Ok(ws)
}

pub mod unix_timestamp {

    use chrono::{DateTime, Utc};

    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i64(date.timestamp())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;

        let timestamp = if let Some(i) = value.as_i64() {
            i
        } else if let Some(f) = value.as_f64() {
            f.trunc() as i64
        } else {
            return Err(serde::de::Error::custom(
                "Error parsing timestamp: expected number",
            ));
        };

        DateTime::from_timestamp(timestamp, 0).ok_or(serde::de::Error::custom(
            "Error parsing timestamp to DateTime",
        ))
    }
}
