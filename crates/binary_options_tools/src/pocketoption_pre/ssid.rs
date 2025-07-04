use core::fmt;
use std::collections::HashMap;

use binary_options_tools_core_pre::error::{CoreError, CoreResult};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use binary_options_tools_core::general::traits::Credentials;

use super::regions::Regions;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SessionData {
    session_id: String,
    ip_address: String,
    user_agent: String,
    last_activity: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Demo {
    session: String,
    is_demo: u32,
    uid: u32,
    platform: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_fast_history: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    is_optimized: Option<bool>,
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Real {
    session: SessionData,
    is_demo: u32,
    uid: u32,
    platform: u32,
    raw: String,
    is_fast_history: Option<bool>,
    is_optimized: Option<bool>,
    #[serde(flatten)]
    extra: HashMap<String, Value>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(untagged)]
pub enum Ssid {
    Demo(Demo),
    Real(Real),
}

impl Ssid {
    pub fn parse(data: impl ToString) -> CoreResult<Self> {
        let data = data.to_string();
        let parsed = data
            .trim()
            .strip_prefix(r#"42["auth","#)
            .ok_or(CoreError::SsidParsing(
                "Error parsing ssid string into object".into(),
            ))?
            .strip_suffix("]")
            .ok_or(CoreError::SsidParsing(
                "Error parsing ssid string into object".into(),
            ))?;
        let ssid: Demo = serde_json::from_str(parsed)
            .map_err(|e| CoreError::SsidParsing(e.to_string()))?;
        if ssid.is_demo == 1 {
            Ok(Self::Demo(ssid))
        } else {
            let real = Real {
                raw: data,
                is_demo: ssid.is_demo,
                session: php_serde::from_bytes(ssid.session.as_bytes()).map_err(|e| {
                    CoreError::SsidParsing(format!("Error parsing session data, {e}"))
                })?,
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
            Self::Real(_) => Regions.get_server().await.map(|s| s.to_string()).map_err(|e| {
                CoreError::HttpRequest(e.to_string())
            }),
        }
    }

    pub async fn servers(&self) -> CoreResult<Vec<String>> {
        match self {
            Self::Demo(_) => Ok(Regions::demo_regions_str()
                .iter()
                .map(|r| r.to_string())
                .collect()),
            Self::Real(_) => Ok(Regions
                .get_servers()
                .await.map_err(|e| CoreError::HttpRequest(e.to_string()))?
                .iter()
                .map(|s| s.to_string())
                .collect()),
        }
    }

    pub fn user_agent(&self) -> String {
        match self {
            Self::Demo(_) => "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36".into(),
            Self::Real(real) => real.session.user_agent.clone()
        }
    }

    pub fn demo(&self) -> bool {
        match self {
            Self::Demo(_) => true,
            Self::Real(_) => false,
        }
    }
}
impl fmt::Display for Demo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ssid = serde_json::to_string(&self).map_err(|_| fmt::Error)?;
        write!(f, r#"42["auth",{ssid}]"#)
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

impl Credentials for Ssid {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn test_descerialize_session() -> Result<(), Box<dyn Error>> {
        let session_raw = b"a:4:{s:10:\"session_id\";s:32:\"ae3aa847add89c341ec18d8ae5bf8527\";s:10:\"ip_address\";s:15:\"191.113.157.139\";s:10:\"user_agent\";s:120:\"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36 OPR/114.\";s:13:\"last_activity\";i:1732926685;}31666d2dc07fdd866353937b97901e2b";
        let session: SessionData = php_serde::from_bytes(session_raw)?;
        dbg!(&session);
        let session_php = php_serde::to_vec(&session)?;
        dbg!(String::from_utf8(session_php).unwrap());
        Ok(())
    }

    #[test]
    fn test_parse_ssid() -> Result<(), Box<dyn Error>> {
        let ssids = [
            // r#"42["auth",{"session":"looc69ct294h546o368s0lct7d","isDemo":1,"uid":87742848,"platform":2}]	"#,
            r#"42["auth",{"session":"a:4:{s:10:\"session_id\";s:32:\"ae3aa847add89c341ec18d8ae5bf8527\";s:10:\"ip_address\";s:15:\"191.113.157.139\";s:10:\"user_agent\";s:120:\"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36 OPR/114.\";s:13:\"last_activity\";i:1732926685;}31666d2dc07fdd866353937b97901e2b","isDemo":0,"uid":87742848,"platform":2}]	"#,
            r#"42["auth",{"session":"vtftn12e6f5f5008moitsd6skl","isDemo":1,"uid":27658142,"platform":2}]"#,
            r#"42["auth",{"session":"a:4:{s:10:\"session_id\";s:32:\"f10395d38f61039ea0a20ba26222895a\";s:10:\"ip_address\";s:12:\"79.177.168.1\";s:10:\"user_agent\";s:111:\"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/133.0.0.0 Safari/537.36\";s:13:\"last_activity\";i:1740261136;}9bef184e52d025d1f07068eeaf555637","isDemo":0,"uid":89028022,"platform":2}]"#,
            r#"42["auth",{"session":"a:4:{s:10:\"session_id\";s:32:\"bebb6bb272efc3b8be0e37ae5eb814c6\";s:10:\"ip_address\";s:14:\"191.113.152.39\";s:10:\"user_agent\";s:120:\"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36 OPR/117.\";s:13:\"last_activity\";i:1742420144;}56b1857cbcf8d66f9bd81900e36803d4","isDemo":0,"uid":87742848,"platform":2}]"#,
            r#"42["auth",{"session":"a:4:{s:10:\"session_id\";s:32:\"f729997775af4ad480d5787c5bc94584\";s:10:\"ip_address\";s:14:\"191.113.152.39\";s:10:\"user_agent\";s:120:\"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36 OPR/117.\";s:13:\"last_activity\";i:1742422103;}20db11eee2b7f75a5244e9faf5cd4f4a","isDemo":0,"uid":96669015,"platform":2}]    "#,
            r#"42["auth",{"session":"a:4:{s:10:\"session_id\";s:32:\"256a82f814e5a1ecca6f2c337262b4d6\";s:10:\"ip_address\";s:12:\"89.172.73.91\";s:10:\"user_agent\";s:80:\"Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:136.0) Gecko/20100101 Firefox/136.0\";s:13:\"last_activity\";i:1742422004;}a3e2ef2e4084593ec39d023337564e37","isDemo":0,"uid":96669015,"platform":2}]"#,
            r#"42["auth",{"session":"a:4:{s:10:\"session_id\";s:32:\"be8de3a8cb5fed23efebb631902263e2\";s:10:\"ip_address\";s:15:\"191.113.139.200\";s:10:\"user_agent\";s:120:\"Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/134.0.0.0 Safari/537.36 OPR/119.\";s:13:\"last_activity\";i:1751057233;}b9d0db50cb32d406f935c63a41484f27","isDemo":0,"uid":104155994,"platform":2,"isFastHistory":true,"isOptimized":true}]	"#,
        ];
        for ssid in ssids {
            let valid = Ssid::parse(ssid)?;
            dbg!(valid);
        }
        Ok(())
    }
}
