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
│   ├── binary_options_tools/     # Core Rust crate (PocketOption integration)
│   ├── core/                     # Core framework (client, runner, router, modules)
│   ├── bindings_uniffi/          # UniFFI bindings for multi-language support
│   ├── bindings_pyo3/            # PyO3 Python bindings
│   └── macros/                   # Procedural macros for rule system
├── python/
│   └── BinaryOptionsToolsV2/     # Python package
├── examples/
│   ├── python/                   # Python examples (async & sync)
│   ├── rust/                     # Rust examples
│   ├── javascript/               # JavaScript/TypeScript examples
│   ├── swift/                    # Swift examples
│   ├── kotlin/                   # Kotlin examples
│   ├── go/                       # Go examples
│   ├── ruby/                     # Ruby examples
│   └── csharp/                   # C# examples
├── docs/                         # Documentation (Docusaurus)
├── tests/                        # Integration tests
├── .github/github/github/workflows/           # CI/CD pipelines
└── Cargo.toml                    # Rust workspace root
```

## Crate Details

### `crates/binary_options_tools`

Main PocketOption integration crate.

```
src/
├── lib.rs                    # Main exports
├── error.rs                  # Error types
├── pocket_client.rs          # PocketOption client implementation
├── candle.rs                 # Candle compilation logic
├── types.rs                  # Shared types
├── utils.rs                  # Utility functions
├── state.rs                  # Connection state
├── ssid.rs                   # SSID handling
├── connect.rs                # Connection logic
└── modules/
    ├── mod.rs                # Module exports
    ├── subscriptions.rs      # Real-time subscriptions
    ├── trades.rs             # Trade management
    ├── deals.rs              # Deal history
    ├── get_candles.rs        # Historical candles
    ├── historical_data.rs    # Historical data module
    ├── pending_trades.rs     # Pending orders
    ├── raw.rs                # Raw message handler
    ├── balance.rs            # Balance module
    ├── assets.rs             # Assets module
    ├── server_time.rs        # Server time
    └── keep_alive.rs         # Keep-alive module
```

### `crates/core`

Core framework for building trading clients.

```
src/
├── lib.rs                    # Main exports
├── builder.rs                # ClientBuilder for module registration
├── client.rs                 # Client and Runner implementation
├── router.rs                 # Message routing
├── middleware.rs             # Middleware stack
├── traits.rs                 # Core traits (ApiModule, LightweightModule, etc.)
├── message.rs                # Message types
├── statistics.rs             # Statistics tracking
├── rules.rs                  # Rule system
├── signals.rs                # Signal handling
└── utils/
    └── stream.rs             # Stream utilities
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
├── error.rs                  # Error mapping
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
Router -- rule --> WS Msg Receiver --> Module.run()
```

### Lightweight Handler

Global stateless callback receiving every WS message.

```
Router -- every msg --> Handler.callback()
```

## Data Flow

```
WebSocket --> Connector --> Runner --> Middleware (inbound) --> Router
                                                              |
                                        +---------------------+---------------------+
                                        |                     |                     |
                                   LW Handlers          LW Modules             ApiModules
                                        |                     |                     |
                                        v                     v                     v
                                   Callbacks           Background           Commands + Responses
                                                                   Task
```

See [Data Flow](/architecture/dataflow) for detailed diagrams.

## Build System

- **Rust**: Cargo workspace with multiple crates
- **Python**: PyO3 + maturin for native bindings
- **UniFFI**: Generates bindings for Kotlin, Swift, Go, Ruby, C#
- **JavaScript**: wasm-bindgen or napi-rs for Node.js native module

## Testing

- Unit tests in each crate (`#[cfg(test)]`)
- Integration tests in `tests/` directory
- Python tests in `tests/python/`
- Examples serve as functional tests