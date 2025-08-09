use binary_options_tools_core_pre::connector::{ConnectorError, ConnectorResult};
use binary_options_tools_core_pre::reimports::{
    Connector, MaybeTlsStream, Request, WebSocketStream, connect_async_tls_with_config,
    generate_key,
};
use chrono::{Duration, Utc};
use rand::{Rng, rng};

use crate::pocketoption::{
    error::{PocketError, PocketResult},
    ssid::Ssid,
};
use serde_json::Value;
use tokio::net::TcpStream;
use url::Url;

pub fn get_index() -> PocketResult<u64> {
    let mut rng = rng();

    let rand = rng.random_range(10..99);
    let time = (Utc::now() + Duration::hours(2)).timestamp();
    format!("{time}{rand}")
        .parse::<u64>()
        .map_err(|e| PocketError::General(e.to_string()))
}

pub async fn get_user_location(ip_address: &str) -> PocketResult<(f64, f64)> {
    let response = reqwest::get(format!("http://ip-api.com/json/{ip_address}")).await?;
    let json: Value = response.json().await?;

    let lat = json["lat"].as_f64().unwrap();
    let lon = json["lon"].as_f64().unwrap();

    Ok((lat, lon))
}

pub fn calculate_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    // Haversine formula to calculate distance between two coordinates
    const R: f64 = 6371.0; // Radius of Earth in kilometers

    let dlat = (lat2 - lat1).to_radians();
    let dlon = (lon2 - lon1).to_radians();

    let lat1 = lat1.to_radians();
    let lat2 = lat2.to_radians();

    let a = dlat.sin().powi(2) + lat1.cos() * lat2.cos() * dlon.sin().powi(2);
    let c = 2.0 * a.sqrt().asin();

    R * c
}

pub async fn get_public_ip() -> PocketResult<String> {
    let response = reqwest::get("https://api.ipify.org?format=json").await?;
    let json: serde_json::Value = response.json().await?;
    Ok(json["ip"].as_str().unwrap().to_string())
}

pub async fn try_connect(
    ssid: Ssid,
    url: String,
) -> ConnectorResult<WebSocketStream<MaybeTlsStream<TcpStream>>> {
    let tls_connector: native_tls::TlsConnector = native_tls::TlsConnector::builder()
        .build()
        .map_err(|e| ConnectorError::Tls(e.to_string()))?;

    let connector = Connector::NativeTls(tls_connector);

    let user_agent = ssid.user_agent();
    let t_url = Url::parse(&url).map_err(|e| ConnectorError::UrlParsing(e.to_string()))?;
    let host = t_url
        .host_str()
        .ok_or(ConnectorError::UrlParsing("Host not found".into()))?;
    let request = Request::builder()
        .uri(t_url.to_string())
        .header("Origin", "https://pocketoption.com")
        .header("Cache-Control", "no-cache")
        .header("User-Agent", user_agent)
        .header("Upgrade", "websocket")
        .header("Connection", "upgrade")
        .header("Sec-Websocket-Key", generate_key())
        .header("Sec-Websocket-Version", "13")
        .header("Host", host)
        .body(())
        .map_err(|e| ConnectorError::HttpRequestBuild(e.to_string()))?;

    let (ws, _) = connect_async_tls_with_config(request, None, false, Some(connector))
        .await
        .map_err(|e| ConnectorError::Custom(e.to_string()))?;
    Ok(ws)
}

pub mod float_time {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(date: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = date.timestamp_millis() as f64 / 1000.0;
        serializer.serialize_f64(s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = f64::deserialize(deserializer)?.to_string();
        let (secs, milis) = match s.split_once(".") {
            Some((seconds, miliseconds)) => {
                let secs: i64 = seconds
                    .parse::<i64>()
                    .map_err(|e| serde::de::Error::custom(e.to_string()))?;
                let mut pow = 0;
                if miliseconds.len() <= 9 {
                    pow = 9u32.saturating_sub(miliseconds.len() as u32);
                }
                let milis = miliseconds
                    .parse::<u32>()
                    .map_err(|e| serde::de::Error::custom(e.to_string()))?
                    * 10i32.pow(pow) as u32;
                (secs, milis)
            }
            None => {
                let secs: i64 = s
                    .parse::<i64>()
                    .map_err(|e| serde::de::Error::custom(e.to_string()))?;

                (secs, 0)
            }
        };
        DateTime::from_timestamp(secs, milis)
            .ok_or(serde::de::Error::custom("Error parsing ints to time"))
    }
}
