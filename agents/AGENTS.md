# AGENTS.md — BinaryOptionsTools-v2

This is a dual-language project: **Rust** core with **Python** bindings (via PyO3/maturin) and optional **UniFFI** bindings for Kotlin/Swift/C#/Go.

## Project Structure

- `crates/` — Rust workspace crates
  - `core/` — Low-level utilities, config, WebSocket base
  - `macros/` — Proc-macros (`Config`, `RegionImpl`, `ActionImpl`, `#[timeout]`)
  - `binary_options_tools/` — Platform implementations (PocketOption, ExpertOption), `framework` module for bots
- `BinaryOptionsToolsV2/` — Python package (maturin/PyO3). Rust source in `rust/`, Python wrapper in `python/`
- `BinaryOptionsToolsUni/` — UniFFI multi-language bindings
- `tests/` — Python tests (`tests/python/`), Rust tests inline
- `docs/` — MkDocs documentation
- `data/` — Test fixtures and JSON data

## Product Context

A high-performance, cross-platform package for automating binary options trading, built with a Rust core and high-level bindings for Python and other languages.

### Primary Users

- **Trading Bot Developers**: Individuals building automated trading systems.
- **Quantitative Traders**: Users requiring high-performance data streaming and execution for strategies.
- **Retail Traders**: Users looking for reliable tools to interface with binary options platforms programmatically.

### Main Goal

To bridge the gap between low-level performance and high-level usability, providing a robust, type-safe, and scalable framework for real-time market data streaming and instant trade execution on binary options platforms (starting with PocketOption).

### Key Features

