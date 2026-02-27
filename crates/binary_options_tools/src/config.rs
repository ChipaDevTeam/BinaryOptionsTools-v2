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
            connection_initialization_timeout: Duration::from_secs(60),
            timeout: Duration::from_secs(30),
            urls: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert_eq!(config.max_allowed_loops, 100);
        assert_eq!(config.sleep_interval, Duration::from_millis(100));
        assert_eq!(config.reconnect_time, Duration::from_secs(5));
        assert_eq!(
            config.connection_initialization_timeout,
            Duration::from_secs(60)
        );
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert!(config.urls.is_empty());
    }

    #[test]
    fn test_config_clone_and_debug() {
        let config = Config::default();
        let cloned = config.clone();
        assert_eq!(cloned.max_allowed_loops, config.max_allowed_loops);

        let debug_str = format!("{:?}", config);
        assert!(debug_str.contains("Config"));
        assert!(debug_str.contains("max_allowed_loops: 100"));
    }
}
