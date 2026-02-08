use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    time::{Duration, Instant},
};

use async_trait::async_trait;
use tokio::{net::TcpStream, sync::Mutex, time::timeout};
use url::Url;

use crate::{
    error::{BinaryOptionsResult, BinaryOptionsToolsError},
    reimports::{MaybeTlsStream, WebSocketStream},
};

#[derive(Debug, Clone)]
pub struct ConnectionStats {
    pub response_times: VecDeque<Duration>,
    pub successes: u64,
    pub failures: u64,
    pub avg_response_time: Duration,
    pub success_rate: f64,
    pub last_used: Instant,
}

impl Default for ConnectionStats {
    fn default() -> Self {
        Self {
            response_times: VecDeque::with_capacity(100),
            successes: 0,
            failures: 0,
            avg_response_time: Duration::ZERO,
            success_rate: 0.0,
            last_used: Instant::now(),
        }
    }
}

impl ConnectionStats {
    pub fn update(&mut self, response_time: Duration, success: bool) {
        self.response_times.push_back(response_time);
        if self.response_times.len() > 100 {
            self.response_times.pop_front();
        }

        if success {
            self.successes += 1;
        } else {
            self.failures += 1;
        }

        self.avg_response_time =
            self.response_times.iter().sum::<Duration>() / self.response_times.len() as u32;

        let total = self.successes + self.failures;
        if total > 0 {
            self.success_rate = self.successes as f64 / total as f64;
        }

        self.last_used = Instant::now();
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub url: Url,
    pub connected_at: Instant,
    pub last_ping: Option<Instant>,
    pub is_healthy: bool,
    pub region: String,
}

pub struct ConnectionPool {
    connections: Arc<Mutex<HashMap<String, ConnectionInfo>>>,
    stats: Arc<Mutex<HashMap<String, ConnectionStats>>>,
    max_connections: usize,
}

impl ConnectionPool {
    pub fn new(max_connections: usize) -> Self {
        Self {
            connections: Arc::new(Mutex::new(HashMap::new())),
            stats: Arc::new(Mutex::new(HashMap::new())),
            max_connections,
        }
    }

