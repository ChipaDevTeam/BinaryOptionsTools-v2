use std::{
    collections::HashMap,
    fmt::{Debug, Display},
    hash::Hash,
    sync::Arc,
};

use async_channel::{bounded, Receiver, Sender};
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use tokio::sync::RwLock;

use crate::error::BinaryOptionsResult;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub enum EventType {
    Connected,
    Disconnected,
    Reconnected,
    MessageReceived,
    MessageSent,
    Error,
    Custom(String),
}

impl Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EventType::Connected => write!(f, "connected"),
            EventType::Disconnected => write!(f, "disconnected"),
            EventType::Reconnected => write!(f, "reconnected"),
            EventType::MessageReceived => write!(f, "message_received"),
            EventType::MessageSent => write!(f, "message_sent"),
            EventType::Error => write!(f, "error"),
            EventType::Custom(name) => write!(f, "{}", name),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Event<T = serde_json::Value>
where
    T: Clone + Send + Sync,
{
    pub event_type: EventType,
    pub data: T,
    pub timestamp: std::time::Instant,
    pub source: Option<String>,
}

impl<T> Event<T>
where
    T: Clone + Send + Sync,
{
    pub fn new(event_type: EventType, data: T) -> Self {
        Self {
            event_type,
            data,
            timestamp: std::time::Instant::now(),
            source: None,
        }
    }

    pub fn with_source(mut self, source: String) -> Self {
        self.source = Some(source);
        self
    }
}

#[async_trait]
pub trait EventHandler<T>: Send + Sync
where
    T: Clone + Send + Sync,
{
    async fn handle(&self, event: &Event<T>) -> BinaryOptionsResult<()>;

    /// Returns the name of the handler for identification
    fn name(&self) -> &str {
        "unnamed"
    }
}

// Convenience trait for closures
#[async_trait]
impl<T, F, Fut> EventHandler<T> for F
where
    T: Clone + Send + Sync + 'static,
    F: Fn(&Event<T>) -> Fut + Send + Sync,
    Fut: std::future::Future<Output = BinaryOptionsResult<()>> + Send,
{
    async fn handle(&self, event: &Event<T>) -> BinaryOptionsResult<()> {
        self(event).await
    }
}

pub struct EventManager<T = serde_json::Value>
where
    T: Clone + Send + Sync,
{
    handlers: Arc<RwLock<HashMap<EventType, Vec<Arc<dyn EventHandler<T>>>>>>,
    event_sender: Sender<Event<T>>,
    event_receiver: Receiver<Event<T>>,
    background_task: Option<tokio::task::JoinHandle<()>>,
}

impl<T> EventManager<T>
where
    T: Clone + Send + Sync + 'static,
{
    pub fn new(buffer_size: usize) -> Self {
        let (event_sender, event_receiver) = bounded(buffer_size);

        Self {
            handlers: Arc::new(RwLock::new(HashMap::new())),
            event_sender,
            event_receiver,
            background_task: None,
        }
    }

    pub async fn add_handler(&self, event_type: EventType, handler: Arc<dyn EventHandler<T>>) {
        let mut handlers = self.handlers.write().await;
        handlers.entry(event_type).or_default().push(handler);
    }

    pub async fn remove_handler(&self, event_type: &EventType, handler_id: usize) -> bool {
        let mut handlers = self.handlers.write().await;
        if let Some(handler_list) = handlers.get_mut(event_type) {
            if handler_id < handler_list.len() {
                handler_list.remove(handler_id);
                return true;
            }
        }
        false
    }

    pub async fn emit(&self, event: Event<T>) -> BinaryOptionsResult<()> {
        self.event_sender.send(event).await.map_err(|e| {
            crate::error::BinaryOptionsToolsError::GeneralMessageSendingError(e.to_string())
        })
    }

    pub fn start_background_processor(&mut self) {
        let handlers = self.handlers.clone();
        let receiver = self.event_receiver.clone();

        self.background_task = Some(tokio::spawn(async move {
            while let Ok(event) = receiver.recv().await {
                let handlers_guard = handlers.read().await;

                if let Some(event_handlers) = handlers_guard.get(&event.event_type) {
                    // Process handlers concurrently
                    let mut tasks = Vec::new();

                    for handler in event_handlers {
                        let handler = handler.clone();
                        let event = event.clone();
                        tasks.push(tokio::spawn(async move {
                            if let Err(e) = handler.handle(&event).await {
                                tracing::warn!("Event handler error: {}", e);
                            }
                        }));
                    }

                    // Wait for all handlers to complete
                    futures_util::future::join_all(tasks).await;
                }
            }
        }));
    }

    pub fn stop_background_processor(&mut self) {
        if let Some(task) = self.background_task.take() {
            task.abort();
        }
    }
}

impl<T> Drop for EventManager<T>
where
    T: Clone + Send + Sync,
{
    fn drop(&mut self) {
        self.stop_background_processor();
    }
}

// Specialized event manager for common use cases
pub type WebSocketEventManager = EventManager<serde_json::Value>;

// Helper macros for creating events
#[macro_export]
macro_rules! emit_event {
    ($manager:expr, $event_type:expr, $data:expr) => {
        $manager.emit(Event::new($event_type, $data)).await
    };
}

#[macro_export]
macro_rules! create_handler {
    ($handler:expr) => {
        Arc::new($handler) as Arc<dyn EventHandler<_>>
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_event_manager() {
        let mut manager = EventManager::<String>::new(100);
        let counter = Arc::new(AtomicUsize::new(0));

        let counter_clone = counter.clone();
        let handler = Arc::new(move |_event: &Event<String>| {
            let counter = counter_clone.clone();
            async move {
                counter.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }
        });

        manager.add_handler(EventType::Connected, handler).await;
        manager.start_background_processor();

        // Emit some events
        for i in 0..5 {
            manager
                .emit(Event::new(EventType::Connected, format!("test {}", i)))
                .await
                .unwrap();
        }

        // Wait for processing
        sleep(Duration::from_millis(100)).await;

        assert_eq!(counter.load(Ordering::SeqCst), 5);
    }
}
