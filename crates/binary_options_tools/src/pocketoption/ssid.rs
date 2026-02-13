use core::fmt;
use std::collections::HashMap;

use binary_options_tools_core_pre::error::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::regions::Regions;

#[derive(Serialize, Deserialize, Clone)]
pub struct SessionData {
    pub session_id: String,
    pub ip_address: String,
    pub user_agent: String,
    pub last_activity: u64,
}

impl fmt::Debug for SessionData {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SessionData")
            .field("session_id", &"REDACTED")
            .field("ip_address", &"REDACTED") // Consider partial redaction
            .field("user_agent", &self.user_agent)
            .field("last_activity", &self.last_activity)
            .finish()
    }
}

fn deserialize_uid<'de, D>(deserializer: D) -> Result<u32, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let v: Value = Deserialize::deserialize(deserializer)?;
    match v {
        Value::Number(n) => n
            .as_u64()
            .map(|x| x as u32)
            .ok_or_else(|| serde::de::Error::custom("Invalid number for uid")),
        Value::String(s) => s
            .parse::<u32>()
            .map_err(|_| serde::de::Error::custom("Invalid string for uid")),
        _ => Err(serde::de::Error::custom("Invalid type for uid")),
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Demo {
    #[serde(alias = "sessionToken")]
    pub session: String,
    #[serde(default)]
    pub is_demo: u32,
    #[serde(deserialize_with = "deserialize_uid")]
    pub uid: u32,
    #[serde(default)]
    pub platform: u32,
    #[serde(alias = "currentUrl")]
    pub current_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_fast_history: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_optimized: Option<bool>,
    #[serde(skip)]
    pub raw: String,
    #[serde(skip)]
    pub json_raw: String,
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, Value>,
}

impl fmt::Debug for Demo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Demo")
            .field("session", &"REDACTED")
            .field("is_demo", &self.is_demo)
            .field("uid", &self.uid)
            .field("platform", &self.platform)
            .field("current_url", &self.current_url)
            .field("is_fast_history", &self.is_fast_history)
            .field("is_optimized", &self.is_optimized)
            .field("extra", &self.extra)
            .finish()
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Real {
    pub session: SessionData,
    pub session_raw: String,
    pub is_demo: u32,
    pub uid: u32,
    pub platform: u32,
    pub raw: String,
    pub json_raw: String,
    pub is_fast_history: Option<bool>,
    pub is_optimized: Option<bool>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}

impl fmt::Debug for Real {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Real")
            .field("session", &self.session)
            .field("session_raw", &"REDACTED")
            .field("is_demo", &self.is_demo)
            .field("uid", &self.uid)
            .field("platform", &self.platform)
            .field("raw", &"REDACTED")
            .field("is_fast_history", &self.is_fast_history)
            .field("is_optimized", &self.is_optimized)
            .field("extra", &self.extra)
            .finish()
    }
}

#[derive(Serialize, Clone)]
#[serde(untagged)]
pub enum Ssid {
    Demo(Demo),
    Real(Real),
}

impl fmt::Debug for Ssid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Demo(d) => f.debug_tuple("Demo").field(d).finish(),
            Self::Real(r) => f.debug_tuple("Real").field(r).finish(),
        }
    }
}

impl Ssid {
    pub fn parse(data: impl ToString) -> CoreResult<Self> {
        let data_str = data.to_string();
        let trimmed = data_str.trim();

        // Handle case where SSID is double-encoded or passed as a JSON string
        // We try this first because "invalid type: string" error suggests it's being parsed as a string
        if let Ok(unquoted) = serde_json::from_str::<String>(trimmed) {
            return Self::parse(unquoted);
        }

        // Handle raw quotes that might be invalid JSON string (e.g. "42["auth",...]")
        if trimmed.starts_with('"') && trimmed.ends_with('"') && trimmed.len() >= 2 {
            let unquoted = &trimmed[1..trimmed.len() - 1];
            // If stripping quotes reveals the prefix, use it
            if unquoted.starts_with("42[") {
                return Self::parse(unquoted);
            }
        }

        let prefix = "42[\"auth\",";

        let parsed = if let Some(stripped) = trimmed.strip_prefix(prefix) {
            stripped.strip_suffix("]").ok_or_else(|| {
                CoreError::SsidParsing("Error parsing ssid: missing closing bracket".into())
            })?
        } else {
            trimmed
        };

        let mut ssid: Demo = serde_json::from_str(parsed)
            .map_err(|e| CoreError::SsidParsing(format!("JSON parsing error: {e}")))?;

        ssid.raw = trimmed.to_string();
        ssid.json_raw = parsed.to_string();

        let is_demo_url = ssid
            .current_url
            .as_deref()
            .is_some_and(|s| s.contains("demo"));

        if ssid.is_demo == 1 || is_demo_url {
            tracing::debug!(target: "Ssid", "Parsed Demo SSID. UID: {}", ssid.uid);
            Ok(Self::Demo(ssid))
        } else {
            let session_raw = ssid.session.clone();
            let json_raw = ssid.json_raw.clone();
            let raw = ssid.raw.clone();
            let session_data = {
                let session_bytes = ssid.session.as_bytes();
                match php_serde::from_bytes::<SessionData>(session_bytes) {
                    Ok(s) => s,
                    Err(_) => {
                        // Try stripping the trailing hash (assuming 32 chars for MD5)
                        if session_bytes.len() > 32 {
                            let stripped = &session_bytes[..session_bytes.len() - 32];
                            php_serde::from_bytes(stripped).map_err(|e| {
                                CoreError::SsidParsing(format!("Error parsing session data: {e}"))
                            })?
                        } else {
                            return Err(CoreError::SsidParsing(
                                "Error parsing session data".into(),
                            ));
                        }
                    }
                }
            };

            let redacted_ip = if let Some(idx) = session_data.ip_address.rfind('.') {
                format!("{}.xxx", &session_data.ip_address[..idx])
            } else if let Some(idx) = session_data.ip_address.rfind(':') {
                format!("{}:xxx", &session_data.ip_address[..idx])
            } else {
                "REDACTED".to_string()
            };

            tracing::debug!(target: "Ssid", "Parsed Real SSID. UID: {}, IP: {}, UA: {}", 
                ssid.uid, redacted_ip, session_data.user_agent);

            let real = Real {
                raw,
                is_demo: ssid.is_demo,
                session_raw,
                json_raw,
                session: session_data,
                uid: ssid.uid,
                platform: ssid.platform,
                is_fast_history: ssid.is_fast_history,
                is_optimized: ssid.is_optimized,
                extra: ssid.extra,
            };
            Ok(Self::Real(real))
        }
    }

