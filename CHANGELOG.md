# Changelog

All notable changes to BinaryOptionsTools v2 will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.17]

### Changed

- **Repository layout cleanup**: tracked files that sat outside the area they belong to were moved or removed, and the documentation that described them was corrected.
  - The CloseOption Python examples moved from the language-neutral `examples/async/` and `examples/sync/` into `examples/python/async/` and `examples/python/sync/`, next to the other Python examples. The run commands in `examples/README.md` point at the new paths.
  - `docs/macro_examples.rs` moved to `crates/macros/docs/macro_examples.rs`. It is an illustrative, non-compiling placeholder and the `docs/macro_proposals.md` it referenced does not exist; it now documents the macros it demonstrates and why Cargo must not compile it.
  - `AGENTS.md` no longer documents a `husky` / `lint-staged` pre-commit setup, because this repository configures no Git hooks.
  - `AGENTS.md` now matches the tree it describes: the macro crate entry no longer lists a `Config` derive that does not exist and now covers `crates/core/macros/`, `examples/` is listed, and the documentation and fixture entries no longer point at a root `data/` directory or claim a MkDocs setup that has no `mkdocs.yml`.
  - The crate documentation files `crates/binary_options_tools/Readme.md` and `crates/bindings_pyo3/Readme.md` are now `README.md`, matching `crates/core/`, `crates/bindings_uniffi/` and the `readme = "README.md"` field in each manifest.
  - `crates/macros/ActionImpl_README.md` is now `crates/macros/docs/action-impl.md`, alongside the other macro documentation instead of at the crate root. It documents a single derive macro rather than the crate, so it is a topic document and follows the kebab-case naming the other topic documents use (for example `crates/core/docs/testing-framework.md`).

- **Live-session Python tests are marked `integration`.** `test_demo_ssid.py`, `test_integration.py`, `test_asynchronous.py`, `test_synchronous.py` and `test_raw_handler.py` open real PocketOption sessions, so without the marker `pytest -k "not integration"` — the command `pytest.ini` documents for skipping network tests — still hung on them and then failed during teardown. Marking the modules makes the documented offline command pass.
- **`docs/project/deployment.md` no longer describes a Docusaurus site that does not exist.** It told readers to edit `docusaurus.config.js`, run `npm run deploy`, and configure a `pages` job in `.gitlab-ci.yml`; none of those files or jobs exist since the site was removed in 0.2.15. It now describes the Markdown that is actually tracked and what re-creating a site would take. `docs/OVERVIEW.md` and `docs/intro.md` had the same stale claim and were corrected.
- `AGENTS.md` documents the `integration` marker, the offline `pytest -k "not integration"` command, and the fact that an unmarked network test blocks and then fails on teardown.

### Security

- **Tracked lockfile no longer resolves a vulnerable `pytest`.** `python/uv.lock` declared `requires-python = ">=3.9"`, which produced a second resolution branch pinned to `pytest 8.4.2` (GHSA-6w46-j5rx-g56g, fixed in 9.0.3). Python 3.9 was never actually supported — the extension is built `abi3-py310`, so maturin emits a `cp310-abi3` wheel that pip cannot install on 3.9 — so that branch resolved packages for an interpreter this package can never target. With the floor corrected to 3.10 the lock resolves a single `pytest 9.0.3`.
- `requirements.txt` pinned `pytest>=7.0.0`, which permitted the vulnerable releases. Its floor is now `>=9.0.3`, and the MkDocs tooling group is gone: the documentation is plain Markdown and no site generator is configured.
- The remaining open alerts reference manifests that do not exist in this repository (`.arive-tasks/python-docstrings/python/uv.lock` and `BinaryOptionsToolsV2/uv.lock`); `Pygments` was already at the patched 2.20.0 in the tracked lock. Nothing in this tree needed changing for those.

### Fixed

