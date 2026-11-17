---
sidebar_position: 5
slug: /architecture/deals-and-trades
---

# Deals, Trades and Pending Orders

Three `ApiModule`s share `TradeState`:

| Module                     | File                                   | Role                                                             |
| -------------------------- | -------------------------------------- | ---------------------------------------------------------------- |
| `DealsApiModule`           | `modules/deals.rs`                     | Passive listener for deal updates; resolves result waiters       |
| `TradesApiModule`          | `modules/trades.rs`                    | Sends `openOrder` and matches `successopenOrder`/`failopenOrder` |
| `PendingTradesApiModule`   | `modules/pending_trades.rs`            | Opens, cancels and tracks pending limit orders                   |

`PocketOption` (`pocket_client.rs`) is the façade: it owns the trade lock,
prevents duplicate market orders, and exposes `buy()`, `sell()`, `check_win()`,
`open_pending_order()` and the deal-history methods.

## Shared state

`TradeState` lives in `pocketoption/state.rs` behind `State::trade_state`:

```rust
pub struct TradeState {
    opened_deals: RwLock<HashMap<Uuid, Deal>>,
    closed_deals: RwLock<HashMap<Uuid, Deal>>,
    pub pending_deals: RwLock<HashMap<Uuid, PendingOrder>>,
    // Requests sent but not confirmed by the server yet.
    pub pending_market_orders: RwLock<HashMap<Uuid, (OpenOrder, Instant)>>,
    // Cache of recent successful trades. Currently written but not read; there
    // is no library-level duplicate check that consults it.
    pub recent_trades: RwLock<HashMap<(String, Action, u32, Decimal), (Uuid, Instant)>>,
}
```

`opened_deals` and `closed_deals` are private; use the async accessors
(`add_*`, `update_*`, `get_*`, `contains_*`, `clear_*`, `prune_closed_deals`)
instead of taking the locks from another module. Keeping the two maps consistent
is the whole point of the accessors: `update_closed_deals` moves a deal out of
`opened_deals` and into `closed_deals` in one step.

## `DealsApiModule`

It listens for:

- `updateOpenedDeals` / `updateClosedDeals` (two-step `451-` and one-step `42` frames)
- `successcloseOrder`

On a close it resolves every waiter registered for that deal id, stores the deal
in `closed_deals`, and prunes the history.

`DealsHandle` exposes:

- `check_result(trade_id) -> PocketResult<Deal>` - waits until the deal closes.
- `check_result_with_timeout(trade_id, timeout) -> PocketResult<Deal>`.

The command carries a `oneshot::Sender`, so a cancelled waiter cannot accumulate
in a shared response queue.

## `TradesApiModule`

`TradesHandle` exposes `trade`, `trade_with_id`, `buy` and `sell`. Each
`Command::OpenOrder` carries its own request id and `oneshot::Sender`; the module
matches the server reply by that id. `CommandResponse` is intentionally empty
because every answer travels through the oneshot.

There is currently no library-level duplicate guard in `trade()`: `recent_trades`
is populated after a successful order but is never consulted before sending a new
one, and `pending_trades_lock` protects only the pending-order methods. Two
identical market orders issued concurrently both reach the broker, so callers that
need deduplication must implement it themselves.

## `PendingTradesApiModule`

`PendingTradesHandle` exposes `open_pending_order`, `cancel_pending_order` and
`cancel_pending_orders`. Pending orders live in `TradeState::pending_deals`,
keyed by ticket, and are recorded when the server confirms an open. They are not
pruned when filled or cancelled: the whole map is cleared when the connection's
temporal data is reset (`State::clear_temporal_data`). `remove_pending_deal`
exists but currently has no callers.

## Where to change what

- Wire events or waiter resolution: `modules/deals.rs`.
- Order submission or response matching: `modules/trades.rs`.
- Pending-order lifecycle: `modules/pending_trades.rs`.
- Public façade, duplicate guard, `check_win`: `pocket_client.rs`.
- Python surface: `crates/bindings_pyo3/src/pocketoption.rs`, then
  `python/BinaryOptionsToolsV2/pocketoption/asynchronous.py` and the sync
  delegation. See [Python API layers](python-api-layers.md).
