use std::sync::Arc;

use binary_options_tools_core::{
    error::CoreResult,
    middleware::{MiddlewareContext, WebSocketMiddleware},
    reimports::Message,
    traits::AppState,
};
use rust_decimal::Decimal;
use std::str::FromStr;

pub mod serialize;

/// Installs the process-wide rustls crypto provider.
///
/// Thin alias for the shared transport implementation so every platform in the
/// crate agrees on a single provider (rustls only accepts one per process).
pub use binary_options_tools_core::transport::init_crypto_provider;

/// Lightweight message printer for debugging purposes
///
/// This handler logs all incoming WebSocket messages for debugging
/// and development purposes. It can be useful for understanding
/// the message flow and troubleshooting connection issues.
///
/// # Usage
///
/// This is typically used during development to monitor all WebSocket
/// traffic. It should be disabled in production due to performance
/// and log volume concerns.
///
/// # Arguments
/// * `msg` - WebSocket message to log
///
/// # Returns
/// Always returns Ok(())
///
/// # Examples
///
/// ```rust,ignore
/// // Add as a lightweight handler to the client
/// client.with_lightweight_handler(|msg, _, _| Box::pin(print_handler(msg)));
/// ```
pub async fn print_handler(msg: Arc<Message>) -> CoreResult<()> {
    tracing::debug!(target: "Lightweight", "Received: {msg:?}");
    Ok(())
}

pub struct PrintMiddleware;

#[async_trait::async_trait]
impl<S: AppState> WebSocketMiddleware<S> for PrintMiddleware {
    async fn on_send(&self, message: &Message, _context: &MiddlewareContext<S>) -> CoreResult<()> {
        // Default implementation does nothing

        tracing::debug!(target: "Middleware", "Sending: {message:?}");
        Ok(())
    }

    async fn on_receive(
        &self,
        message: &Message,
        _context: &MiddlewareContext<S>,
    ) -> CoreResult<()> {
        // Default implementation does nothing
        tracing::debug!(target: "Middleware", "Receiving: {message:?}");
        Ok(())
    }
}

/// Converts a WebSocket message into its UTF-8 payload.
///
/// Text frames are returned verbatim, binary frames are decoded lossily, and
/// any other frame type (ping/pong/close) yields an empty string. Every
/// language binding used to carry its own copy of this match; this is the
/// single implementation they now share so the decoding rules cannot drift.
pub fn message_to_string(message: &Message) -> String {
    match message {
        Message::Text(text) => text.to_string(),
        Message::Binary(data) => String::from_utf8_lossy(data.as_ref()).into_owned(),
        _ => String::new(),
    }
}

/// Converts an `Arc<Message>` into its UTF-8 payload.
///
/// Convenience wrapper around [`message_to_string`] for the messages the core
/// hands to module and binding streams.
pub fn arc_message_to_string(message: &Arc<Message>) -> String {
    message_to_string(message.as_ref())
}

/// Converts an f64 to Decimal with exact precision.
///
/// Uses the `ryu` algorithm to produce the shortest decimal string
/// that exactly represents the f64 value, then parses it to Decimal.
/// This handles scientific notation correctly and avoids precision loss.
///
/// # Arguments
/// * `value` - The f64 value to convert
///
/// # Returns
/// `Some(Decimal)` if conversion succeeded, `None` if the value is NaN or infinite
pub fn f64_to_decimal(value: f64) -> Option<Decimal> {
    if !value.is_finite() {
        return None;
    }
    // Use ryu's buffer to get the shortest exact representation
    let mut buffer = ryu::Buffer::new();
    let formatted = buffer.format_finite(value);
    Decimal::from_str(formatted).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::prelude::{FromPrimitive, ToPrimitive};

    #[test]
    fn test_f64_to_decimal_basic() {
        assert_eq!(f64_to_decimal(1.5), Some(Decimal::from_f64(1.5).unwrap()));
        assert_eq!(
            f64_to_decimal(122.24),
            Some(Decimal::from_f64(122.24).unwrap())
        );
        assert_eq!(f64_to_decimal(0.0), Some(Decimal::from_f64(0.0).unwrap()));
        assert_eq!(
            f64_to_decimal(-5.75),
            Some(Decimal::from_f64(-5.75).unwrap())
        );
    }

    #[test]
    fn test_f64_to_decimal_scientific() {
        // Test scientific notation values
        // The key is that the conversion is exact and round-trips correctly
        let value = 1.770706e+09;
        let result = f64_to_decimal(value).unwrap();
        // Should convert to 1770706000 exactly
        assert_eq!(result, Decimal::from_u32(1770706000).unwrap());

        // Test another scientific notation value
        let value2 = 1.23e+05;
        let result2 = f64_to_decimal(value2).unwrap();
        assert_eq!(result2, Decimal::from_u32(123000).unwrap());

        // Test that the conversion round-trips correctly
        let round_trip = result.to_f64().unwrap();
        assert_eq!(round_trip, value);
    }

    #[test]
    fn test_message_to_string_decoding_rules() {
        assert_eq!(message_to_string(&Message::Text("hello".into())), "hello");
        assert_eq!(
            message_to_string(&Message::Binary(vec![b'h', b'i'].into())),
            "hi"
        );
        // Invalid UTF-8 is replaced rather than causing an error.
        assert_eq!(
            message_to_string(&Message::Binary(vec![0xff].into())),
            "\u{fffd}"
        );
        // Non-data frames have no payload representation.
        assert_eq!(message_to_string(&Message::Ping(Vec::new().into())), "");
        assert_eq!(message_to_string(&Message::Pong(Vec::new().into())), "");
        assert_eq!(message_to_string(&Message::Close(None)), "");
        assert_eq!(
            arc_message_to_string(&Arc::new(Message::Text("arc".into()))),
            "arc"
        );
    }

    #[test]
    fn test_f64_to_decimal_invalid() {
        assert_eq!(f64_to_decimal(f64::NAN), None);
        assert_eq!(f64_to_decimal(f64::INFINITY), None);
        assert_eq!(f64_to_decimal(f64::NEG_INFINITY), None);
    }

    #[test]
    fn test_f64_to_decimal_extreme_precision() {
        // Test a value that exceeds Decimal's 28-digit precision
        let extreme_small = 1e-31f64;
        let result = f64_to_decimal(extreme_small);
        // Decimal should return None or a rounded value depending on implementation,
        // but it must not crash.
        assert!(result.is_none() || result.unwrap().is_zero());
    }
}