- **`opened_deals()` / `closed_deals()` returned an object, not a list.** The Rust bindings serialised the trading state's `HashMap<Uuid, Deal>` directly, so Python received a JSON object keyed by deal id while the docstrings, the type stubs and `docs/api/python.md` all promised `List[Deal]`. Both methods now serialise the values as an array ordered by deal id, so `for deal in client.closed_deals()` and `len(...)`/indexing work as documented.
- **`PocketOption.cancel_pending_orders()` annotation contradicted the async client.** The sync wrapper was annotated `List[Dict]` while the async wrapper and the binding both produce a single `Dict` with a `cancelled` key. The annotation and docstring now match, and `PocketOption.opened_deals()` is annotated `List[Dict]` instead of `List[str]`.
- **`get_opened_deal()` / `get_closed_deal()` disagreed on their parameter name.** The async client called it `id`, the sync client and the type stubs called it `trade_id`. All of them now use `trade_id`, and the async bodies were updated so they still reference the parameter they receive.
- **The live PocketOption tests failed instead of skipping.** `pocket_client.rs` treated a missing `POCKET_OPTION_SSID` as a skip but an unreachable broker as a failure, so a machine without broker access reported seven errors. Connection failures now print a skip notice and return, matching the existing `POCKET_OPTION_SSID` behaviour.
- **`test_concurrent_identical_trades_hammer` hung forever.** It assumed the order in which trades were spawned was the order they reached the socket, then awaited a reply for a request id that had not been sent, with no timeout. It now assigns its own request ids via `trade_with_id`, asserts the socket saw exactly those ids in any order, sends the id-less `failopenOrder` first and observes which trade it hit before answering two survivors by id, and bounds every wait so a regression fails with a message instead of hanging.
- **`crates/binary_options_tools/tests/mod.rs` re-ran other test binaries.** The file declared `pub mod` for four test modules that Cargo already builds as their own binaries, so their tests ran twice in every workspace run. The file is removed.
- **`requires-python` contradicted the built wheel.** `pyproject.toml` declared `>=3.9` and listed a Python 3.9 classifier while `crates/bindings_pyo3` builds with `abi3-py310`. The declared floor, the classifier and the Ruff `target-version` (now `py310`) all match the ABI, and `AGENTS.md`, `README.md` and `examples/README.md` now say Python 3.10+.

### Removed

- Deleted the `docs-site/` `package.json` and `package-lock.json` left behind by the documentation-site removal in 0.2.15, the root `bun.lock` and `package-lock.json` (there is no root `package.json` for them to describe), the `tests/assets.txt` file that duplicated `docs/data/assets.txt`, and the `tests/login_test.txt` scratch HTTP request dump.
- Stopped tracking `crates/macros/Cargo.lock`. It was the only lockfile committed to this repository and `crates/macros/.gitignore` already listed it, so it was added by mistake; workspace builds read the root lockfile. The local copy is left in place and is now ignored.

## [0.2.16]

### Added

- **Simplified PocketOption client API**: the Python clients (`PocketOptionAsync`, `PocketOption`) now expose one obvious entry point per concern, each with the same Args/Returns docstring on both the async and sync clients.
  - Market data: `candles(asset, period, count, end_time)`, `stream_candles(asset, period, count)` and `ticks(asset, seconds)`.
  - Streaming: `subscribe_ticks(asset, *, chunk_size=None, interval=None, aligned=False)` replaces the four `subscribe_symbol*` variants.
  - Raw protocol: `raw_request(message, validator, timeout, retry)`, `raw_stream(message, validator, timeout)` and `raw_handler(validator, keep_alive)`.
  - Account: `payouts()` returns the payout of every asset; `payout(asset)` returns a single one.
- Added the `Trade` model (a `dict` subclass with attribute access plus `result`/`profit`/`is_win`/`is_loss`/`is_draw`/`is_settled`), returned by `buy`, `sell`, `check_win`, `get_opened_deal` and `get_closed_deal`. Exported as `BinaryOptionsToolsV2.Trade` alongside the `TradeResult` alias.
- Added `PocketOption.is_ssid_valid()`, mirroring the async client.

