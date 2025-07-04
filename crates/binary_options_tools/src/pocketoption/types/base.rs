use core::fmt;

use binary_options_tools_core::general::traits::RawMessage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChangeSymbol {
    pub asset: String,
    pub period: i64,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SubscribeSymbol(String);

impl ChangeSymbol {
    pub fn new(asset: String, period: i64) -> Self {
        Self { asset, period }
    }
}

#[derive(Debug, Clone)]
pub struct RawWebsocketMessage {
    value: String,
}

impl fmt::Display for RawWebsocketMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.value.fmt(f)
    }
}

impl Serialize for RawWebsocketMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.value)
    }
}

impl<'de> Deserialize<'de> for RawWebsocketMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        String::deserialize(deserializer).map(|s| Self { value: s })
    }
}

impl From<String> for RawWebsocketMessage {
    fn from(value: String) -> Self {
        Self { value }
    }
}

impl From<&str> for RawWebsocketMessage {
    fn from(value: &str) -> Self {
        Self {
            value: value.to_string(),
        }
    }
}

impl RawMessage for RawWebsocketMessage {}
