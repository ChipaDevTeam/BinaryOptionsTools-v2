# Unimplemented Features and Placeholders in BinaryOptionsToolsV2 (BoTv2)

This document tracks features that are currently unimplemented, partially implemented, or contain placeholders in the `BinaryOptionsToolsV2` repository and its core dependencies.

## Core Module (`crates/binary_options_tools`)

### Subscriptions Module (`src/pocketoption/modules/subscriptions.rs`)

- **Main Run Loop (`run`)**: Partially implemented. Contains `TODO` for:
  - Managing subscription limits.
  - Forwarding data to appropriate streams.
- **Subscription/Unsubscription Logic**:
  - `TODO`: Implement full subscription/unsubscription validation.
  - `TODO`: Check why `option_type` is always 100 in `types.rs`.
- **Data Forwarding**: `TODO`: Implement efficient data forwarding to multiple subscribers.
- **Rule Implementation**: `TODO`: Implement specific rules for all subscription-related message types.
- **WebSocket Subscription Message** (`subscriptions.rs:778`): `TODO`: Implement proper WebSocket subscription message format. Currently delegates to a helper function but the TODO indicates the implementation is incomplete.

### Client Runner (`crates/core/src/client.rs`)

- **Disconnect Control**: Implemented via `RunnerCommand::DisconnectAndHold`. This allows the client to remain disconnected without automatic reconnection until an explicit `Connect` or `Reconnect` command is received.

### API Module Traits (`crates/core-pre/src/traits.rs`)

- **LightweightModule / ApiModule**: Added `RunnerCommand` but integration across all modules is still in progress (e.g., handling `Shutdown` gracefully in every module).

### Configuration Macros (`crates/macros/src/impls/config_impl.rs`)

- **Config Macro Logic**: Entire file is a placeholder. `TODO`: Implement derive macros for configuration structures and other configuration-related functionality. Only contains a 7-line stub with no actual implementation.

### ExpertOptions Platform (`src/expertoptions/modules/profile.rs`)

- **Multiple Action Extension** (`profile.rs:339`): Comment indicates `multipleAction` with "basic actions placeholder (can be extended)". The profile module's initialization actions are minimal and intended to be expanded.

## Macro System (`docs/macro_proposals.md`, `docs/macro_examples.rs`)

Multiple proc-macro attributes and derives are proposed in `macro_proposals.md` but remain unimplemented. `macro_examples.rs` contains non-compiling placeholder code for all of these:

- **`#[lightweight_module]`**: Generate struct fields, `fn new`, `fn rule`, and run-loop scaffolding for `LightweightModule` implementations.
- **`#[api_module]`**: Remove boilerplate for `ApiModule` implementors.
- **`#[action_rule]`**: Replace repeated `MultiPatternRule::new(...)` / `TwoStepRule::new(...)` declarations.
- **`#[ws_message]`**: Generate inbound/outbound WebSocket message structs with pattern matching.
- **`#[platform_client]`**: Scaffold a platform client with module wiring and language bindings.
- **`#[pyo3_async_json]`**: Automate PyO3 async JSON method generation.
- **`uni_err!`**: Error handling macro for unified error types.
- **`#[validator_factory]`**: Generate validator structs from patterns.
- **`#[connect_strategy]`**: Generate connection parameter builders.
- **`#[module_doc_example]`**: Auto-generate doc examples showing builder wiring.

Only `#[uniffi_doc]` is implemented.

## Rule Macro Tests (`crates/core/tests/rule_macro_tests.rs`)

- **Chained Method Parsing** (`rule_macro_tests.rs:76`): `TODO`: Fix chained method parsing. Currently has issues with argument parsing, causing tests for chained method rules (e.g., `.wait()`, `.wait_messages()`) to be commented out.
- **Chained Method Tests** (`rule_macro_tests.rs:595`): `TODO`: Fix chained method tests. Tests for `ChainedWait` and `ChainedMultipleMethods` rules are commented out due to the parsing issue.

## Examples (`crates/binary_options_tools/examples/pending_trades_example.rs`)

- **Timeouts and Retries Scenarios** (`pending_trades_example.rs:536`): `TODO`: Implement scenario functions (`scenario1_mismatched_responses`, `scenario2_exceed_retries`, `scenario3_timeout`). The example function body is empty with commented-out calls.

## Python Extension (`BinaryOptionsToolsV2`)

### Validator (`src/validator.rs`)

- **Validation Methods**: High-level `Validator` class in Python (`BinaryOptionsToolsV2/validator.py`) is fully functional, supporting regex, prefix/suffix, and custom logic.

### PocketOption Client

- **Advanced Indicators**: Many technical indicators available in V1 are not yet exposed or implemented in the V2 Rust core.
- **Social Trading**: Unimplemented.
- **Tournament Logic**: Unimplemented.

## Python Extension (Uni) (`BinaryOptionsToolsUni/out/python/binary_options_tools_uni.py`)

- **Buffer Packing Optimization** (`binary_options_tools_uni.py:215`): `XXX TODO`: Investigate using `struct.pack_into` directly instead of the current byte-by-byte copy approach in `_pack_into`.

## Tests

- **Trade Tests**: Currently skipped on real accounts for safety. Requires a dedicated demo account SSID for full CI coverage of trading features.