### Fixed

- **Multi-asset candle feed**: fetching live candles for several assets at once (e.g. `get_candles_live()`/`candles()` for multiple symbols concurrently) no longer returns empty history. The historical-candle module shared a single response queue across all callers, so concurrent requests consumed and discarded each other's responses; responses are now routed to the waiting request by request id.
- **`get_candles_live()` backfill**: the backfill now compiles candles from raw tick pages - the fastest source that answers reliably - and only falls back to the `changeSymbol` chart history when those pages do not cover the requested window. It no longer calls `get_candles_advanced()`: that request used the `loadHistoryPeriodFast` event, which current servers never answer, so every call left a request hanging until the connection dropped. The previous 3s timeout also discarded the chart history, which takes several seconds, leaving the feed with no history at all.
- **`get_candles_advanced()` arguments (Python)**: `PocketOptionAsync.candles(..., end_time=...)` and the deprecated `get_candles_advanced()` forwarded `time` as `offset` and `offset` as `time` to the Rust client, so the request asked the server for a window anchored at a handful of seconds. Both now map onto the raw `(asset, period, offset, time)` order.
- **`get_candles_advanced()` no longer sends `loadHistoryPeriodFast`**: current servers do not answer that event, so the request waited for a response that never arrived (and its `pending` entry stayed behind). The regular `loadHistoryPeriod` event is answered, and an abandoned request is now removed from the module instead of being kept pending. There are two possible outcomes: if the server answers, the call returns the candles compiled from that answer - normally an empty list, because the server sends `"data":[]` (`{"asset":...,"index":...,"data":[],"period":...}`) and only buffered `updateStream` ticks can add candles; if the server stays silent, the caller gets a bounded timeout instead of waiting forever. Either way the endpoint itself contributes no candle data, so `get_candles_live()` compiles history from tick pages instead.
- **Guest session balance**: guest ("try demo") sessions never receive a balance frame, so `balance()` burned its full timeout and `PocketOptionAsync.balance()` retried that 100 times (over 16 minutes without an answer). It now returns immediately with the balance carried by the auth payload, or `-1.0` when the token was replayed without one.
- **Pending history requests**: a cancelled or abandoned Rust `SubscriptionsHandle::history()` call no longer leaves the `(asset, period)` marked as in-flight, which previously made every later request for that pair fail as a duplicate forever. This path is not used by the Python `history()`/`candles()` methods, which compile candles from tick history instead.

### Changed

- `open_pending_order` now takes keyword-only arguments and a string `open_time` (`"YYYY-MM-DD HH:MM:SS"` in UTC, or `"0"` for price-based orders), matching the underlying Rust binding. The silent type-coercion fallback was removed.
- `payout(asset)` accepts a single asset symbol only; call `payouts()` and index it when several assets are needed.
- `RawCloseOption.send_raw()` now returns `None` instead of the JSON string `"null"`, matching `RawPocketOption.send_raw()`. The `CloseOption` Python wrappers already declared `-> None` and discarded the value.

### Deprecated

These methods keep working (they emit `DeprecationWarning`) and forward to the replacements above: `get_candles`, `get_candles_advanced`, `history`, `get_ticks`, `compile_candles`, `subscribe_symbol_chunked`, `subscribe_symbol_timed`, `subscribe_symbol_time_aligned`, `create_raw_handler`, `send_raw_message`, `create_raw_order`, `create_raw_order_with_timeout`, `create_raw_order_with_timeout_and_retry`, `create_raw_iterator`.

## [0.2.15]

### Added

