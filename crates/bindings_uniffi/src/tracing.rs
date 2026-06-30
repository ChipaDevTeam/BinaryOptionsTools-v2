/// Initialize logging for the UniFFI bindings.
///
/// Uses the `tracing` crate's default subscriber.
/// Call this once at application startup.
pub fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
}
