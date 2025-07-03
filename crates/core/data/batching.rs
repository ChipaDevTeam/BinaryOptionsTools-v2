use std::{
    collections::VecDeque,
    sync::Arc,
    time::{Duration, Instant},
};

use async_channel::{Receiver, Sender, bounded};
use tokio::{
    sync::Mutex,
    time::{interval, sleep},
};
use tokio_tungstenite::tungstenite::Message;

use crate::error::{BinaryOptionsResult, BinaryOptionsToolsError};

#[derive(Debug, Clone)]
pub struct BatchingConfig {
    pub batch_size: usize,
    pub batch_timeout: Duration,
    pub max_pending: usize,
    pub rate_limit: Option<u32>, // Messages per second
}

impl Default for BatchingConfig {
    fn default() -> Self {
        Self {
            batch_size: 10,
            batch_timeout: Duration::from_millis(100),
            max_pending: 1000,
            rate_limit: Some(100),
        }
    }
}

pub struct MessageBatcher {
    config: BatchingConfig,
    pending_messages: Arc<Mutex<VecDeque<Message>>>,
    last_batch_time: Arc<Mutex<Instant>>,
    batch_sender: Sender<Vec<Message>>,
    batch_receiver: Receiver<Vec<Message>>,
}

impl MessageBatcher {
    pub fn new(config: BatchingConfig) -> Self {
        let (batch_sender, batch_receiver) = bounded(config.max_pending / config.batch_size);

        Self {
            config,
            pending_messages: Arc::new(Mutex::new(VecDeque::new())),
            last_batch_time: Arc::new(Mutex::new(Instant::now())),
            batch_sender,
            batch_receiver,
        }
    }

    pub async fn add_message(&self, message: Message) -> BinaryOptionsResult<()> {
        let mut pending = self.pending_messages.lock().await;

        if pending.len() >= self.config.max_pending {
            return Err(BinaryOptionsToolsError::GeneralMessageSendingError(
                "Message queue is full".to_string(),
            ));
        }

        pending.push_back(message);

        // Check if we should flush immediately
        if pending.len() >= self.config.batch_size {
            self.flush_batch_internal(&mut pending).await?;
        } else {
            // Check timeout
            let last_batch = *self.last_batch_time.lock().await;
            if last_batch.elapsed() >= self.config.batch_timeout {
                self.flush_batch_internal(&mut pending).await?;
            }
        }

        Ok(())
    }

    async fn flush_batch_internal(
        &self,
        pending: &mut VecDeque<Message>,
    ) -> BinaryOptionsResult<()> {
        if pending.is_empty() {
            return Ok(());
        }

        let batch: Vec<Message> = pending.drain(..).collect();
        *self.last_batch_time.lock().await = Instant::now();

        self.batch_sender
            .send(batch)
            .await
            .map_err(|e| BinaryOptionsToolsError::GeneralMessageSendingError(e.to_string()))?;

        Ok(())
    }

    pub async fn flush_batch(&self) -> BinaryOptionsResult<()> {
        let mut pending = self.pending_messages.lock().await;
        self.flush_batch_internal(&mut pending).await
    }

    pub fn get_batch_receiver(&self) -> Receiver<Vec<Message>> {
        self.batch_receiver.clone()
    }

    pub async fn start_background_flusher(&self) -> tokio::task::JoinHandle<()> {
        let pending = self.pending_messages.clone();
        let last_batch_time = self.last_batch_time.clone();
        let sender = self.batch_sender.clone();
        let timeout = self.config.batch_timeout;

        tokio::spawn(async move {
            let mut interval = interval(timeout / 2); // Check twice as often as timeout

            loop {
                interval.tick().await;

                let mut pending_guard = pending.lock().await;
                if !pending_guard.is_empty() {
                    let last_batch = *last_batch_time.lock().await;
                    if last_batch.elapsed() >= timeout {
                        let batch: Vec<Message> = pending_guard.drain(..).collect();
                        *last_batch_time.lock().await = Instant::now();

                        if sender.send(batch).await.is_err() {
                            break; // Channel closed
                        }
                    }
                }
            }
        })
    }
}

pub struct RateLimiter {
    rate: u32, // Messages per second
    tokens: Arc<Mutex<u32>>,
    last_refill: Arc<Mutex<Instant>>,
}

impl RateLimiter {
    pub fn new(rate: u32) -> Self {
        Self {
            rate,
            tokens: Arc::new(Mutex::new(rate)),
            last_refill: Arc::new(Mutex::new(Instant::now())),
        }
    }

    pub async fn acquire(&self) -> BinaryOptionsResult<()> {
        loop {
            {
                let mut tokens = self.tokens.lock().await;
                let mut last_refill = self.last_refill.lock().await;

                // Refill tokens based on elapsed time
                let elapsed = last_refill.elapsed();
                let tokens_to_add = (elapsed.as_secs_f64() * self.rate as f64) as u32;

                if tokens_to_add > 0 {
                    *tokens = (*tokens + tokens_to_add).min(self.rate);
                    *last_refill = Instant::now();
                }

                if *tokens > 0 {
                    *tokens -= 1;
                    return Ok(());
                }
            }

            // Wait a bit before trying again
            sleep(Duration::from_millis(10)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{Duration, sleep};

    #[tokio::test]
    async fn test_message_batcher() {
        let config = BatchingConfig {
            batch_size: 3,
            batch_timeout: Duration::from_millis(100),
            max_pending: 100,
            rate_limit: None,
        };

        let batcher = MessageBatcher::new(config);
        let receiver = batcher.get_batch_receiver();

        // Add messages one by one
        batcher.add_message(Message::text("msg1")).await.unwrap();
        batcher.add_message(Message::text("msg2")).await.unwrap();
        batcher.add_message(Message::text("msg3")).await.unwrap(); // Should trigger batch

        let batch = receiver.recv().await.unwrap();
        assert_eq!(batch.len(), 3);
    }

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new(2); // 2 messages per second

        let start = Instant::now();

        limiter.acquire().await.unwrap(); // Should be immediate
        limiter.acquire().await.unwrap(); // Should be immediate
        limiter.acquire().await.unwrap(); // Should wait

        assert!(start.elapsed() >= Duration::from_millis(500));
    }
}
