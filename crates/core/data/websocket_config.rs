use std::{collections::HashMap, time::Duration};

use serde::{Deserialize, Serialize};
use url::Url;

use crate::constants::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebSocketConfig {
    // Connection settings
    pub ping_interval: Duration,
    pub ping_timeout: Duration,
    pub close_timeout: Duration,
    pub max_reconnect_attempts: u32,
    pub reconnect_delay: Duration,
    pub message_timeout: Duration,
    pub connection_timeout: Duration,
    
    // Performance settings
    pub batch_size: usize,
    pub batch_timeout: Duration,
    pub max_concurrent_operations: usize,
    pub cache_ttl: Duration,
    pub rate_limit: Option<u32>,
    
    // SSL and headers
    pub ssl_verify: bool,
    pub custom_headers: HashMap<String, String>,
    
    // Connection pool settings
    pub max_connections: usize,
    pub connection_stats_history: usize,
    
    // Health monitoring
    pub health_check_interval: Duration,
    pub enable_health_monitoring: bool,
    
    // Event system
    pub event_buffer_size: usize,
    pub enable_event_system: bool,
    
    // Fallback URLs
    pub fallback_urls: Vec<Url>,
}

impl Default for WebSocketConfig {
    fn default() -> Self {
        let mut headers = HashMap::new();
        headers.insert("Origin".to_string(), DEFAULT_ORIGIN.to_string());
        headers.insert("User-Agent".to_string(), DEFAULT_USER_AGENT.to_string());
        headers.insert("Cache-Control".to_string(), "no-cache".to_string());
        
        Self {
            ping_interval: DEFAULT_PING_INTERVAL,
            ping_timeout: DEFAULT_PING_TIMEOUT,
            close_timeout: DEFAULT_CLOSE_TIMEOUT,
            max_reconnect_attempts: DEFAULT_MAX_RECONNECT_ATTEMPTS,
            reconnect_delay: DEFAULT_RECONNECT_DELAY,
            message_timeout: DEFAULT_MESSAGE_TIMEOUT,
            connection_timeout: DEFAULT_CONNECTION_TIMEOUT,
            
            batch_size: DEFAULT_BATCH_SIZE,
            batch_timeout: DEFAULT_BATCH_TIMEOUT,
            max_concurrent_operations: DEFAULT_MAX_CONCURRENT_OPERATIONS,
            cache_ttl: DEFAULT_CACHE_TTL,
            rate_limit: Some(DEFAULT_RATE_LIMIT),
            
            ssl_verify: false, // For PocketOption compatibility
            custom_headers: headers,
            
            max_connections: DEFAULT_MAX_CONNECTIONS,
            connection_stats_history: CONNECTION_STATS_HISTORY_SIZE,
            
            health_check_interval: HEALTH_CHECK_INTERVAL,
            enable_health_monitoring: true,
            
            event_buffer_size: EVENT_BUFFER_SIZE,
            enable_event_system: true,
            
            fallback_urls: Vec::new(),
        }
    }
}

impl WebSocketConfig {
    pub fn builder() -> WebSocketConfigBuilder {
        WebSocketConfigBuilder::default()
    }
    
    pub fn for_pocketoption() -> Self {
        let mut config = Self::default();
        
        // PocketOption specific settings
        config.ping_interval = Duration::from_secs(20);
        config.ssl_verify = false;
        config.batch_size = 5; // Smaller batches for real-time trading
        config.batch_timeout = Duration::from_millis(50);
        
        // Add PocketOption fallback URLs
        let fallback_urls = vec![
            "wss://api-eu.po.market/socket.io/?EIO=4&transport=websocket",
            "wss://api-sc.po.market/socket.io/?EIO=4&transport=websocket",
            "wss://api-hk.po.market/socket.io/?EIO=4&transport=websocket",
            "wss://demo-api-eu.po.market/socket.io/?EIO=4&transport=websocket",
        ];
        
        for url_str in fallback_urls {
            if let Ok(url) = Url::parse(url_str) {
                config.fallback_urls.push(url);
            }
        }
        
        config
    }
}

#[derive(Default)]
pub struct WebSocketConfigBuilder {
    config: WebSocketConfig,
}

impl WebSocketConfigBuilder {
    pub fn ping_interval(mut self, interval: Duration) -> Self {
        self.config.ping_interval = interval;
        self
    }
    
    pub fn ping_timeout(mut self, timeout: Duration) -> Self {
        self.config.ping_timeout = timeout;
        self
    }
    
    pub fn reconnect_delay(mut self, delay: Duration) -> Self {
        self.config.reconnect_delay = delay;
        self
    }
    
    pub fn max_reconnect_attempts(mut self, attempts: u32) -> Self {
        self.config.max_reconnect_attempts = attempts;
        self
    }
    
    pub fn batch_size(mut self, size: usize) -> Self {
        self.config.batch_size = size;
        self
    }
    
    pub fn batch_timeout(mut self, timeout: Duration) -> Self {
        self.config.batch_timeout = timeout;
        self
    }
    
    pub fn rate_limit(mut self, limit: Option<u32>) -> Self {
        self.config.rate_limit = limit;
        self
    }
    
    pub fn ssl_verify(mut self, verify: bool) -> Self {
        self.config.ssl_verify = verify;
        self
    }
    
    pub fn add_header(mut self, key: String, value: String) -> Self {
        self.config.custom_headers.insert(key, value);
        self
    }
    
    pub fn max_connections(mut self, max: usize) -> Self {
        self.config.max_connections = max;
        self
    }
    
    pub fn health_monitoring(mut self, enabled: bool) -> Self {
        self.config.enable_health_monitoring = enabled;
        self
    }
    
    pub fn event_system(mut self, enabled: bool) -> Self {
        self.config.enable_event_system = enabled;
        self
    }
    
    pub fn add_fallback_url(mut self, url: Url) -> Self {
        self.config.fallback_urls.push(url);
        self
    }
    
    pub fn build(self) -> WebSocketConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = WebSocketConfig::default();
        assert_eq!(config.ping_interval, DEFAULT_PING_INTERVAL);
        assert_eq!(config.batch_size, DEFAULT_BATCH_SIZE);
        assert!(config.enable_health_monitoring);
    }

    #[test]
    fn test_builder() {
        let config = WebSocketConfig::builder()
            .ping_interval(Duration::from_secs(30))
            .batch_size(20)
            .ssl_verify(true)
            .build();
            
        assert_eq!(config.ping_interval, Duration::from_secs(30));
        assert_eq!(config.batch_size, 20);
        assert!(config.ssl_verify);
    }

    #[test]
    fn test_pocketoption_config() {
        let config = WebSocketConfig::for_pocketoption();
        assert!(!config.ssl_verify);
        assert!(!config.fallback_urls.is_empty());
        assert_eq!(config.ping_interval, Duration::from_secs(20));
    }
}