- **Node.js Bindings**: Added `crates/bindings_napi`, an N-API (napi-rs) native addon exposing the PocketOption client, the raw WebSocket helpers, the message validators and the logging setup to JavaScript and TypeScript. The JavaScript examples and documentation previously referenced a `binary-options-tools.node` addon that did not exist anywhere in the repository, so none of them could run.
- Added the `nodejs/` package (`index.js`, `index.d.ts`, `scripts/build.js`, smoke tests) which loads the addon, adds `Symbol.asyncIterator` support to the subscription and raw streams, and exposes `snake_case` aliases next to the idiomatic `camelCase` method names.
- **SSID Fetchers**: Rewrote both CloseOption and PocketOption SSID extractor userscripts to use Violentmonkey/Tampermonkey menu commands only (no floating buttons, no `alert()`/`confirm()` dialogs). Added `GM_notification` support for success/error feedback.
- **CI/CD Cleanup**: Removed `docs-site` (Docusaurus documentation site) entirely from the repository. Deleted `.github/workflows/deploy.yml`, removed `docs` job from CI, removed `docs` stage from `.gitlab-ci.yml`, and cleaned up `.dockerignore`.

### Changed

- Rewrote every script in `examples/javascript/` against the real API: they now take the session id from `process.argv[2]` or `POCKET_OPTION_SSID`, use the documented argument order (`buy(asset, amount, seconds)`), and shut the client down when finished.
- Updated the JavaScript sections of the documentation (`docs/examples/javascript`, `docs/api/reference.md`, `docs/architecture/structure.md`) to match the implemented API. The previous snippets referenced an `binaryoptionstoolsv2` npm package and methods such as `getClosedDeals()` that do not exist.

## [0.2.14] - 2026-08-28

### Added

- **CloseOption Support**: Added new `closeoption` module for the CloseOption binary options platform, supporting Socket.IO EIO=3 protocol on `wss://www.closeoption.com:8443`.
- Added `CloseOptionAsync` and `CloseOption` Python clients mirroring the PocketOption API.
- Added PyO3 bindings for CloseOption with full async method support.
- **Historical Tick Data**: Added `get_ticks(asset, lookback_seconds)` method to `PocketOptionAsync`, `PocketOption` (sync), and `RawPocketOption` clients for fetching raw tick history via `loadHistoryPeriod` WebSocket message with pagination.
- Added `get_ticks.py` examples for both async and sync Python APIs.

### Fixed

- **Critical: Request/Response Routing**: Fixed `send_and_wait` to properly route responses via `HashMap<u64, oneshot::Sender>` registry instead of creating orphaned channels that always timeout.
- **Implemented missing methods**: Added `payout`, `history`, `opened_deals`, `closed_deals`, `get_candles_live`, `raw_handler`, `subscribe_symbol`, `subscribe_raw`, and `reconnect` to the Rust `CloseOption` client.
- **Fixed duplicate code**: Extracted `duration_to_time_intervals()` helper to eliminate duplicated duration-mapping logic in `buy` and `sell` methods.
- **Implemented check_win**: Changed from stub returning error to functional implementation using asset lookup.
- **Removed unused code**: Eliminated dead `engine_io_type` variable and unused `state` field from `KeepAliveModule`.

### Changed

- Updated all internal crates to version `0.2.14`.

## [0.2.13] - 2026-07-19

### Added

- **Live Candle Streaming**: Added `get_candles_live` method to both `PocketOptionAsync` (async) and `PocketOption` (sync) clients, which implements gap-free real-time candle building and backfilling.
- Added a 15-second per-test-function global timeout to the pytest suite to prevent hangs.

### Changed

- Deprecated duplicate candle functions `candles()` and `get_candles()` in favor of `get_candles_live()`.
- Redirected the deprecated methods to run `get_candles_live()` internally (returning the first yielded closed candle list) to maintain backward compatibility.

### Fixed

- **Robust Connection Handoff**: Wrapped historical candle fetching API calls within `get_candles_live()` with short timeouts and try-except fallbacks, preventing connection initialization hangs when compile-candles or tick history endpoints are slow or rate-limited.
- Removed anti-pattern `hasattr(client, "send_raw")` in unit tests, replacing it with direct calls to `await client.send_raw(...)`.
- Updated all internal crates and dependencies to version `0.2.13`.

## [0.2.12] - 2026-06-30

### Added

