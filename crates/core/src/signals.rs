use std::sync::Arc;
use tokio::sync::watch;

#[derive(Clone, Debug)]
pub struct Signals {
    connected_watch: Arc<watch::Sender<bool>>,
    connected_receiver: watch::Receiver<bool>,
}

impl Signals {
    pub fn new() -> Self {
        let (tx, rx) = watch::channel(false);
        Self {
            connected_watch: Arc::new(tx),
            connected_receiver: rx,
        }
    }

    pub fn set_connected(&self) {
        let _ = self.connected_watch.send_replace(true);
    }

    pub fn set_disconnected(&self) {
        let _ = self.connected_watch.send_replace(false);
    }

    pub fn is_connected(&self) -> bool {
        *self.connected_receiver.borrow()
    }

    pub async fn wait_connected(&self) {
        let mut rx = self.connected_receiver.clone();
        if *rx.borrow() {
            return;
        }
        let _ = rx.changed().await;
    }

    pub async fn wait_disconnected(&self) {
        let mut rx = self.connected_receiver.clone();
        if !*rx.borrow() {
            return;
        }
        let _ = rx.changed().await;
    }
}

impl Default for Signals {
    fn default() -> Self {
        Self::new()
    }
}