- **High-Performance Rust Core**: Leveraging Rust for concurrency and memory safety.
- **Cross-Platform Bindings**: Seamless integration with Python (PyO3) and multiple other languages via UniFFI (Kotlin, Swift, Go, Ruby, C#).
- **Real-Time Data Streaming**: Native WebSocket support for live OHLC candles and market updates.
- **Instant Trade Execution**: Fast placement and monitoring of trades with configurable timeouts.
- **Historical Data Support**: Fetching OHLC data for backtesting and analysis.
- **Robust Connectivity**: Automatic reconnection, keep-alive monitoring, and server time synchronization.
- **Extensible Architecture**: Raw Handler API for custom protocols and built-in message validators.

## Tech Stack

### Languages

- **Rust**: Core logic, performance-critical components, and WebSocket handling.
- **Python**: Primary user interface via high-level bindings (3.8 – 3.13 support).
- **JavaScript/TypeScript**: Documentation tooling and potential future bindings.

### Rust Core Libraries

| Category        | Libraries                                        |
| --------------- | ------------------------------------------------ |
| Async Runtime   | `tokio`                                          |
| Serialization   | `serde`, `serde_json`                            |
| Python Bindings | `pyo3`, `pyo3-async-runtimes`                    |
| WebSockets      | `tokio-tungstenite` (with `rustls`)              |
| HTTP            | `reqwest` (with `rustls-tls`, no native OpenSSL) |
| Error Handling  | `thiserror`, `anyhow`                            |
| Logging/Tracing | `tracing`, `tracing-subscriber`                  |
| Time/Date       | `chrono`                                         |
| Decimals        | `rust_decimal`                                   |
| Proc-macros     | `darling` for attribute parsing                  |
| Cross-Platform  | `UniFFI` (Kotlin, Swift, Go, Ruby, C#)           |

### Python Bindings

| Category           | Tools                      |
| ------------------ | -------------------------- |
| Build System       | `maturin`                  |
| Testing            | `pytest`, `pytest-asyncio` |
| Linting/Formatting | `ruff`                     |

### Infrastructure & Tooling

- **Version Control**: Git (GitHub)
- **CI/CD**: GitHub Actions
- **Documentation**: MkDocs (Material theme)
- **Containerization**: Docker (multi-platform builds)
- **Dependency Management**: Rust — `cargo`; Python — `pip`, `uv.lock`; JS — `bun`
- **Quality Control**: `husky`, `lint-staged`, `rustfmt`, `prettier`, `markdownlint`

## Build Commands

### Rust

```bash
cargo build                              # Debug build all crates
cargo build --release                    # Release build (LTO thin, opt-level 3)
cargo build -p binary_options_tools      # Specific crate
cargo build -p BinaryOptionsToolsV2 --features stubgen  # Build with .pyi stub generation
cargo clean                              # Clean artifacts
```

### Python (maturin)

```bash
maturin develop                          # Dev install (from BinaryOptionsToolsV2/)
maturin build --release                  # Build wheel
maturin build --release -i python3.13t   # Free-threaded Python build
maturin sdist                            # Source distribution
```

### UniFFI Bindings

```bash
cargo run -p uniffi-bindgen generate src/binary_options_tools_uni.udl --language kotlin --out-dir out/kotlin
cargo run -p uniffi-bindgen generate src/binary_options_tools_uni.udl --language swift --out-dir out/swift
```

## Test Commands

### Python (pytest)

```bash
pytest                                   # All tests (testpaths in pytest.ini)
pytest -v                                # Verbose
pytest -s                                # Show print output
pytest tests/python/pocketoption/test_synchronous.py::test_sync_manual_connect_shutdown  # Single test
pytest -m "pocketoption"                 # By marker
pytest --cov=BinaryOptionsToolsV2        # With coverage
```

- Config: `pytest.ini` sets `asyncio_mode = auto`, `timeout = 60`, testpaths = `tests/python/core tests/python/pocketoption tests/python/tracing`
- `conftest.py` loads `.env` for `POCKET_OPTION_SSID`; tests skip if not set
- Fixtures: `api` (async), `api_sync` — module-scoped, reuse connections
- Tests located in `tests/` directory

### Rust

```bash
cargo test                               # All tests
cargo test -p binary_options_tools       # Specific crate
cargo test test_name                     # By name
cargo test -- --nocapture                # Show output
cargo test --package binary_options_tools --lib framework::tests  # Framework tests
```

- Implement unit tests in each crate's `src` or `tests` directory
- Ensure all tests pass (`cargo test` and `pytest`) before submitting a PR
- Tests must be deterministic and use mocks for network calls where appropriate

## Lint & Format

### Python (Ruff)

```bash
ruff check .                             # Lint
ruff check --fix .                       # Lint + auto-fix
ruff format .                            # Format
ruff format --check .                    # Check formatting
```

- Line length: 120, target Python: 3.8+
- Config in `BinaryOptionsToolsV2/pyproject.toml`

### Rust

```bash
cargo fmt                                # Format (edition 2021 per .rustfmt.toml)
cargo fmt -- --check                     # Check formatting
cargo clippy --all-targets --all-features -- -D warnings  # Lint (deny warnings)
```

### Markdown

```bash
markdownlint-cli2 "**/*.md"              # Lint markdown
```

### Pre-commit (husky + lint-staged)

Runs on commit via `package.json` lint-staged config:

- `*.py` → `ruff check --fix` then `ruff format`
- `*.rs` → `rustfmt`

Install hooks: `bun install` (uses `bun@1.3.10`)

## Code Style — Rust

- **Edition**: 2021
- **Imports**: Group by std → external crates → internal modules; `use` at top of file
- **Naming**: `snake_case` functions/variables, `CamelCase` types, `SCREAMING_SNAKE_CASE` constants
- **Errors**: `thiserror` for custom error types, `anyhow::Result<T>` for app-level
- **Async**: `tokio` runtime; `#[tokio::test]` for async tests; `async_trait` for async trait methods
- **Logging**: `tracing` crate (`debug!`, `info!`, `warn!`, `error!`)
- **Serialization**: `serde` with `derive`; custom serializers in `utils/serialize.rs`
- **HTTP**: `reqwest` with `rustls-tls` (no native OpenSSL dependency)
- **WebSockets**: `tokio-tungstenite` with `rustls`
- **Proc-macros**: Use `darling` for attribute parsing; see `crates/macros`
- **Modules**: One module per file; `mod.rs` for submodules
- **Public APIs**: Explicit types; type inference OK in local scope
- **Python interop**: `#[pyfunction]`, `#[pymodule]`, `#[pyclass]`; convert errors with `PyErr::new::<PyRuntimeError, _>(msg)`
- **Documentation**: Triple-slash (`///`) doc comments for all public APIs
- **Warnings**: Fix all clippy warnings; no warnings allowed in the final code

## Code Style — Python

- **Version**: 3.8+
- **Formatter/Linter**: Ruff (replaces black + flake8 + isort)
- **Line length**: 120
- **Imports**: Ruff handles ordering (stdlib → third-party → local)
- **Naming**: `snake_case` functions/variables, `PascalCase` classes, `UPPER_SNAKE_CASE` constants
- **Type hints**: Required on function signatures; explicit imports from `typing`
- **Error handling**: `try/except` with specific exception types; no bare `except:`
- **Async**: `async def`/`await`; pytest-asyncio with `asyncio_mode=auto`
- **Docstrings**: Google or NumPy style; minimum: brief description + args/returns
- **Logging**: Use `BinaryOptionsToolsV2.tracing` bridge; avoid `print()` in library code
- **Stub files**: `.pyi` files generated from Rust via `stubgen` feature; Python wrapper should be thin

## Commit Conventions

- **Subject line**: Present tense, imperative mood ("Add feature" not "Added feature"), ≤ 72 characters
- **Body**: Detailed description of the "why" behind the change
- **Footer**: Reference issues using `Fixes #123` or `Closes #123`

See [CONTRIBUTING.md](../CONTRIBUTING.md) for full commit message guidelines and examples.

## Workflow & PRs

- **Branching**: Create feature branches from `master`
- **Pre-commit**: `husky` and `lint-staged` run automatic formatting and linting on commit
- **Testing**: Ensure all tests pass (`cargo test` and `pytest`) before submitting a PR
- **Documentation**: Update `docs/` and `README.md` if the change affects public behavior
- **Reviews**: All PRs require a clear description and must pass all CI checks

## Cross-language Conventions

- Business logic lives in Rust; Python wrapper is thin
- Keep API surface consistent between sync and async Python variants
- Errors surface as Python exceptions via PyO3 conversion
- Version managed in Cargo.toml; maturin reads it for Python package
- Stub generation: `stubgen` feature on `BinaryOptionsToolsV2` crate → `.pyi` in `python/BinaryOptionsToolsV2/`

## CI

- Builds wheels for Linux (manylinux, musllinux), Windows, macOS
- Runs pytest on x86
- Integration tests use isolated `test_run` directory

## Environment

- Package manager: `bun@1.3.10` (for dev tooling only — husky, lint-staged, markdownlint)
- Virtual env: `.venv/` (gitignored)
- `.env` file for secrets (`POCKET_OPTION_SSID`); never commit it