- **UTC Candle Compilation**: Refactored candle compilation to fetch raw 1-second ticks and manually build candles based on UTC time boundaries (`timestamp / period * period`), avoiding server-side candle time-alignment mismatches, overlaps, and gaps.
- Added `getBalance` to initial connection messages to ensure balance is fetched immediately upon reconnecting.
- Added warnings and logging to previously swallowed async channel errors.
- Exposed more functions in the python documentation wrapper.

### Changed

- Replaced the Notify-based wait primitives in `Signals` with stateful `tokio::sync::watch` to prevent TOCTOU race conditions.
- Derived `Debug` for `Action` in `crates/bindings_pyo3/src/framework.rs` to fix test compilation.
- Updated all internal crates and dependencies to version `0.2.12`.

### Fixed

- Fixed subscription restoration to fire on both `on_connect` and `on_reconnect` paths so subscriptions are not lost on explicit disconnects.
- Fixed memory leaks: Added pruning/eviction to `latest_ticks` in `GetCandlesApiModule` and cleared pending state (`pending_market_orders`, `recent_trades`, `pending_deals`) on reconnection.
- Fixed panics: Replaced `.expect()` calls with poison recovery (`.unwrap_or_else(|e| e.into_inner())`) in `state.rs` and `raw.rs`, and returned `Result` from `RuleBuilder::regex()` instead of panicking on invalid regexes.
- Fixed module lifecycle hang: Implemented proper waiter notification on module `Drop` for `PendingTrades`, `Raw`, and `HistoricalData` modules.
- Fixed command queue mismatches in `pending_trades.rs` by separating queues per command type.

## [0.2.10] - 2026-03-22

### Added

- Rule macro system with DSL support for ergonomic rule definitions (`#[rule({ starts_with("42") & !contains("error") })]`)
- Per-language documentation macro for BinaryOptionsToolsUni crate
- Demo test example for verifying library functionality (connection, subscriptions, candle fetching)
- Trade state regression test
- Documentation JSON files for API modules (error, pocket_option, raw_handler, stream, types, validator)
- Max subscriptions configuration in State builder (default: 4)
- Memory pruning for closed deals (limit to 1000) to prevent memory growth

### Changed

- **Major core crate refactoring**: Removed `core-pre` crate and consolidated into `core` crate
- Improved SSID parsing with double-encoding detection and regex recovery for malformed JSON
- Enhanced deals module with better Socket.IO frame handling and message pattern matching
- Refactored historical data module for improved reliability
- Updated README with new Python bot framework examples and corrected version support
- Updated Python version support (dropped 3.8, 3.13, 3.14, 3.15)
- Dependency updates across multiple directories

### Fixed

- Fixed check win functionality
- Fixed documentation errors in Rust source
- Fixed source distribution (sdist) build

## [0.2.9] - 2026-03-09

### Added

- N/a

### Changed

- Updated python support
- Improved SSID parsing to prevent double encoded JSON msgs
- Minor docs updates

### Fixed

- Fixed auth failure with valid SSID: `Ssid::Display` now returns the raw auth message (`42["auth",{...}]`) instead of a human-readable label, so the correct credential string is sent to the server during WebSocket handshake.
- Balance returning -1 (possibly)
- Unsafe unwraps

## [0.2.8] - 2026-02-22

### Added

- Pre-registration API on `ResponseRouter` to eliminate race conditions in command responses
- SSID Fetcher UserScript for easier SSID extraction from browser
- Framework improvements: `on_balance_update` now works correctly
- Support for storing indicators in the Python framework
- PyStrategy integration improvements

### Changed (Breaking Logic)

- **Virtual Market Profit Semantics**: `Deal.profit` now stores **net gain/loss** (e.g., -stake on loss, 0 on draw, stake payout % on win) instead of total payout.
- **WebSocket Event System**: Unified on `EventHandler` trait and tuple/unit variants for `WebSocketEvent`. Custom handlers must update their signatures and can now provide an optional `name()`.
- **Enhanced Client Architecture**: Updated `EnhancedWebSocketInner` to require and store `credentials`, `handler`, and `connector`.
- **Context Manager Lifecycle**: Exiting the `PocketOption` context manager now explicitly closes the internal event loop, preventing resource leaks but also preventing instance reuse.

