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

### API Module Traits (`crates/core-pre/src/traits.rs`)

- **LightweightModule / ApiModule**: Added `RunnerCommand` but integration across all modules is still in progress (e.g., handling `Shutdown` gracefully in every module).

## Python Extension (`BinaryOptionsToolsV2`)

### Validator (`src/validator.rs`)

- **Validation Methods**: `TODO`: Restore validation methods (e.g., `is_valid`, `validate_json`) when the new API supports it.
- **BoxedValidator/RegexValidator**: `TODO`: Restore these implementations.

### PocketOption Client

- **Advanced Indicators**: Many technical indicators available in V1 are not yet exposed or implemented in the V2 Rust core.
- **Social Trading**: Unimplemented.
- **Tournament Logic**: Unimplemented.

## Tests

- **Trade Tests**: Currently skipped on real accounts for safety. Requires a dedicated demo account SSID for full CI coverage of trading features.
