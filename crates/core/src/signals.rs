use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::sync::watch;

/// Milliseconds since the Unix epoch.
///
/// A wall clock is used on purpose: the value is compared across threads and
/// only ever needs to answer "how long ago was the last frame", not to order
/// events precisely. `0` is reserved as the "never happened" sentinel, so a
/// clock that predates the epoch is clamped to `1` by the caller.
fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[derive(Clone, Debug)]
pub struct Signals {
    connected_watch: Arc<watch::Sender<bool>>,
    connected_receiver: watch::Receiver<bool>,
    /// Last inbound WebSocket frame, in Unix milliseconds (0 = never).
    ///
    /// Every frame counts, including Engine.IO pings: a frame arriving is the
    /// only proof that the socket is still being serviced, which is what makes
    /// a non-blocking liveness check possible.
    last_frame_ms: Arc<AtomicU64>,
}

impl Signals {
    pub fn new() -> Self {
        let (tx, rx) = watch::channel(false);
        Self {
            connected_watch: Arc::new(tx),
            connected_receiver: rx,
            last_frame_ms: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn set_connected(&self) {
        // Start the liveness clock at the moment the socket comes up so a
        // freshly connected client is not reported as silent before its first
        // frame arrives.
        self.mark_frame();
        let _ = self.connected_watch.send_replace(true);
    }

    pub fn set_disconnected(&self) {
        let _ = self.connected_watch.send_replace(false);
    }

    /// Records that a frame was received from the server.
    pub fn mark_frame(&self) {
        self.last_frame_ms
            .store(now_millis().max(1), Ordering::Relaxed);
    }

    /// Milliseconds since the last inbound frame, or `None` if none was ever
    /// received (the client has never connected).
    pub fn millis_since_last_frame(&self) -> Option<u64> {
        let last = self.last_frame_ms.load(Ordering::Relaxed);
        if last == 0 {
            return None;
        }
        Some(now_millis().saturating_sub(last))
    }

    pub fn is_connected(&self) -> bool {
        *self.connected_receiver.borrow()
    }

    pub async fn wait_connected(&self) {
        let mut rx = self.connected_receiver.clone();
        if *rx.borrow_and_update() {
            return;
        }
        while rx.changed().await.is_ok() {
            if *rx.borrow_and_update() {
                return;
            }
        }
    }

    pub async fn wait_disconnected(&self) {
        let mut rx = self.connected_receiver.clone();
        if !*rx.borrow_and_update() {
            return;
        }
        while rx.changed().await.is_ok() {
            if !*rx.borrow_and_update() {
                return;
            }
        }
    }
}

impl Default for Signals {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn frame_age_is_unknown_until_the_first_connection() {
        let signals = Signals::new();
        assert_eq!(signals.millis_since_last_frame(), None);
    }

    #[test]
    fn connecting_starts_the_liveness_clock() {
        let signals = Signals::new();
        signals.set_connected();
        let age = signals
            .millis_since_last_frame()
            .expect("connecting must seed the frame clock");
        assert!(
            age < 1_000,
            "a fresh connection must not look silent: {age} ms"
        );
    }

    #[test]
    fn marking_a_frame_keeps_the_age_small() {
        let signals = Signals::new();
        signals.set_connected();
        std::thread::sleep(Duration::from_millis(20));
        signals.mark_frame();
        let age = signals
            .millis_since_last_frame()
            .expect("the timestamp must exist");
        assert!(
            age < 1_000,
            "a just-received frame must look fresh: {age} ms"
        );
    }
}