### Changed

- Updated `BinaryOptionsToolsV2.pyi` to match the actual Rust return types (JSON strings/Lists instead of Dicts).
- Updated documentation and README
- Code quality improvements and clippy fixes
- CI workflow updated for stable PyO3
- Thread-safe `buy()` calls in synchronous client via `threading.RLock()`
- Removed unused `resend_connection_messages` method

### Fixed

- Event loop leak in Python synchronous client by fixing `__exit__` and `close()` logic.
- Boxing issues in `BinaryOptionsToolsError::WebsocketConnectionError` variant.
- API mismatches in `client2.rs` preventing successful compilation.
- Fixed `on_balance_update` event handling.
- Fixed concurrent test failures.
- Fixed failing pytest tests.
- Fixed pending trades test.
- Fixed PyO3 compatibility issues with `chrono::Duration`.

## [0.2.6] - 2026-02-13

### Added

- Robust SSID parsing supporting complex PHP serialized session objects and sanitized Socket.IO frames
- Automated asset and payout gathering (`AssetsModule`) upon connection
- New `wait_for_assets` method to ensure library readiness before operations
- Refactored GitHub Issue and Pull Request templates
- Pre-registration API on `ResponseRouter` to eliminate race conditions in command responses
- Real event handler removal by name in `WebSocketClient2`
- Preserve original event variants when broadcasting events in `WebSocketClient2`

### Changed (Breaking Logic)

- **Virtual Market Profit Semantics**: `Deal.profit` now stores **net gain/loss** (e.g., -stake on loss, 0 on draw, stake payout % on win) instead of total payout.
- **WebSocket Event System**: Unified on `EventHandler` trait and tuple/unit variants for `WebSocketEvent`. Custom handlers must update their signatures and can now provide an optional `name()`.
- **Enhanced Client Architecture**: Updated `EnhancedWebSocketInner` to require and store `credentials`, `handler`, and `connector`.
- **Context Manager Lifecycle**: Exiting the `PocketOption` context manager now explicitly closes the internal event loop, preventing resource leaks but also preventing instance reuse.

### Changed

- Increased historical data and pending order timeouts to 30s for enhanced reliability during network congestion
- Improved WebSocket routing rules (`TwoStepRule`, `MultiPatternRule`) to be resilient against interleaved messages
- Updated documentation deployment workflow to include `mkdocstrings` dependencies (gh pages)
- Reorganized internal project scripts
- Updated `BinaryOptionsToolsV2.pyi` to match the actual Rust return types (JSON strings/Lists instead of Dicts).
- Improved message sending priority by using biased polling in `EnhancedWebSocketClient`.
- Enhanced event dispatching with concurrency limiting (semaphore) in `WebSocketClient2`.

### Fixed

- GitHub Pages 404 error by normalizing documentation filenames to lowercase (`index.md`).
- Race conditions in history retrieval by properly pairing response messages with request indices.
- Event loop leak in Python synchronous client by fixing `__exit__` and `close()` logic.
- Boxing issues in `BinaryOptionsToolsError::WebsocketConnectionError` variant.
- API mismatches in `client2.rs` preventing successful compilation.
- Silent `Decimal` to `f64` conversion error in `subscriptions.rs` with proper error propagation.
- Misleading connection error reporting; now returns the actual last failure from multiple URL attempts.

## [0.2.5] - 2026-02-08

### Added

- Files to sort into respective folders - /SortLaterOr_rm/

### Changed

- Organized - Merged `/examples/` to `/docs/examples/`
- Added more rules within `.gitignore`

### Fixed

- Prettier format
- SSID parsing errors within demo vs real differences

## [0.2.4] - 2026-02-03

### Added

