use std::time::Duration;
use url::Url;

#[derive(Clone, Debug)]
pub struct Config {
    pub max_allowed_loops: u32,
    pub sleep_interval: Duration,
    pub reconnect_time: Duration,
    pub connection_initialization_timeout: Duration,
    pub timeout: Duration,
    pub urls: Vec<Url>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            max_allowed_loops: 100,
            sleep_interval: Duration::from_millis(100),
            reconnect_time: Duration::from_secs(5),
            connection_initialization_timeout: Duration::from_secs(30),
            timeout: Duration::from_secs(30),
            urls: Vec::new(),
        }
    }
}
