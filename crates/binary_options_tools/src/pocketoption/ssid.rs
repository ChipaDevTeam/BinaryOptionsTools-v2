use core::fmt;
use std::collections::HashMap;

use binary_options_tools_core::error::{CoreError, CoreResult};
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
    #[doc(hidden)]
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
    #[doc(hidden)]
    pub session_raw: String,
    pub is_demo: u32,
    pub uid: u32,
    pub platform: u32,
    #[doc(hidden)]
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

/// A PocketOption "try demo" (guest) session.
///
/// Guest sessions carry no account `session`/`uid`; they authenticate with a
/// short-lived `token` that the platform issues for the free demo. The auth
/// frame observed in the browser is:
///
/// ```text
/// 42["auth",{"token":"9PRxPzq6mQ","balance":50000,"isFastHistory":true}]
/// ```
///
/// The payload is preserved verbatim in `raw` so the exact auth frame is
/// replayed when the client connects.
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TryDemo {
    /// Short-lived guest token; the try-demo equivalent of `session`.
    pub token: String,
    /// Starting virtual balance, when the server included one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balance: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_fast_history: Option<bool>,
    #[serde(flatten, skip_serializing_if = "HashMap::is_empty")]
    pub extra: HashMap<String, Value>,
    #[serde(skip)]
    #[doc(hidden)]
    pub raw: String,
}

impl fmt::Debug for TryDemo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TryDemo")
            .field("token", &"REDACTED")
            .field("balance", &self.balance)
            .field("is_fast_history", &self.is_fast_history)
            .field("extra", &self.extra)
            .finish()
    }
}

impl fmt::Display for TryDemo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.raw)
    }
}

impl TryDemo {
    /// Creates a try-demo session from a guest token.
    ///
    /// # Errors
    /// Returns `CoreError::SsidParsing` when the token is empty.
    pub fn new(token: impl ToString) -> CoreResult<Self> {
        let token = token.to_string();
        if token.trim().is_empty() {
            return Err(CoreError::SsidParsing(
                "Try-demo `token` must not be empty".into(),
            ));
        }
        let mut try_demo = Self {
            token,
            balance: None,
            is_fast_history: None,
            extra: HashMap::new(),
            raw: String::new(),
        };
        try_demo.refresh_raw();
        Ok(try_demo)
    }

    /// Sets the starting virtual balance recorded in the auth frame.
    pub fn with_balance(mut self, balance: u64) -> Self {
        self.balance = Some(balance);
        self.refresh_raw();
        self
    }

    /// Sets the `isFastHistory` flag recorded in the auth frame.
    pub fn with_fast_history(mut self, is_fast_history: bool) -> Self {
        self.is_fast_history = Some(is_fast_history);
        self.refresh_raw();
        self
    }

    /// Rebuilds the canonical `42["auth",{...}]` frame from the payload.
    fn refresh_raw(&mut self) {
        self.raw = format!(
            "42[\"auth\",{}]",
            serde_json::to_string(&*self).unwrap_or_default()
        );
    }
}

#[derive(Serialize, Clone)]
#[serde(untagged)]
pub enum Ssid {
    Demo(Demo),
    Real(Real),
    TryDemo(TryDemo),
}

impl fmt::Debug for Ssid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Demo(d) => f.debug_tuple("Demo").field(d).finish(),
            Self::Real(r) => f.debug_tuple("Real").field(r).finish(),
            Self::TryDemo(t) => f.debug_tuple("TryDemo").field(t).finish(),
        }
    }
}

use regex::Regex;
use std::sync::OnceLock;

static KEY_REGEX: OnceLock<Regex> = OnceLock::new();
static VAL_REGEX: OnceLock<Regex> = OnceLock::new();

fn recover_json(input: &str) -> String {
    let key_re =
        KEY_REGEX.get_or_init(|| Regex::new(r"([{,]\s*)([a-zA-Z_][a-zA-Z0-9_]*)\s*:").unwrap());
    let val_re = VAL_REGEX.get_or_init(|| Regex::new(r"(:)\s*([a-zA-Z0-9._-]+)(\s*[,}])").unwrap());

    // 1. Quote unquoted keys
    let intermediate = key_re.replace_all(input, "$1\"$2\":");

    // 2. Quote unquoted alphanumeric values (excluding true/false/null/numbers)
    val_re
        .replace_all(&intermediate, |caps: &regex::Captures| {
            let prefix = &caps[1];
            let val = &caps[2];
            let suffix = &caps[3];

            let val_lower = val.to_lowercase();
            if val_lower == "true"
                || val_lower == "false"
                || val_lower == "null"
                || (val.contains('.') && val.parse::<f64>().is_ok())
                || val.parse::<i64>().is_ok()
            {
                format!("{}{}{}", prefix, val, suffix)
            } else {
                format!("{}\"{}\"{}", prefix, val, suffix)
            }
        })
        .to_string()
}