- Advanced candle data retrieval with `get_candles` and `get_candles_advanced`
- Advanced validators for message filtering
- Improved WebSocket message handling
- Enhanced documentation in the `docs/` directory

### Changed

- Improved error handling for connection management
- Updated Python bindings for better async support
- Enhanced type safety across Rust and Python interfaces

### Fixed

- Connection stability improvements
- Memory leak fixes in WebSocket handlers
- Error handling in subscription management

## [0.2.3] - 2023-12-XX

### Added

- Raw Handler API for advanced WebSocket control
- Validator system for response filtering
- Enhanced subscription management
- Time-aligned subscription support

### Changed

- Improved reconnection logic with exponential backoff
- Better error messages and logging
- Updated dependencies for security patches

### Fixed

- Race conditions in message routing
- Subscription cleanup on disconnect
- Memory management in async operations

## [0.2.0] - 2023-11-XX

### Added

- Complete rewrite in Rust for performance and reliability
- Python bindings via PyO3
- Async and sync Python APIs
- Real-time market data streaming
- WebSocket connection management
- Automatic reconnection with exponential backoff
- Type-safe interfaces across languages

### Changed

- Architecture redesigned with Rust core
- Improved performance (10x faster than v1)
- Better memory management
- Enhanced error handling

### Removed

- Python-only implementation (replaced with Rust core)
- Legacy API endpoints (deprecated in v1)

## [0.1.x] - 2023-XX-XX

### Initial Release

- Python-based implementation
- Basic PocketOption API support
- Trading operations (buy/sell)
- Balance retrieval
- Basic WebSocket connection

---

## Version Naming Convention

- **Major version** (X.0.0): Breaking changes, major architecture changes
- **Minor version** (0.X.0): New features, non-breaking changes
- **Patch version** (0.0.X): Bug fixes, security patches

## Types of Changes

- **Added**: New features
- **Changed**: Changes in existing functionality
- **Deprecated**: Soon-to-be removed features
- **Removed**: Removed features
- **Fixed**: Bug fixes
- **Security**: Security vulnerability fixes

## Links

- [GitLab Releases](https://gitlab.chipatrade.com/chipadevorg/BinaryOptionsTools-v2/-/releases)
- [PyPI Package](https://pypi.org/project/binaryoptionstoolsv2/)
- [Documentation](https://chipatrade.gitlab.io/chipadevorg/BinaryOptionsTools-v2/)

[0.2.14]: https://gitlab.chipatrade.com/chipadevorg/BinaryOptionsTools-v2/-/releases/v0.2.14
[0.2.13]: https://gitlab.chipatrade.com/chipadevorg/BinaryOptionsTools-v2/-/releases/v0.2.13
[0.2.12]: https://gitlab.chipatrade.com/chipadevorg/BinaryOptionsTools-v2/-/releases/v0.2.12
[0.2.10]: https://gitlab.chipatrade.com/chipadevorg/BinaryOptionsTools-v2/-/releases/v0.2.10
[0.2.9]: https://gitlab.chipatrade.com/chipadevorg/BinaryOptionsTools-v2/-/releases/v0.2.9
[0.2.8]: https://gitlab.chipatrade.com/chipadevorg/BinaryOptionsTools-v2/-/releases/v0.2.8
[0.2.6]: https://gitlab.chipatrade.com/chipadevorg/BinaryOptionsTools-v2/-/releases/v0.2.6
[0.2.5]: https://gitlab.chipatrade.com/chipadevorg/BinaryOptionsTools-v2/-/releases/v0.2.5
[0.2.4]: https://gitlab.chipatrade.com/chipadevorg/BinaryOptionsTools-v2/-/releases/v0.2.4
[0.2.3]: https://gitlab.chipatrade.com/chipadevorg/BinaryOptionsTools-v2/-/releases/v0.2.3
[0.2.0]: https://gitlab.chipatrade.com/chipadevorg/BinaryOptionsTools-v2/-/releases/v0.2.0
