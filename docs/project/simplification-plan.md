# Simplification Plan

A survey of the Rust and Python sources for changes that shrink or de-duplicate
the code **without** changing behaviour, removing features, or altering any public
API. Applied items are marked; the rest are closed with the evidence that decided
them.

Ground rule used throughout: a simplification is only accepted when the existing
tests still pass unchanged, and when any wire format it touches stays
byte-identical (the broker compares frames verbatim). Where the wire format is
hand-parsed, a characterisation test is written **first** and must pass both before
and after.

## Applied

| Area                                        | Change                                                                                                                                                                                                                                                                     | Lines saved |
| ------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------- |
| `closeoption/client.rs`                     | `buy`/`sell` were 58-line copies differing only in `"call"`/`"put"`; both now delegate to one `place_order`. The four `self.state()` calls per order collapsed into one binding.                                                                                           | ~45         |
| `closeoption/types.rs`                      | `parse_frame` collected the frame into a `Vec<char>` before slicing; it now slices by byte offset (every inspected byte is ASCII). The six separate control-frame arms collapsed into one fallthrough arm. 108 → 71 lines.                                                 | ~37         |
| `closeoption/types.rs`                      | `wire_code()` / `from_wire_code()` are now the single source of truth for the Engine.IO digit mapping. The parser and the encoder previously carried separate, hand-written tables that could drift apart; `encode_frame`'s `unreachable!()` table is gone. 35 → 28 lines. | ~20         |
| `pocketoption/modules/get_candles.rs`       | `get_candles_advanced` and each `get_ticks` page repeated the same slot → register → send → guard → await dance; now one `request()` helper.                                                                                                                               | ~55         |
| `pocketoption/modules/historical_data.rs`   | The `GetTicks`/`GetCandles` arms were identical apart from the request kind; now one `begin_request()`.                                                                                                                                                                    | ~30         |
| `pocketoption/utils.rs`                     | New `socketio_frame()` owns the `42["event", payload]` framing that four modules built by hand.                                                                                                                                                                            | ~15         |
| `pocketoption/modules/get_candles.rs`       | Added `GetCandlesHandle::new`, replacing three struct literals (two of which had already drifted).                                                                                                                                                                         | ~10         |
| `pocketoption/types.rs`, `modules/deals.rs` | `MultiPatternRule::new` / `DealsUpdateRule::new` now accept any `IntoIterator<Item: ToString>`, so callers stop wrapping literals in `vec![...]` and cloning owned strings.                                                                                                | ~8          |
| `pocketoption/modules/trades.rs`            | `CommandResponse` was declared with two variants and never constructed (every order is answered by a `oneshot`); made it empty so the impossibility is checked by the compiler.                                                                                            | ~10         |
| `pocketoption/modules/assets.rs`            | Asset test fixtures gained the `opens_at` field through a shared literal instead of four copies.                                                                                                                                                                           | ~5          |
| `bindings_pyo3/src/logs.rs`                 | Three clippy-identified redundancies (needless borrows, `drain(..).collect()` into the same type).                                                                                                                                                                         | ~6          |

The `closeoption` parser is covered by `closeoption::types::tests::frame_parser`:
every frame shape the stream uses (handshake, ping/pong, upgrade, connect,
disconnect, event with and without a namespace, ack, binary event/ack, connect
error) is pinned, plus the rejection of malformed frames.

## Closed (verified not simplifiable)

1. **`parse_binary<T>` helper for the nine `serde_json::from_slice` + `warn!`
   sites - rejected.** `tracing` requires a _compile-time constant_ target
   (`error[E0435]: attempt to use a non-constant value in a constant`), so a shared
   helper would report every parse failure against `utils.rs` and destroy
   per-module log filtering, which is how a broker protocol change is diagnosed.
   The arms also differ in substance, not just wording: `deals.rs` recurses into a
   second type, `trades.rs` propagates the error, and `assets.rs`/`balance.rs` log
   the raw bytes rather than the error. Net saving would have been ~2 lines per
   site for a real loss in observability.
2. **`AssetRawTuple` with `IgnoredAny` slots - rejected.** The wire payload is a
   positional JSON array and `Asset` uses the positions `0,1,2,3,5,9,14,15,18`.
   A named-field struct could only work if serde tolerated the gaps, and it does
   not: named structs consume exactly as many elements as they declare, so a
   longer array fails with `trailing characters at line 1 column 8` (measured).
   `IgnoredAny` placeholders are therefore _required_ to keep the 19 positions
   aligned. Replacing them needs a hand-written `SeqAccess` visitor - more code,
   and the failure mode is silently shifting every field.
3. **Loop-invariant `symbol.clone()` in `historical_data.rs` - rejected.**
   `Candle.symbol` is an owned `String`, so one allocation per candle is inherent
   to the type, not loop-invariant work.

## Rejected (would change behaviour)

- Merging `HistoricalDataApiModule` into `GetCandlesApiModule`. They overlap, but
  the first answers `changeSymbol` history (bounded, single pending request) and
  the second answers `loadHistoryPeriod` pages; several callers use both request
  shapes and the pending/error semantics differ.
- Making `SubscriptionStream` a `Stream`. It reads as a natural simplification, but
  `receive()` returns only completed candles (aggregation happens inside the
  module), whereas a `Stream` of raw events would change what callers observe.
- Removing `PocketOption::history()` (deprecated alias of `candles()`). Public API;
  removing it needs a major version.
