---
sidebar_position: 4
slug: /architecture/python-api-layers
---

# Python API Layers

The Python package has three layers. Each one has a different job, and mixing
them up is the usual source of "why does `payout()` return a string here and an
int there?" confusion.

| Layer            | Where                                                                 | Returns                             | Owns                                    |
| ---------------- | --------------------------------------------------------------------- | ----------------------------------- | --------------------------------------- |
| Rust core        | `crates/binary_options_tools/src/pocketoption/`                       | Rust types / `PocketResult`         | Protocol, caching, pacing, retries      |
| Raw PyO3 binding | `crates/bindings_pyo3/src/pocketoption.rs` (`RawPocketOption`)        | JSON strings, numbers, futures      | Translating Rust values for Python      |
| Async client     | `python/BinaryOptionsToolsV2/pocketoption/asynchronous.py`            | Python types (`dict`, `int`, ...)   | Parsing, validation, ergonomic wrappers |
| Sync client      | `python/BinaryOptionsToolsV2/pocketoption/synchronous.py`             | Same Python types                   | Thread bridge only                      |

The sync client is **not** a second implementation. It schedules the matching
`PocketOptionAsync` coroutine on the client's background event loop and blocks the
calling thread. Keep parsing and protocol logic in `asynchronous.py`; add a method
there first, then delegate from `synchronous.py`. `tests/python/pocketoption/test_api_parity.py`
guard that the two classes expose the same names.

## Adding or changing a method

1. Implement the operation once in the Rust core (`pocket_client.rs` or a module).
2. Expose it from `crates/bindings_pyo3/src/pocketoption.rs`. Keep the raw signature
   close to the core one and document what it returns.
3. Add the public method to `PocketOptionAsync` and parse/validate the raw result.
4. Delegate from `PocketOption` (one `self._run(self._client.<method>(...))` line).
5. Add a mocked test for the async client and a delegation test for the sync client.
6. Update `docs/api/python.md`, the type stub, and the example pages that show it.

If a method exists only on the high-level client and is computed entirely from
other public calls, it still belongs in `asynchronous.py`; do not push it into the
Rust binding.

## Naming traps

### `payout` vs `payouts`

| Call                                   | Layer      | Returns                                  |
| -------------------------------------- | ---------- | ---------------------------------------- |
| `client.payout(assets=None)`           | Raw PyO3   | JSON string of `{symbol: percentage}`    |
| `client.payout_for(asset)`             | Raw PyO3   | `Optional[int]`                          |
| `await client.payouts(assets=None)`    | High-level | `Dict[str, int]`                         |
| `await client.payout(asset)`           | High-level | `Optional[int]`                          |

The high-level `payout(asset)` is a single-entry lookup: it calls the raw
`payout_for(asset)` and does not serialise the catalog. `payouts(assets)` filters
inside the cached catalog so polling a watchlist does not pay for every symbol.
`None` and unknown/inactive symbols are omitted by the filtered form.

### Candle methods

| Method                                             | Shape / behaviour                                                                                                                                                           |
| -------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `candles(asset, period, count=100, end_time=None)` | Without `end_time`: closed candles, oldest first, `{time, open, high, low, close}`. With `end_time`: the raw history window (`timestamp`, `symbol`, `volume`, `is_closed`). |
| `candles_many(assets, period, count=100)`          | Same shape per symbol, built from tick snapshots; no live subscription per symbol.                                                                                          |
| `get_candles_live(asset, period, hours, max_rows)` | Async generator of `(closed_candles, forming_candle)`.                                                                                                                      |
| `stream_candles(...)`                              | Thin generator wrapper over `get_candles_live`.                                                                                                                             |
| `get_candles(...)`                                 | **Deprecated**: delegates to `get_candles_live`.                                                                                                                            |
| `get_candles_advanced(...)`                        | **Deprecated**: one history request; `offset` is a window in seconds.                                                                                                       |
| `compile_candles(...)`                             | **Deprecated** on the wrapper; builds non-standard timeframes from ticks.                                                                                                   |
| `ticks(asset, seconds)` / `get_ticks(...)`         | Raw `(timestamp, price)` tuples.                                                                                                                                            |

`candles_many` caches each `(asset, period, count)` snapshot until the next candle
boundary — closed candles are immutable — and single-flights concurrent callers
that want the same key. The cache is cleared on `disconnect()`, `connect()`,
`reconnect()` and `shutdown()`. A dead session raises `ConnectionError` (not
connected, disconnected mid-batch, or the engine's channel-closed error) instead
of returning an empty mapping; per-symbol failures are still omitted.

`offset` is the most common trap: on `get_candles_advanced` it is **seconds**, not
a count of periods. `candles(..., end_time=...)` currently forwards `count` as
that `offset`, so the requested window can be much shorter than `count` periods,
and the result is the binding's `Candle` JSON unchanged rather than the normalized
`{time, ...}` shape. See [candle history limits](../guides/candle-history-limits.md)
for the broker-side ceilings.

## Event loop and cost rules

`PocketOption` owns a single background asyncio loop and every call is scheduled
on it with `run_coroutine_threadsafe`. Therefore:

- Do not run blocking work inside a coroutine. It delays every other call on the
  shared loop (candles, orders, payouts) until it returns.
- Do not clone or serialise the full asset catalog for a single-symbol question.
  Read the cached entry in place, as `Assets::payout` now does.
- Do not open a live subscription to answer a one-shot snapshot. Use the history
  endpoint, which is paced by the core, and remember the ceiling: requests start
  ~200 ms apart with at most four in flight, about five per second per client.
- A sync method is a thread bridge; it must never touch `self._loop` state from
  the calling thread. Delegate and let `_run` do the hand-off.

## Tests

- `tests/python/pocketoption/test_async_mocked.py` - async behaviour against a
  mocked raw client.
- `tests/python/pocketoption/test_sync_mocked.py` - sync delegation and wrappers.
- `tests/python/pocketoption/test_api_parity.py` - both clients expose the same
  public names and every documented method has a docstring.
- Live-broker tests carry the `integration` marker; run the offline subset with
  `pytest -k "not integration"`.