    pub async fn server(&self) -> CoreResult<String> {
        match self {
            Self::Demo(_) => Ok(Regions::DEMO.0.to_string()),
            Self::Real(real) => Regions
                .get_server_for_ip(&real.session.ip_address)
                .await
                .map(|s| s.to_string())
                .map_err(|e| CoreError::HttpRequest(e.to_string())),
        }
    }

    pub async fn servers(&self) -> CoreResult<Vec<String>> {
        match self {
            Self::Demo(_) => Ok(Regions::demo_regions_str()
                .iter()
                .map(|r| r.to_string())
                .collect()),
            Self::Real(real) => Ok(Regions
                .get_servers_for_ip(&real.session.ip_address)
                .await
                .map_err(|e| CoreError::HttpRequest(e.to_string()))?
                .iter()
                .map(|s| s.to_string())
                .collect()),
        }
    }

    pub fn user_agent(&self) -> String {
        match self {
            Self::Demo(_) => "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36".into(),
            Self::Real(real) => real.session.user_agent.clone(),
        }
    }

    pub fn ip_address(&self) -> Option<&str> {
        match self {
            Self::Demo(_) => None,
            Self::Real(real) => Some(&real.session.ip_address),
        }
    }

    /// Returns true if the session is a demo session.
    pub fn demo(&self) -> bool {
        match self {
            Self::Demo(_) => true,
            Self::Real(_) => false,
        }
    }

    /// Get the current_url from the SSID if available.
    /// For Demo accounts, this is stored directly.
    /// For Real accounts, this may be in the extra field.
    pub fn current_url(&self) -> Option<String> {
        match self {
            Self::Demo(demo) => demo.current_url.clone(),
            Self::Real(real) => {
                // Try to get current_url from the extra field
                if let Some(url) = real
                    .extra
                    .get("currentUrl")
                    .or_else(|| real.extra.get("current_url"))
                {
                    url.as_str().map(String::from)
                } else {
                    None
                }
            }
        }
    }

    pub fn session_id(&self) -> String {
        match self {
            Self::Demo(demo) => demo.session.clone(),
            Self::Real(real) => real.session_raw.clone(),
        }
    }
}
impl fmt::Display for Demo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if !self.raw.is_empty() {
            write!(f, "{}", self.raw)
        } else {
            let ssid = serde_json::to_string(&self).map_err(|_| fmt::Error)?;
            write!(f, r#"42["auth",{ssid}]"#)
        }
    }
}

impl fmt::Display for Real {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
    }
}

impl fmt::Display for Ssid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Demo(demo) => demo.fmt(f),
            Self::Real(real) => real.fmt(f),
        }
    }
}

impl<'de> Deserialize<'de> for Ssid {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let data: Value = Value::deserialize(deserializer)?;
        Ssid::parse(data).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn test_descerialize_session() -> Result<(), Box<dyn Error>> {
        let session_raw = b"a:4:{s:10:\"session_id\";s:32:\"00000000000000000000000000000000\";s:10:\"ip_address\";s:7:\"0.0.0.0\";s:10:\"user_agent\";s:111:\"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/144.0.0.0 Safari/537.36\";s:13:\"last_activity\";i:1732926685;}00000000000000000000000000000000";
        let session: SessionData = php_serde::from_bytes(session_raw)?;
        dbg!(&session);
        let session_php = php_serde::to_vec(&session)?;
        dbg!(String::from_utf8(session_php).unwrap());
        Ok(())
    }

    #[test]
    fn test_parse_ssid() -> Result<(), Box<dyn Error>> {
        let ssids = [
            r#"42["auth",{"session":"a:4:{s:10:\"session_id\";s:32:\"00000000000000000000000000000000\";s:10:\"ip_address\";s:7:\"0.0.0.0\";s:10:\"user_agent\";s:111:\"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/144.0.0.0 Safari/537.36\";s:13:\"last_activity\";i:1732926685;}00000000000000000000000000000000","isDemo":0,"uid":12345678,"platform":2}]"#,
            r#"42["auth",{"session":"dummy_session_id","isDemo":1,"uid":87654321,"platform":2}]"#,
        ];
        for ssid in ssids {
            let parsed = Ssid::parse(ssid)?;
            let reconstructed = parsed.to_string();
            let re_parsed = Ssid::parse(&reconstructed)?;
            assert_eq!(format!("{:?}", parsed), format!("{:?}", re_parsed));
        }
        Ok(())
    }
}
