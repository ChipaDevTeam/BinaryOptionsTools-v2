# Changelog

All notable changes to BinaryOptionsTools v2 will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- CONTRIBUTING.md with detailed contribution guidelines
- CODE_OF_CONDUCT.md for community standards
- SECURITY.md with security policy and best practices
- CHANGELOG.md for tracking version history
- CITATION.cff for academic citations
- AUTHORS.md listing all contributors
- ACKNOWLEDGMENTS.md for credits and thanks

## [0.2.4] - 2024-01-XX

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

[Unreleased]: https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/compare/BinaryOptionsToolsV2-0.2.4...HEAD
[0.2.4]: https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/releases/tag/BinaryOptionsToolsV2-0.2.4
[0.2.3]: https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/releases/tag/BinaryOptionsToolsV2-0.2.3
[0.2.0]: https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/releases/tag/BinaryOptionsToolsV2-0.2.0
