# Trade and Deal Management Modules

This document outlines the architecture for handling trades and deal updates within the PocketOption client. The system is split into two primary modules: `DealsUpdateModule` (an `ApiModule`) and `TradesApiModule` (an `ApiModule`).

## Overview

- **`DealsUpdateModule`**: An API module that passively listens for WebSocket messages related to deal status changes (`updateOpenedDeals`, `updateClosedDeals`, `successcloseOrder`), keeps a shared state updated, and provides a mechanism to check for trade results.
- **`TradesApiModule`**: An interactive module that exposes an API for users to place trades. It orchestrates sending trade requests and retrieving outcomes, interacting with the shared state maintained by the `DealsUpdateModule`.

---

## Shared State (`TradeState`)

To facilitate communication and data sharing between modules, a `TradeState` struct is added to the global `AppState`.

```rust
pub struct TradeState {
    /// A map of currently opened deals, keyed by their UUID.
    pub opened_deals: RwLock<HashMap<Uuid, Deal>>,
    /// A map of recently closed deals, keyed by their UUID.
    pub closed_deals: RwLock<HashMap<Uuid, Deal>>,
}
```

---

## `DealsUpdateModule` (`ApiModule`)

This module's responsibility is to maintain the accuracy of the `TradeState` and provide a way to query for trade results.

- **Responsibilities**:
  - Listen for incoming WebSocket messages.
  - Parse messages related to deal updates.
  - Update `opened_deals` and `closed_deals` in the shared `TradeState`.
  - Provide a mechanism to check the result of a trade.
- **Messages Handled**:
  - `451-["updateOpenedDeals", ...]`
  - `451-["updateClosedDeals", ...]`
  - `451-["successcloseOrder", ...]`

### Handle Functions

The `DealsHandle` will expose the following method:

- `async fn check_result(&self, trade_id: Uuid) -> PocketResult<Deal>`: Waits for a trade to be closed and returns the final `Deal` object.

### Commands and Responses

- **`Command` Enum**:

  - `CheckResult(Uuid)`: Command to check the result of a specific trade.

- **`CommandResponse` Enum**:
  - `CheckResult(PocketResult<Deal>)`: The result of a `CheckResult` command, containing the final `Deal` object on success.

### Workflow for `check_result`

1. The user calls `check_result(trade_id)` on the handle.
2. The `DealsUpdateModule` first checks if the deal is already in the `closed_deals` map in `TradeState`.
3. If found, it returns the `Deal` immediately.
4. If not, it subscribes to updates for the `closed_deals` map and waits until the deal with `trade_id` appears. This can be implemented using a `tokio::sync::watch` channel or by periodically checking the map.
5. Once the deal is found, it returns the result. A timeout is used to prevent indefinite waiting.

---

## `TradesApiModule` (`ApiModule`)

This module provides the user-facing API for all trading-related actions.

- **Responsibilities**:
  - Provide a `TradesHandle` for users to interact with the API.
  - Accept commands to open trades (`buy`/`sell`).
  - Send `openOrder` messages to the WebSocket server.
  - Handle responses (`successopenOrder`, `failopenOrder`).
- **Responsibilities**:
  - Provide a `TradesHandle` for users to interact with the API.
  - Accept commands to open trades (`buy`/`sell`).
  - Send `openOrder` messages to the WebSocket server.
  - Handle responses (`successopenOrder`, `failopenOrder`).

### Handle Functions

The `TradesHandle` will expose the following methods:

- `async fn trade(&self, asset: String, action: Action, amount: f64, time: u32) -> PocketResult<Deal>`: Places a new trade.
- `async fn buy(&self, asset: String, amount: f64, time: u32) -> PocketResult<Deal>`: A convenience wrapper for a `Call` trade.
- `async fn sell(&self, asset: String, amount: f64, time: u32) -> PocketResult<Deal>`: A convenience wrapper for a `Put` trade.

### Commands and Responses

- **`Command` Enum**:

  - `OpenOrder(OpenOrder)`: Command to place a new trade.

- **`CommandResponse` Enum**:
  - `OpenOrder(PocketResult<Deal>)`: The result of an `OpenOrder` command, containing the initial `Deal` object on success.
