---
sidebar_position: 1
slug: /architecture/structure
---

# System Architecture: Project Structure

This document provides an overview of the BinaryOptionsTools project structure.

## Repository Layout

```
BinaryOptionsTools-v2/
├── crates/
│   ├── binary_options_tools/     # Platform integrations (PocketOption, CloseOption, ExpertOption) and bot framework
│   ├── core/                     # Core framework (client, connector, router rules, modules, transport)
│   ├── bindings_uniffi/          # UniFFI bindings for multi-language support
│   ├── bindings_pyo3/            # PyO3 Python bindings
│   ├── bindings_napi/            # N-API (napi-rs) Node.js bindings
│   └── macros/                   # Procedural macros for the rule system
├── python/
│   └── BinaryOptionsToolsV2/     # Python package
├── nodejs/                       # Node.js package wrapping the N-API addon
├── examples/
│   ├── python/                   # Python examples (async & sync)
│   ├── rust/                     # Rust examples
│   ├── javascript/               # JavaScript/TypeScript examples
│   ├── swift/                    # Swift examples
│   ├── kotlin/                   # Kotlin examples
│   ├── go/                       # Go examples
│   ├── ruby/                     # Ruby examples
│   └── csharp/                   # C# examples
├── docs/                         # Documentation (MkDocs)
├── tests/                        # Integration tests
├── .github/workflows/            # CI/CD pipelines
└── Cargo.toml                    # Rust workspace root
```

## Crate Details

### `crates/binary_options_tools`

Platform integrations. Each platform is behind its own Cargo feature
(`pocketoption`, `closeoption`, `expertoption`, all enabled by default), so a
binding only compiles the platform it exposes.

```
src/
├── lib.rs                    # Main exports and feature gates
├── error.rs                  # Cross-platform error type
├── config.rs                 # Connection configuration
├── traits.rs                 # Validator trait
├── validator.rs              # Message validators
├── utils/
│   ├── mod.rs                # Shared helpers (message decoding, decimal conversion)
│   └── serialize.rs          # Custom serde helpers
├── framework/                # Bot framework (strategy, market, backtesting)
├── pocketoption/
│   ├── mod.rs                # PocketOption exports
│   ├── pocket_client.rs      # PocketOption client
│   ├── connect.rs            # Connection logic
│   ├── candle.rs             # Candle compilation
│   ├── types.rs              # Shared types and rules
│   ├── utils.rs              # Connection helpers and message parsing
│   ├── state.rs              # Connection state
│   ├── ssid.rs               # SSID handling
│   ├── error.rs              # PocketError
│   └── modules/              # ApiModule / LightweightModule implementations
│       ├── mod.rs
│       ├── assets.rs         # Assets module
│       ├── balance.rs        # Balance module
│       ├── deals.rs          # Deal history
│       ├── get_candles.rs    # Historical candles
│       ├── historical_data.rs# Historical data
│       ├── keep_alive.rs     # Keep-alive and initialization
│       ├── pending_trades.rs # Pending orders
│       ├── raw.rs            # Raw message handler
│       ├── server_time.rs    # Server time
│       ├── subscriptions.rs  # Real-time subscriptions
│       └── trades.rs         # Trade management
├── closeoption/
│   ├── mod.rs                # CloseOption exports
│   ├── client.rs             # CloseOption client
│   ├── connect.rs            # Connection logic
│   ├── types.rs              # Types and Socket.IO framing
│   ├── utils.rs              # Connection helpers
│   ├── state.rs              # Connection state
│   ├── error.rs              # CloseOptionError
│   └── ...                   # (see `src/closeoption`)
└── expertoptions/
    └── ...                   # ExpertOption client and types
```

### `crates/core`

Core framework for building trading clients.

```
src/
├── lib.rs                    # Main exports
├── builder.rs                # ClientBuilder for module registration
├── client.rs                 # Client and Runner implementation
├── connector.rs              # Connector trait and WebSocket stream types
├── transport.rs              # Shared TLS config, SOCKS5 / HTTP CONNECT, timestamp normalization
├── middleware.rs             # Middleware stack
├── traits.rs                 # Core traits (ApiModule, LightweightModule, etc.)
├── message.rs                # Message types
├── statistics.rs             # Statistics tracking
├── rules.rs                  # Rule system
├── callback.rs               # Lightweight handler callbacks
├── signals.rs                # Signal handling
├── testing.rs                # Test utilities
└── utils/
    ├── mod.rs                # Shared core utilities
    ├── stream.rs             # Stream utilities
    ├── time.rs               # Time helpers
    └── tracing.rs            # Tracing layers
```

### `crates/bindings_uniffi`

UniFFI bindings for generating multi-language APIs.

```
src/
├── lib.rs                    # Main exports
├── error.rs                  # Error mapping
├── tracing.rs                # Tracing initialization
├── utils.rs                  # Utility functions
├── test.rs                   # Test utilities
└── platforms/pocketoption/
    ├── mod.rs                # Platform exports
    ├── client.rs             # PocketOption client wrapper
    ├── validator.rs          # Validator wrapper
    ├── raw_handler.rs        # Raw handler wrapper
    ├── stream.rs             # Stream wrapper
    └── types.rs              # Type conversions
```

### `crates/bindings_pyo3`

PyO3 Python bindings for native Python performance.

```
src/
├── lib.rs                    # Python module entry
├── pocketoption.rs           # PocketOption Python class
├── framework.rs              # PyStrategy framework
├── validator.rs              # Validator Python class
├── config.rs                 # PyConfig
├── error.rs                  # Error mapping
├── runtime.rs                # Async runtime bridge
├── stream.rs                 # Stream wrapper
└── logs.rs                   # Logging setup
```

### `crates/macros`

Procedural macros for the rule system.

```
src/
├── lib.rs                    # Macro exports
├── region.rs                 # Region macro
└── doc.rs                    # Documentation macro
```

## Module Architecture

### ApiModule

Full-featured module with commands, responses, and a Handle.

```
UserCode --> Handle --> CommandReceiver --> Module.run()
                    ^                          |
                    |                          v
              CommandResponder <-- Response <--+
```

### LightweightModule

Background task receiving routed WS messages, no command/response.

```
Router rules --> WS Msg Receiver --> Module.run()
```

### Lightweight Handler

Global stateless callback receiving every WS message.

```
Router -- every msg --> Handler.callback()
```

## Data Flow

```
WebSocket --> Connector --> Runner --> Middleware (inbound) --> Router rules
                                                                    |
                                        +---------------------------+---------------------------+
                                        |                           |                           |
                                   LW Handlers                 LW Modules                  ApiModules
                                        |                           |                           |
                                        v                           v                           v
                                   Callbacks                 Background Tasks            Commands + Responses
```

See [Data Flow](/architecture/dataflow) for detailed diagrams.

## Build System

- **Rust**: Cargo workspace with multiple crates
- **Python**: PyO3 + maturin for native bindings
- **UniFFI**: Generates bindings for Kotlin, Swift, Go, Ruby, C#
- **Node.js**: napi-rs native addon (`crates/bindings_napi`), packaged in `nodejs/`

## Testing

- Unit tests in each crate (`#[cfg(test)]`)
- Integration tests in each crate's `tests/` directory
- Python tests in `tests/python/`
- Node.js smoke tests in `nodejs/test/` (`npm test`)
- Examples serve as functional tests
