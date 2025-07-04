use serde_json::Value;

use crate::pocketoption::error::PocketResult;

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
