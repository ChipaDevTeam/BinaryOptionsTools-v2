use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Notify;

#[derive(Clone, Default, Debug)]
pub struct Signals {
    is_connected: Arc<AtomicBool>,
    connected_notify: Arc<Notify>,
    disconnected_notify: Arc<Notify>,
}

impl Signals {
    /// Call this when a connection is established.
    pub fn set_connected(&self) {
        self.is_connected.store(true, Ordering::SeqCst);
        self.connected_notify.notify_waiters();
    }

    /// Call this when a disconnection occurs.
    pub fn set_disconnected(&self) {
        self.is_connected.store(false, Ordering::SeqCst);
        self.disconnected_notify.notify_waiters();
    }

    /// Check current connection state.
    pub fn is_connected(&self) -> bool {
        self.is_connected.load(Ordering::SeqCst)
    }

    /// Wait for the next connection event.
    pub async fn wait_connected(&self) {
        // Only wait if not already connected
        if !self.is_connected() {
            self.connected_notify.notified().await;
        }
    }

    /// Wait for the next disconnection event.
    pub async fn wait_disconnected(&self) {
        // Only wait if currently connected
        if self.is_connected() {
            self.disconnected_notify.notified().await;
        }
    }
}
