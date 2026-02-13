# Changelog

All notable changes to BinaryOptionsTools v2 will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Bleeding Edge / Unreleased]

### Added

- N/a

### Changed

- N/a

### Fixed

- N/a

## [0.2.6] - 2026-02-13

### Added

- Robust SSID parsing supporting complex PHP serialized session objects and sanitized Socket.IO frames
- Automated asset and payout gathering (`AssetsModule`) upon connection
- New `wait_for_assets` method to ensure library readiness before operations
- Refactored GitHub Issue and Pull Request templates
- Pre-registration API on `ResponseRouter` to eliminate race conditions in command responses

### Changed (Breaking Logic)

- **Virtual Market Profit Semantics**: `Deal.profit` now stores **net gain/loss** (e.g., -stake on loss, 0 on draw, stake*payout% on win) instead of total payout.
- **WebSocket Event System**: Unified on `EventHandler` trait and tuple/unit variants for `WebSocketEvent`. Custom handlers must update their signatures.
- **Enhanced Client Architecture**: Updated `EnhancedWebSocketInner` to require and store `credentials`, `handler`, and `connector`.
- **Context Manager Lifecycle**: Exiting the `PocketOption` context manager now explicitly closes the internal event loop, preventing resource leaks but also preventing instance reuse.

### Changed

- Increased historical data and pending order timeouts to 30s for enhanced reliability during network congestion
- Improved WebSocket routing rules (`TwoStepRule`, `MultiPatternRule`) to be resilient against interleaved messages
- Updated documentation deployment workflow to include `mkdocstrings` dependencies (gh pages)
- Reorganized internal project scripts
- Updated `BinaryOptionsToolsV2.pyi` to match the actual Rust return types (JSON strings/Lists instead of Dicts).

### Fixed

- GitHub Pages 404 error by normalizing documentation filenames to lowercase (`index.md`).
- Race conditions in history retrieval by properly pairing response messages with request indices.
- Event loop leak in Python synchronous client by fixing `__exit__` and `close()` logic.
- Boxing issues in `BinaryOptionsToolsError::WebsocketConnectionError` variant.
- API mismatches in `client2.rs` preventing successful compilation.

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

- [GitHub Releases](https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/releases)
- [PyPI Package](https://pypi.org/project/binaryoptionstoolsv2/)
- [Documentation](https://chipadevteam.github.io/BinaryOptionsTools-v2/)

[0.2.5]: https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/releases/tag/BinaryOptionsToolsV2-0.2.5
[0.2.4]: https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/releases/tag/BinaryOptionsToolsV2-0.2.4
[0.2.3]: https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/releases/tag/BinaryOptionsToolsV2-0.2.3
[0.2.0]: https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/releases/tag/BinaryOptionsToolsV2-0.2.0
