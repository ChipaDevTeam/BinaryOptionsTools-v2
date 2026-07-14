/// Message types for WebSocket communication.
pub enum Message {
    Text(String),
    Binary(Vec<u8>),
}
