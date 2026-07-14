# BinaryOptionsTools-v2 — Codemap

**High-performance binary options trading automation library.** Python-first with Rust core via PyO3. Supports PocketOption (primary) and ExpertOption platforms. Provides async/sync Python clients, real-time data streaming, automated trading strategies, and raw WebSocket API access.

- **Version:** 0.2.12
- **Repo:** <https://gitlab.chipatrade.com/chipadevorg/BinaryOptionsTools-v2>

---

## Architecture Overview

| Layer                      | Path                                                         | Description                                                                                                                                                                        |
| -------------------------- | ------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Python SDK                 | [python/BinaryOptionsToolsV2/](python/BinaryOptionsToolsV2/) | User-facing Python API. Two entrypoints: synchronous (`PocketOption`) and asynchronous (`PocketOptionAsync`). Wraps Rust FFI via pyo3.                                             |
| PyO3 Bindings              | [crates/bindings_pyo3/src/](crates/bindings_pyo3/src/)       | Rust crate compiled as cdylib. Bridges Python ↔ Rust via pyo3 and pyo3-async-runtimes. Exposes `RawPocketOption`, `RawValidator`, `PyBot`, `PyStrategy`, `PyConfig`, `Logger` etc. |
| Binary Options Tools Crate | [crates/binary_options_tools/](crates/binary_options_tools/) | High-level Rust library. Platform client implementations (PocketOption, ExpertOption), config, validator, framework (Bot/Strategy/Market), all platform-specific modules.          |
| Core Crate                 | [crates/core/](crates/core/)                                 | Low-level WebSocket client framework. Connection lifecycle (`ClientRunner`), message routing (`Router`), middleware stack, signals, testing utilities, stream utilities.           |
| Macros Crate               | [crates/macros/](crates/macros/)                             | Proc macros for serialization (`serialize!`/`deserialize!`), timeout, action, config, region, lightweight_module generation.                                                       |
| UniFFI Bindings            | [crates/bindings_uniffi/](crates/bindings_uniffi/)           | Experimental UniFFI bindings for multi-language support (Kotlin, Swift, Go, Python, C#, Ruby, JS). Shares `binary_options_tools` as dependency.                                    |

---

## Source Tree

### Python SDK — [python/BinaryOptionsToolsV2/](python/BinaryOptionsToolsV2/)

| File                                                                                     | Description                                                                                                                                                                                                                                               |
| ---------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [**init**.py](python/BinaryOptionsToolsV2/__init__.py)                                   | Package entry. Imports Rust cdylib, re-exports all PyO3 classes. Sub-modules: config, tracing, validator, pocketoption.                                                                                                                                   |
| [config.py](python/BinaryOptionsToolsV2/config.py)                                       | Python Config dataclass. Wraps `PyConfig` (Rust). Lock-on-use pattern. Supports `from_dict`/`from_json`. Fields: `max_allowed_loops`, `sleep_interval`, `reconnect_time`, `urls`, `max_subscriptions`, `terminal_logging`, `log_level`, `extra_duration`. |
| [tracing.py](python/BinaryOptionsToolsV2/tracing.py)                                     | `Logger`, `LogBuilder`, `StreamLogsIterator`. Wraps Rust Logger/LogBuilder.                                                                                                                                                                               |
| [validator.py](python/BinaryOptionsToolsV2/validator.py)                                 | `Validator` class (high-level). Wraps `RawValidator` (Rust). Static methods: `regex`, `starts_with`, `ends_with`, `contains`, `ne` (not), `all`, `any`, `custom(func)`.                                                                                   |
| [pocketoption/\_\_init\_\_.py](python/BinaryOptionsToolsV2/pocketoption/__init__.py)     | Re-exports `PocketOptionAsync`, `PocketOption`, `RawHandler`, `RawHandlerSync`.                                                                                                                                                                           |
| [pocketoption/asynchronous.py](python/BinaryOptionsToolsV2/pocketoption/asynchronous.py) | `PocketOptionAsync` class. Async context manager. Full API surface including `buy`/`sell`, `subscribe_symbol*` (4 variants), `create_raw_handler`, `create_raw_order*` (3 variants).                                                                      |
| [pocketoption/synchronous.py](python/BinaryOptionsToolsV2/pocketoption/synchronous.py)   | `PocketOption` class. Creates new event loop, wraps all `PocketOptionAsync` methods via `run_until_complete`. Thread-safe with `RLock`.                                                                                                                   |

### PyO3 Bindings — [crates/bindings_pyo3/src/](crates/bindings_pyo3/src/)

| File                                                        | Description                                                                              |
| ----------------------------------------------------------- | ---------------------------------------------------------------------------------------- |
| [lib.rs](crates/bindings_pyo3/src/lib.rs)                   | PyO3 module entry. Registers all classes and functions with `#[pymodule]`.               |
| [pocketoption.rs](crates/bindings_pyo3/src/pocketoption.rs) | `RawPocketOption` (~1123 lines). Core PyO3 class. Methods mirror Python API.             |
| [framework.rs](crates/bindings_pyo3/src/framework.rs)       | `PyStrategy` (subclassable), `StrategyWrapper`, `PyContext`, `PyVirtualMarket`, `PyBot`. |
| [config.rs](crates/bindings_pyo3/src/config.rs)             | `PyConfig` — wraps `binary_options_tools::config::Config`.                               |
| [validator.rs](crates/bindings_pyo3/src/validator.rs)       | `RawValidator` enum. Converts to `CrateValidator`.                                       |
| [error.rs](crates/bindings_pyo3/src/error.rs)               | `BinaryErrorPy` enum. Converts from `BinaryOptionsError`, `PocketError`.                 |
| [runtime.rs](crates/bindings_pyo3/src/runtime.rs)           | Global tokio Runtime singleton via `PyOnceLock`.                                         |
| [stream.rs](crates/bindings_pyo3/src/stream.rs)             | `next_stream` helper for async/sync iteration.                                           |
| [logs.rs](crates/bindings_pyo3/src/logs.rs)                 | `Logger`, `LogBuilder`, `StreamLogsIterator`, `StreamLogsLayer`.                         |

### Core Crate — [crates/core/src/](crates/core/src/)

| File/Dir                                             | Description                                                                                                                                             |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [lib.rs](crates/core/src/lib.rs)                     | Re-exports `core_macros::rule` as `Rule`.                                                                                                               |
| [client.rs](crates/core/src/client.rs)               | `Client<S: AppState>` — public handle. `Router<S>` — message routing with middleware. `ClientRunner<S>` — WebSocket lifecycle with exponential backoff. |
| [connector.rs](crates/core/src/connector.rs)         | `Connector<S>` trait. `ConnectorError`.                                                                                                                 |
| [traits.rs](crates/core/src/traits.rs)               | Core traits: `AppState`, `ApiModule<S>`, `Rule`, `ReconnectCallback`, `RunnerCommand`.                                                                  |
| [middleware.rs](crates/core/src/middleware.rs)       | `MiddlewareStack<S>` with hooks: `on_connect`, `on_disconnect`, `on_send`, `on_receive`, `record_connection_attempt`.                                   |
| [testing.rs](crates/core/src/testing.rs)             | `TestingWrapper` and `TestingWrapperBuilder` for mocking WebSocket streams.                                                                             |
| [signals.rs](crates/core/src/signals.rs)             | `Signals` — connected/disconnected state notification.                                                                                                  |
| [builder.rs](crates/core/src/builder.rs)             | `ClientBuilder` for constructing `Client` + `ClientRunner`.                                                                                             |
| [utils/stream.rs](crates/core/src/utils/stream.rs)   | `ReceiverStream` — wraps kanal receiver as `Stream`.                                                                                                    |
| [utils/tracing.rs](crates/core/src/utils/tracing.rs) | `stream_logs_layer` — tracing subscriber layer for log streaming.                                                                                       |

### Binary Options Tools Crate — [crates/binary_options_tools/src/](crates/binary_options_tools/src/)

| Path                                                                                           | Description                                                                                                                                                   |
| ---------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [lib.rs](crates/binary_options_tools/src/lib.rs)                                               | Public modules: config, error, expertoptions, framework, pocketoption, reimports, traits, utils, validator.                                                   |
| [config.rs](crates/binary_options_tools/src/config.rs)                                         | `Config` struct — `max_allowed_loops`, `sleep_interval`, `reconnect_time`, `timeout`, `urls`, `max_subscriptions` etc.                                        |
| [validator.rs](crates/binary_options_tools/src/validator.rs)                                   | `CrateValidator` enum implementing `ValidatorTrait`.                                                                                                          |
| [pocketoption/pocket_client.rs](crates/binary_options_tools/src/pocketoption/pocket_client.rs) | `PocketOption` (~1430 lines). Main client. All trading operations.                                                                                            |
| [pocketoption/modules/](crates/binary_options_tools/src/pocketoption/modules/)                 | API modules: `keep_alive`, `balance`, `server_time`, `subscriptions`, `trades`, `deals`, `assets`, `get_candles`, `historical_data`, `pending_trades`, `raw`. |
| [pocketoption/candle.rs](crates/binary_options_tools/src/pocketoption/candle.rs)               | `Candle`, `SubscriptionType` (none/chunk/time/time_aligned).                                                                                                  |
| [pocketoption/ssid.rs](crates/binary_options_tools/src/pocketoption/ssid.rs)                   | SSID parsing and validation.                                                                                                                                  |
| [pocketoption/types.rs](crates/binary_options_tools/src/pocketoption/types.rs)                 | `Action`, `Assets`, `Asset`, `Deal`, `Candle`, `PendingOrder`.                                                                                                |
| [framework/](crates/binary_options_tools/src/framework/)                                       | `Context`, `Strategy` trait, `Bot`, `VirtualMarket`.                                                                                                          |
| [expertoptions/](crates/binary_options_tools/src/expertoptions/)                               | ExpertOption platform integration (placeholder/stub).                                                                                                         |

### Macros Crate — [crates/macros/src/](crates/macros/src/)

| File                                               | Description                                                                                             |
| -------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| [lib.rs](crates/macros/src/lib.rs)                 | Proc macros: `impl_module!`, `impl_config!`, `action`, `region`, `serialize`, `deserialize`, `timeout`. |
| [action.rs](crates/macros/src/action.rs)           | `Action` derive macro — implements `ActionName` trait + generates `Rule` struct.                        |
| [config.rs](crates/macros/src/config.rs)           | `Config` derive macro — generates config struct + builder + `From`/`TryFrom` impls.                     |
| [region.rs](crates/macros/src/region.rs)           | `Region` derive macro — generates region-based server URL constants from JSON.                          |
| [serialize.rs](crates/macros/src/serialize.rs)     | `serialize!` proc macro — wraps `serde_json::to_string`.                                                |
| [deserialize.rs](crates/macros/src/deserialize.rs) | `deserialize!` proc macro — wraps `serde_json::from_str`.                                               |
| [timeout.rs](crates/macros/src/timeout.rs)         | `timeout!` attribute macro for async functions with optional `#[tracing::instrument]`.                  |

### UniFFI Bindings — [crates/bindings_uniffi/src/](crates/bindings_uniffi/src/)

| File                                                                                                      | Description                                                                                             |
| --------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------- |
| [lib.rs](crates/bindings_uniffi/src/lib.rs)                                                               | Scaffolding. Re-exports `PocketOption`, `RawHandler`, `Action`, `Asset`, `Candle`, `Deal`, `Validator`. |
| [platforms/pocketoption/client.rs](crates/bindings_uniffi/src/platforms/pocketoption/client.rs)           | PocketOption UniFFI client (subset of full API).                                                        |
| [platforms/pocketoption/types.rs](crates/bindings_uniffi/src/platforms/pocketoption/types.rs)             | UniFFI-compatible types.                                                                                |
| [platforms/pocketoption/validator.rs](crates/bindings_uniffi/src/platforms/pocketoption/validator.rs)     | Validator for UniFFI.                                                                                   |
| [platforms/pocketoption/stream.rs](crates/bindings_uniffi/src/platforms/pocketoption/stream.rs)           | Subscription stream for UniFFI.                                                                         |
| [platforms/pocketoption/raw_handler.rs](crates/bindings_uniffi/src/platforms/pocketoption/raw_handler.rs) | RawHandler for UniFFI.                                                                                  |

---

## Data Flow

### Trading

```
Python PocketOption/PocketOptionAsync
→ PyO3 RawPocketOption (pyo3_async_runtimes::future_into_py)
→ binary_options_tools::pocketoption::pocket_client::PocketOption
→ Core Client::Client<S> → ClientRunner WebSocket lifecycle
→ API Modules (TradesApiModule, DealsApiModule, etc.)
→ WebSocket → PocketOption server
```

### Subscription (Real-Time)

```
Python subscribe_symbol() → Rust subscribe()
→ SubscriptionsApiModule (manages 4-sub limit)
→ Server stream → candle::SubscriptionType (Direct/Time/Chunk/TimeAligned)
→ StreamIterator → Python AsyncSubscription
```

### Raw WebSocket

```
Python create_raw_handler(validator) → RawHandler
→ RawApiModule → RawHandle/RawHandler
→ Validator-based message filtering
→ send_and_wait / subscribe pattern
```

### Trading Bot

```
Python PyStrategy subclass
→ Rust StrategyWrapper → binary_options_tools::framework::Bot
→ Bot::run() → subscribe to assets → on_candle loop
→ PyStrategy.on_candle() / on_start() / on_balance_update()
→ PyContext.buy()/sell()/balance() for trading
```

---

## Key Patterns

### Connection Lifecycle

- **States:** Connected, Disconnected (auto-reconnect), Disconnected (hold), Shutdown
- **Backoff:** Exponential with jitter (`base * 2^attempts`, max 300s, ±20% jitter)
- **Middleware hooks:** `on_connect`, `on_disconnect`, `on_send`, `on_receive`, `record_connection_attempt`

### Module Architecture

| Pattern             | Usage                                           | Examples                                                                        |
| ------------------- | ----------------------------------------------- | ------------------------------------------------------------------------------- |
| `ApiModule<S>`      | Full module with Command/CommandResponse/Handle | trades, deals, subscriptions, get_candles, historical_data, pending_trades, raw |
| `LightweightModule` | Simple background task without command-response | server_time, keep_alive                                                         |
| `Rule`              | Message routing predicate                       | Each module registers a Rule + AsyncSender pair on the Router                   |

### Validator System

| Type                       | Description                                                              |
| -------------------------- | ------------------------------------------------------------------------ |
| `RawValidator` (Rust/PyO3) | Enum: None, Regex, StartsWith, EndsWith, Contains, All, Any, Not, Custom |
| `Validator` (Python)       | High-level wrapper with static factory methods                           |
| `CrateValidator`           | Rust-native validator enum implementing `ValidatorTrait`                 |
| `PyCustomValidator`        | Bridges Python callable into Rust `ValidatorTrait` via `Arc<Py<PyAny>>`  |

### Subscription Types

| Type                  | Behavior                                                                     |
| --------------------- | ---------------------------------------------------------------------------- |
| Direct/none           | Yields raw candles as they arrive from server                                |
| Chunk(n)              | Aggregates n raw ticks/candles into one; yields aggregated candle            |
| Time(duration)        | Yields candle every `duration` seconds (sliding window)                      |
| TimeAligned(duration) | Yields candles aligned to time boundaries (e.g., every minute on the minute) |

### Error Propagation

```
PocketError → BinaryErrorPy (PyO3) → Python PyValueError
```

---

## Protocol Details

### WebSocket Framing

- **Transport:** Socket.IO 4.x (Engine.IO v4) over WebSocket
- **Connection URL:** `wss://{host}/socket.io/?EIO=4&transport=websocket`
- **Message Format:** `{packet_type}{event_id}-[{event_name},{payload}]` or `42[{event_name},{payload}]`

### SSID Format

- **Raw:** `42["auth",{...}]`
- **Regex:** `^42\["auth",\{.*\}\]$`
- **Payload (demo):** `{session, isDemo:1, uid, platform, currentUrl?, isFastHistory?, isOptimized?}`
- **Payload (real):** `{session (PHP-serialized), isDemo:0, uid, platform}`
- **JSON Recovery:** When shell-stripped, regex re-quotes unquoted keys/values

### Key Message IDs

| ID  | Meaning                                 |
| --- | --------------------------------------- |
| 42  | Standard Socket.IO event message        |
| 430 | Socket.IO event with binary attachments |
| 451 | Alternative event format                |
| 3   | Ping from server (respond with 2)       |

### Two-Step Messages

Some server events split into two WebSocket messages: (1) text header with `"_placeholder":true`, (2) binary payload with actual data. `TwoStepRule` (AtomicBool-based) and `MultiPatternRule` handle this.

---

## Testing

| Type              | Location                                                   | Tool                                                                       |
| ----------------- | ---------------------------------------------------------- | -------------------------------------------------------------------------- |
| Python tests      | `tests/python/`                                            | `pytest` (conftest.py, subdirs: pocketoption, core, tracing, experimental) |
| Rust tests        | `tests/rust/`                                              | `cargo test`                                                               |
| Crate-level tests | `crates/binary_options_tools/tests/`, `crates/core/tests/` | `cargo test`                                                               |
| Mock framework    | Core crate `TestingWrapper`                                | Mocks WebSocket streams without server connection                          |

---

## Conventions

- **Python:** Ruff linting/formatting, type hints, async/dual-mode sync with thread-safe `RLock` + event loop
- **Rust:** rustfmt, clippy, async with tokio, kanal channels for module communication, tracing for logging
- **Commit lint:** Husky + lint-staged (ruff, rustfmt on pre-commit)
- **Naming:** Rust = snake_case functions, CamelCase types. Python = snake_case methods, CamelCase classes. PyO3 = `Raw` prefix for Rust-facing classes

## Quick Reference for Agents

- **Most edited file:** `crates/bindings_pyo3/src/pocketoption.rs` (1123 lines) — make changes here for new Python API methods
- **Adding a feature:** (1) Rust logic in `crates/binary_options_tools/src/pocketoption/`, (2) Expose via PyO3 in `crates/bindings_pyo3/src/pocketoption.rs`, (3) Python wrapper in `asynchronous.py`, (4) Sync wrapper in `synchronous.py`, (5) Update `__init__.py` if new class, (6) Examples + tests
- **Changing connection:** Core lifecycle in `crates/core/src/client.rs` (`ClientRunner`)
- **Changing subscriptions:** `SubscriptionsApiModule` at `crates/binary_options_tools/src/pocketoption/modules/subscriptions.rs`
- **Adding a module:** Implement `ApiModule<S>` trait in `modules/`, register in `pocket_client.rs` router setup
- **Lint commands:** Python = `ruff check && ruff format`. Rust = `cargo clippy && cargo fmt`
- **Tests:** `pytest tests/python/` and `cargo test`
- **Docs:** MkDocs at `mkdocs.yml`. Serve: `python -m mkdocs serve`