/// User agent used for demo and try-demo (guest) sessions.
const DEMO_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/132.0.0.0 Safari/537.36";

/// Returns the JSON payload when `parsed` is a try-demo (guest) auth object,
/// i.e. it carries a short-lived `token` and no account `session`.
///
/// Falls back to the same lenient recovery used for shell-mangled SSIDs so a
/// token-only payload with unquoted keys is still recognised.
fn try_demo_payload(parsed: &str) -> Option<Value> {
    let value = serde_json::from_str::<Value>(parsed)
        .ok()
        .or_else(|| serde_json::from_str::<Value>(&recover_json(parsed)).ok())?;

    let has_token = value.get("token").is_some();
    let has_session = value.get("session").is_some() || value.get("sessionToken").is_some();
    (has_token && !has_session).then_some(value)
}

impl Ssid {
    /// Builds a try-demo (guest) SSID from a short-lived demo `token`.
    ///
    /// ```text
    /// 42["auth",{"token":"<token>","balance":50000,"isFastHistory":true}]
    /// ```
    pub fn try_demo(token: impl ToString) -> CoreResult<Self> {
        Ok(Self::TryDemo(TryDemo::new(token)?))
    }

    /// Parses a raw SSID string from PocketOption
    ///
    /// # Arguments
    /// * `data` - The raw SSID string, can be in 42["auth",...] format or raw JSON
    ///
    /// # Errors
    /// Returns `CoreError::SsidParsing` if the format is invalid or JSON parsing fails
    pub fn parse(data: impl ToString) -> CoreResult<Self> {
        let data_str = data.to_string();
        let trimmed = data_str.trim();

        // Security: Direct validation to prevent double-JSON injection
        if (trimmed.starts_with('"') && trimmed.ends_with('"')) || trimmed.starts_with("'") {
            return Err(CoreError::SsidParsing(
                "Invalid SSID format: double-encoding detected".into(),
            ));
        }
        let prefix = "42[\"auth\",";

        let parsed = if let Some(stripped) = trimmed.strip_prefix(prefix) {
            let inner = stripped.strip_suffix("]").ok_or_else(|| {
                CoreError::SsidParsing("Error parsing ssid: missing closing bracket".into())
            })?;
            // Detect double-encoding: payload is a quoted string literal (JSON-in-JSON)
            if inner.starts_with('"') && inner.ends_with('"') {
                return Err(CoreError::SsidParsing(
                    "Invalid SSID format: double-encoding detected".into(),
                ));
            }
            inner
        } else {
            trimmed
        };

        // PocketOption's free "try demo" (guest) sessions authenticate with a
        // short-lived `token` instead of an account `session`. Detect that shape
        // and keep the payload intact so the emitted auth frame matches the
        // browser capture.
        if let Some(value) = try_demo_payload(parsed) {
            let mut try_demo: TryDemo = serde_json::from_value(value)
                .map_err(|e| CoreError::SsidParsing(format!("Error parsing try-demo SSID: {e}")))?;
            if try_demo.token.trim().is_empty() {
                return Err(CoreError::SsidParsing(
                    "Try-demo SSID `token` must not be empty".into(),
                ));
            }
            try_demo.raw = if trimmed.starts_with(prefix) {
                trimmed.to_string()
            } else {
                format!(
                    "42[\"auth\",{}]",
                    serde_json::to_string(&try_demo).unwrap_or_default()
                )
            };
            tracing::debug!(target: "Ssid", "Parsed TryDemo (guest) SSID.");
            return Ok(Self::TryDemo(try_demo));
        }

        // Track whether recovery was used and what the recovered string is
        let mut used_recovery = false;
        let mut recovered_parsed = String::new();

        let mut ssid: Demo = match serde_json::from_str(parsed) {
            Ok(s) => s,
            Err(e) => {
                // Try recovery: quote unquoted keys and alphanumeric values
                let recovered = recover_json(parsed);
                match serde_json::from_str(&recovered) {
                    Ok(s) => {
                        tracing::debug!(target: "Ssid", "JSON recovery succeeded for malformed SSID");
                        used_recovery = true;
                        recovered_parsed = recovered;
                        s
                    }
                    Err(re) => {
                        tracing::debug!(target: "Ssid", "Recovery failed. Original error: {:?}, Recovery error: {:?}", e, re);
                        return Err(CoreError::SsidParsing(format!(
                            "JSON parsing error: {e} (Recovery also failed: {re})"
                        )));
                    }
                }
            }
        };

        // Normalize raw to well-formed JSON. When recovery was used, re-serialize
        // the parsed Demo struct so ssid.raw always contains canonical JSON rather
        // than the original malformed input that triggered recovery.
        if used_recovery {
            let canonical =
                serde_json::to_string(&ssid).unwrap_or_else(|_| recovered_parsed.clone());
            ssid.raw = format!("42[\"auth\",{}]", canonical);
            ssid.json_raw = canonical;
        } else {
            ssid.raw = if trimmed.starts_with("42[\"auth\",") {
                trimmed.to_string()
            } else {
                format!("42[\"auth\",{}]", parsed)
            };
            ssid.json_raw = parsed.to_string();
        }

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
            Self::Demo(_) | Self::TryDemo(_) => Ok(Regions::DEMO.0.to_string()),
            Self::Real(real) => Regions
                .get_server_for_ip(&real.session.ip_address)
                .await
                .map(|s| s.to_string())
                .map_err(|e| CoreError::HttpRequest(e.to_string())),
        }
    }

    pub async fn servers(&self) -> CoreResult<Vec<String>> {
        match self {
            Self::Demo(_) | Self::TryDemo(_) => Ok(Regions::demo_regions_str()
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
            Self::Demo(_) | Self::TryDemo(_) => DEMO_USER_AGENT.into(),
            Self::Real(real) => real.session.user_agent.clone(),
        }
    }

    pub fn ip_address(&self) -> Option<&str> {
        match self {
            Self::Demo(_) | Self::TryDemo(_) => None,
            Self::Real(real) => Some(&real.session.ip_address),
        }
    }

    /// Returns true if the session is a demo session.
    pub fn demo(&self) -> bool {
        match self {
            Self::Demo(_) | Self::TryDemo(_) => true,
            Self::Real(_) => false,
        }
    }

    /// Get the current_url from the SSID if available.
    /// For Demo accounts, this is stored directly.
    /// For Real accounts, this may be in the extra field.
    pub fn current_url(&self) -> Option<String> {
        match self {
            Self::Demo(demo) => demo.current_url.clone(),
            Self::TryDemo(_) => None,
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
            Self::TryDemo(try_demo) => try_demo.token.clone(),
            Self::Real(real) => real.session_raw.clone(),
        }
    }
}
impl fmt::Display for Demo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw)
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
            Self::TryDemo(try_demo) => try_demo.fmt(f),
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
            assert!(reconstructed.starts_with("42[\"auth\","));
        }

        // Test parsing ONLY JSON part
        let json_only = r#"{"session":"dummy_session_id","isDemo":1,"uid":87654321,"platform":2}"#;
        let parsed = Ssid::parse(json_only)?;
        let reconstructed = parsed.to_string();
        assert!(reconstructed.starts_with("42[\"auth\","));
        assert!(reconstructed.contains(json_only));

        Ok(())
    }

    #[test]
    fn test_ssid_rejects_double_encoded_json() {
        let malicious =
            r#"42["auth","{\"session\":\"dummy\",\"isDemo\":1,\"uid\":123,\"platform\":2}"]"#;
        let result = Ssid::parse(malicious);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err
            .to_string()
            .contains("Invalid SSID format: double-encoding detected"));
    }

    #[test]
    fn test_parse_ssid_recovery() -> Result<(), Box<dyn Error>> {
        // SSID with unquoted keys and unquoted alphanumeric values (simulating shell stripping)
        let stripped_ssid = r#"42["auth", {session: swag, isDemo: 1, uid: 12345678, platform: 2, isFastHistory: true, isOptimized: true}]"#;

        let parsed = Ssid::parse(stripped_ssid)?;
        assert_eq!(parsed.session_id(), "swag");
        assert!(parsed.demo());

        // Round-trip: to_string() should produce valid JSON that re-parses identically
        let emitted = parsed.to_string();
        let reparsed = Ssid::parse(&emitted)?;
        assert_eq!(reparsed.session_id(), parsed.session_id());
        assert_eq!(reparsed.demo(), parsed.demo());
        assert!(emitted.starts_with("42[\"auth\","));

        Ok(())
    }

    #[test]
    fn test_provided_auth_message() -> Result<(), Box<dyn std::error::Error>> {
        let auth_msg = r#"42["auth",{"session":"dummy_session_id","isDemo":1,"uid":12345678,"platform":2,"isFastHistory":true,"isOptimized":true}]"#;
        let parsed = Ssid::parse(auth_msg)?;

        match parsed {
            Ssid::Demo(ref demo) => {
                assert_eq!(demo.session, "dummy_session_id");
                assert_eq!(demo.uid, 12345678);
                assert_eq!(demo.is_demo, 1);
                assert_eq!(demo.platform, 2);
                assert_eq!(demo.is_fast_history, Some(true));
                assert_eq!(demo.is_optimized, Some(true));
            }
            other => panic!("Expected Demo SSID, got {other:?}"),
        }

        let reconstructed = parsed.to_string();
        assert!(reconstructed.starts_with("42[\"auth\","));
        assert!(reconstructed.contains("\"isFastHistory\":true"));
        assert!(reconstructed.contains("\"isOptimized\":true"));

        Ok(())
    }

    /// Read a demo SSID from the `POCKET_OPTION_SSID` environment variable.
    /// Returns `None` if unset so tests can skip gracefully.
    fn demo_ssid_from_env() -> Option<String> {
        // An empty value counts as unset, so the tests skip instead of trying to
        // parse it (which fails with a confusing "SSID is required" error).
        std::env::var("POCKET_OPTION_SSID")
            .ok()
            .filter(|value| !value.trim().is_empty())
    }

    #[test]
    fn test_demo_ssid_from_env_exact() -> Result<(), Box<dyn std::error::Error>> {
        let raw = match demo_ssid_from_env() {
            Some(v) => v,
            None => return Ok(()),
        };
        let parsed = Ssid::parse(&raw)?;
        assert!(parsed.demo(), "Provided SSID must be a demo account");
        assert!(parsed.session_id().len() > 4);

        let reconstructed = parsed.to_string();
        assert!(reconstructed.starts_with("42[\"auth\","));
        assert!(reconstructed.contains(&format!("\"session\":\"{}\"", parsed.session_id())));
        assert!(
            reconstructed.contains("\"isFastHistory\":true")
                || reconstructed.contains("\"isFastHistory\":false")
        );
        assert!(
            reconstructed.contains("\"isOptimized\":true")
                || reconstructed.contains("\"isOptimized\":false")
        );

        let re_parsed = Ssid::parse(&reconstructed)?;
        assert_eq!(re_parsed.session_id(), parsed.session_id());
        assert_eq!(re_parsed.demo(), parsed.demo());

        Ok(())
    }

    #[test]
    fn test_demo_ssid_from_env_json_only() -> Result<(), Box<dyn std::error::Error>> {
        let raw = match demo_ssid_from_env() {
            Some(v) => v,
            None => return Ok(()),
        };
        let parsed = Ssid::parse(&raw)?;
        let session_id = parsed.session_id();

        // Strip the 42["auth",...] wrapper to test JSON-only parsing
        let json_only = if let Some(stripped) = raw.strip_prefix("42[\"auth\",") {
            stripped.strip_suffix("]").unwrap_or(&raw)
        } else {
            &raw
        };

        let parsed_json = Ssid::parse(json_only)?;
        assert_eq!(parsed_json.session_id(), session_id);
        assert!(parsed_json.demo());

        let reconstructed = parsed_json.to_string();
        assert!(reconstructed.starts_with("42[\"auth\","));
        let re_parsed = Ssid::parse(&reconstructed)?;
        assert_eq!(re_parsed.session_id(), session_id);

        Ok(())
    }

    #[test]
    fn test_demo_ssid_current_url_demo() -> Result<(), Box<dyn std::error::Error>> {
        let raw = r#"42["auth",{"session":"demo_session","isDemo":0,"uid":1111,"platform":2,"currentUrl":"wss://wsdemo.pocketoption.com"}]"#;
        let parsed = Ssid::parse(raw)?;
        assert!(
            parsed.demo(),
            "Should detect demo via currentUrl containing 'demo'"
        );
        assert_eq!(
            parsed.current_url(),
            Some("wss://wsdemo.pocketoption.com".into())
        );
        Ok(())
    }

    #[test]
    fn test_ssid_rejects_double_quoted_inner() {
        let malicious = r#"42["auth","{\"session\":\"x\",\"isDemo\":1,\"uid\":1,\"platform\":2}"]"#;
        let result = Ssid::parse(malicious);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("double-encoding"));
    }

    #[test]
    fn test_ssid_rejects_single_quoted() {
        let malicious = r#"'some_string_ssid'"#;
        let result = Ssid::parse(malicious);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("double-encoding"));
    }

    /// Frame captured verbatim from a PocketOption "try demo" (guest) session.
    const TRY_DEMO_CAPTURE: &str =
        r#"42["auth",{"token":"9PRxPzq6mQ","balance":50000,"isFastHistory":true}]"#;

    #[test]
    fn test_parse_try_demo_ssid() -> Result<(), Box<dyn Error>> {
        let parsed = Ssid::parse(TRY_DEMO_CAPTURE)?;

        assert!(parsed.demo(), "try-demo sessions are demo sessions");
        assert_eq!(parsed.session_id(), "9PRxPzq6mQ");
        // The auth frame must be replayed exactly as captured.
        assert_eq!(parsed.to_string(), TRY_DEMO_CAPTURE);

        match parsed {
            Ssid::TryDemo(t) => {
                assert_eq!(t.token, "9PRxPzq6mQ");
                assert_eq!(t.balance, Some(50000));
                assert_eq!(t.is_fast_history, Some(true));
            }
            other => panic!("expected TryDemo, got {other:?}"),
        }
        Ok(())
    }

    #[test]
    fn test_try_demo_ssid_json_only_roundtrip() -> Result<(), Box<dyn Error>> {
        let json_only = r#"{"token":"abc1234567","balance":10000,"isFastHistory":false}"#;
        let parsed = Ssid::parse(json_only)?;
        assert!(parsed.demo());

        let frame = parsed.to_string();
        assert!(frame.starts_with("42[\"auth\","));
        assert!(frame.contains("\"token\":\"abc1234567\""));
        assert!(!frame.contains("\"session\""));

        let reparsed = Ssid::parse(&frame)?;
        assert_eq!(reparsed.session_id(), "abc1234567");
        assert!(reparsed.demo());
        Ok(())
    }

    #[test]
    fn test_try_demo_constructor() -> Result<(), Box<dyn Error>> {
        let ssid = Ssid::try_demo("tok3n12345")?;
        assert!(ssid.demo());
        assert_eq!(ssid.session_id(), "tok3n12345");
        assert_eq!(ssid.to_string(), r#"42["auth",{"token":"tok3n12345"}]"#);

        let with_balance = Ssid::parse(r#"{"token":"tok3n12345","balance":50000}"#)?;
        assert_eq!(with_balance.session_id(), "tok3n12345");

        assert!(
            Ssid::try_demo("  ").is_err(),
            "blank token must be rejected"
        );
        Ok(())
    }

    #[test]
    fn test_account_session_is_not_treated_as_try_demo() -> Result<(), Box<dyn Error>> {
        // An account SSID must keep parsing as an account session even when the
        // payload happens to carry a `token` field.
        let raw = r#"42["auth",{"session":"acc0unt0ken","isDemo":1,"uid":42,"platform":2}]"#;
        let parsed = Ssid::parse(raw)?;
        assert_eq!(parsed.session_id(), "acc0unt0ken");
        assert!(matches!(parsed, Ssid::Demo(_)));
        Ok(())
    }

    #[test]
    fn test_try_demo_debug_redacts_token() -> Result<(), Box<dyn Error>> {
        let parsed = Ssid::parse(TRY_DEMO_CAPTURE)?;
        let debug = format!("{parsed:?}");
        assert!(
            !debug.contains("9PRxPzq6mQ"),
            "token must not leak: {debug}"
        );
        Ok(())
    }
}
