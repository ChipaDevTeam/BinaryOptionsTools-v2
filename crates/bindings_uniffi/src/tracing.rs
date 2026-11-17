use std::str::FromStr;

use tracing_subscriber::{
    filter::Targets, layer::SubscriberExt, util::SubscriberInitExt,
};

/// Initialize logging for the UniFFI bindings.
///
/// Honours `RUST_LOG` directives such as `warn` or `warn,binary_options_tools=debug`.
/// `filter::Targets` parses those without the `env-filter` feature, whose matcher
/// pulls regex-automata into every crate that depends on the subscriber.
///
/// With `RUST_LOG` unset nothing is enabled, which is what `EnvFilter`'s default
/// did (no directive added, base level off): the bindings stay silent until the
/// host opts in. Span-field directives (`[span{field=x}]=debug`) are not part of
/// the `Targets` syntax and are ignored.
pub fn init_tracing() {
    let targets = std::env::var("RUST_LOG")
        .ok()
        .and_then(|value| Targets::from_str(&value).ok())
        .unwrap_or_default();

    let _ = tracing_subscriber::registry()
        .with(targets)
        .with(tracing_subscriber::fmt::layer())
        .try_init();
}
