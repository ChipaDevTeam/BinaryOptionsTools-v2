# Simplification Plan

A survey of the Rust and Python sources for changes that shrink or de-duplicate
the code **without** changing behaviour, removing features, or altering any public
API. Applied items are marked; the rest are queued with the reason they were left.

Ground rule used throughout: a simplification is only accepted when the existing
tests still pass unchanged, and when any wire format it touches stays
byte-identical (the broker compares frames verbatim).

## Applied

| Area | Change | Lines saved |
| --- | --- | --- |
| `closeoption/client.rs` | `buy`/`sell` were 58-line copies differing only in `"call"`/`"put"`; both now delegate to one `place_order`. The four `self.state()` calls per order collapsed into one binding. | ~45 |
| `pocketoption/modules/get_candles.rs` | `get_candles_advanced` and each `get_ticks` page repeated the same slot → register → send → guard → await dance; now one `request()` helper. | ~55 |
| `pocketoption/modules/historical_data.rs` | The `GetTicks`/`GetCandles` arms were identical apart from the request kind; now one `begin_request()`. | ~30 |
| `pocketoption/utils.rs` | New `socketio_frame()` owns the `42["event", payload]` framing that four modules built by hand. | ~15 |
| `pocketoption/modules/get_candles.rs` | Added `GetCandlesHandle::new`, replacing three struct literals (two of which had already drifted). | ~10 |
| `pocketoption/types.rs`, `modules/deals.rs` | `MultiPatternRule::new` / `DealsUpdateRule::new` now accept any `IntoIterator<Item: ToString>`, so callers stop wrapping literals in `vec![...]` and cloning owned strings. | ~8 |
| `pocketoption/modules/trades.rs` | `CommandResponse` was declared with two variants and never constructed (every order is answered by a `oneshot`); made it empty so the impossibility is checked by the compiler. | ~10 |
| `pocketoption/modules/assets.rs` | Asset test fixtures gained the `opens_at` field through a shared literal instead of four copies. | ~5 |
| `bindings_pyo3/src/logs.rs` | Three clippy-identified redundancies (needless borrows, `drain(..).collect()` into the same type). | ~6 |

## Queued (safe, not yet done)

1. **`serde_json::from_slice` + `warn!` repeated in nine modules.** A
   `parse_binary<T>(data, target) -> Option<T>` helper in `pocketoption/utils.rs`
   would remove the repeated match arms (~40 lines). Deferred because each site
   logs a slightly different message and the log text is useful when diagnosing a
   broker change; worth doing together with a deliberate logging pass.
2. **`closeoption/types.rs` hand-rolled Engine.IO/Socket.IO parser (~100 lines).**
   The `Vec<char>` + repeated slicing could be replaced with `serde_json` on the
   payload and byte offsets for the prefix (~40 lines). Medium risk: every frame
   type must keep parsing, and there is no capture-based test for it yet, so the
   test has to come first.
3. **`AssetRawTuple` with nine `IgnoredAny` slots.** A named-field struct
   deserialising only the eight used positions would drop the `#[allow(dead_code)]`
   (~10 lines) but is medium risk because the wire format is positional and a
   mistake silently shifts every field.
4. **`loop-invariant clone in historical_data.rs`** (`symbol.clone()` per candle,
   ~2 lines) - trivial, batched with the next edit of that function.

## Rejected

- Merging `HistoricalDataApiModule` into `GetCandlesApiModule`. They overlap, but
  the first answers `changeSymbol` history (bounded, single pending request) and
  the second answers `loadHistoryPeriod` pages; several callers use both request
  shapes and the pending/error semantics differ. Merging would change behaviour.
- Making `SubscriptionStream` a `Stream`. It reads as a natural simplification, but
  `receive()` returns only completed candles (aggregation happens inside the
  module), whereas a `Stream` of raw events would change what callers observe.
- Removing `PocketOption::history()` (deprecated alias of `candles()`). Public API;
  removing it needs a major version.