    pub async fn get_best_url(&self) -> Option<String> {
        let stats = self.stats.lock().await;

        if stats.is_empty() {
            return None;
        }

        stats
            .iter()
            .min_by(|(_, a), (_, b)| {
                let a_score = a.avg_response_time.as_millis() as f64 / (a.success_rate + 0.1);
                let b_score = b.avg_response_time.as_millis() as f64 / (b.success_rate + 0.1);
                a_score
                    .partial_cmp(&b_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(url, _)| url.clone())
    }

    pub async fn update_stats(&self, url: &str, response_time: Duration, success: bool) {
        let mut stats = self.stats.lock().await;
        let entry = stats.entry(url.to_string()).or_default();
        entry.update(response_time, success);
    }

    pub async fn add_connection(
        &self,
        url: String,
        info: ConnectionInfo,
    ) -> BinaryOptionsResult<()> {
        let mut connections = self.connections.lock().await;

        if connections.len() >= self.max_connections {
            // Remove oldest connection
            if let Some((oldest_url, _)) = connections
                .iter()
                .min_by_key(|(_, info)| info.connected_at)
                .map(|(url, info)| (url.clone(), info.clone()))
            {
                connections.remove(&oldest_url);
            }
        }

        connections.insert(url, info);
        Ok(())
    }

    pub async fn get_stats(&self) -> HashMap<String, ConnectionStats> {
        self.stats.lock().await.clone()
    }
}

#[async_trait]
pub trait ConnectionManager: Send + Sync {
    async fn connect(
        &self,
        urls: &[Url],
    ) -> BinaryOptionsResult<(WebSocketStream<MaybeTlsStream<TcpStream>>, String)>;
    async fn test_connection(&self, url: &Url) -> BinaryOptionsResult<Duration>;
}

pub struct EnhancedConnectionManager {
    pool: ConnectionPool,
    connect_timeout: Duration,
    ssl_verify: bool,
}

impl EnhancedConnectionManager {
    pub fn new(max_connections: usize, connect_timeout: Duration, ssl_verify: bool) -> Self {
        Self {
            pool: ConnectionPool::new(max_connections),
            connect_timeout,
            ssl_verify,
        }
    }

    async fn try_connect_single(
        &self,
        url: &Url,
    ) -> BinaryOptionsResult<WebSocketStream<MaybeTlsStream<TcpStream>>> {
        use crate::reimports::{connect_async_tls_with_config, Connector};
        use tokio_tungstenite::tungstenite::http::Request;

        let request = Request::builder()
            .uri(url.as_str())
            .header("Origin", "https://pocketoption.com")
            .header(
                "User-Agent",
                "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
            )
            .header("Cache-Control", "no-cache")
            .body(())?;

        let connector = if self.ssl_verify {
            Connector::default()
        } else {
            Connector::default() // TODO: Configure for no SSL verification
        };

        let start = Instant::now();
        let result = timeout(
            self.connect_timeout,
            connect_async_tls_with_config(request, None, false, Some(connector)),
        )
        .await;

        match result {
            Ok(Ok((stream, _))) => {
                let response_time = start.elapsed();
                self.pool
                    .update_stats(url.as_str(), response_time, true)
                    .await;
                Ok(stream)
            }
            Ok(Err(e)) => {
                self.pool
                    .update_stats(url.as_str(), start.elapsed(), false)
                    .await;
                Err(BinaryOptionsToolsError::WebsocketConnectionError(e))
            }
            Err(_) => {
                self.pool
                    .update_stats(url.as_str(), self.connect_timeout, false)
                    .await;
                Err(BinaryOptionsToolsError::TimeoutError {
                    task: "Connection".to_string(),
                    duration: self.connect_timeout,
                })
            }
        }
    }
}

#[async_trait]
impl ConnectionManager for EnhancedConnectionManager {
    async fn connect(
        &self,
        urls: &[Url],
    ) -> BinaryOptionsResult<(WebSocketStream<MaybeTlsStream<TcpStream>>, String)> {
        // Try best URL first if available
        if let Some(best_url) = self.pool.get_best_url().await {
            if let Ok(url) = Url::parse(&best_url) {
                if let Ok(stream) = self.try_connect_single(&url).await {
                    return Ok((stream, best_url));
                }
            }
        }

        // Try all URLs in parallel
        let mut handles = Vec::new();
        for url in urls {
            let url = url.clone();
            let manager = self.clone();
            handles.push(tokio::spawn(async move {
                manager
                    .try_connect_single(&url)
                    .await
                    .map(|stream| (stream, url.to_string()))
            }));
        }

        // Wait for first successful connection
        while !handles.is_empty() {
            let (result, _index, remaining) = futures_util::future::select_all(handles).await;
            handles = remaining;

            match result {
                Ok(Ok((stream, url))) => {
                    // Cancel remaining attempts
                    for handle in handles {
                        handle.abort();
                    }
                    return Ok((stream, url));
                }
                Ok(Err(_)) => continue, // Try next connection
                Err(_) => continue,     // Handle join error
            }
        }

        Err(BinaryOptionsToolsError::WebsocketConnectionError(
            tokio_tungstenite::tungstenite::Error::ConnectionClosed,
        ))
    }

    async fn test_connection(&self, url: &Url) -> BinaryOptionsResult<Duration> {
        let start = Instant::now();
        self.try_connect_single(url).await?;
        Ok(start.elapsed())
    }
}

impl Clone for EnhancedConnectionManager {
    fn clone(&self) -> Self {
        Self {
            pool: ConnectionPool::new(self.pool.max_connections),
            connect_timeout: self.connect_timeout,
            ssl_verify: self.ssl_verify,
        }
    }
}
