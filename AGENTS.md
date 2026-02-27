# Agent Guidelines for BinaryOptionsTools-v2

## Build Commands

### Rust

- Build all crates: `cargo build`
- Build release: `cargo build --release`
- Build specific crate: `cargo build -p binary_options_tools`
- Build Python extension (from BinaryOptionsToolsV2): `maturin build --release`
- Build free-threaded Python: `maturin build --release -i python3.13t`
- Build sdist: `maturin sdist`
- Build with stub generation: `cargo build -p BinaryOptionsToolsV2 --features stubgen`
- Generate stubs only: `cargo build -p BinaryOptionsToolsV2 --features stubgen --target-dir target/stubgen`
- Clean build artifacts: `cargo clean`

### Python Stub Generation

- Enable stubgen feature to auto-generate `.pyi` files via build script
- Stubs are placed in `BinaryOptionsToolsV2/python/BinaryOptionsToolsV2/`
- Stub files are automatically included in wheel via `pyproject.toml` maturin config
- Manual generation (alternative): Use `pyo3-stub-gen` CLI tool if needed

### Python

- Install in dev mode (from BinaryOptionsToolsV2): `maturin develop`
- Install from wheel: `pip install BinaryOptionsToolsV2 --find-links dist --force-reinstall`
- Create virtualenv: `python3 -m venv .venv`

## Test Commands

### Python Tests (pytest)

- Run all tests: `pytest`
- Run with verbose: `pytest -v`
- Run specific test file: `pytest tests/python/pocketoption/test_synchronous.py`
- Run specific test function: `pytest tests/python/pocketoption/test_synchronous.py::test_sync_manual_connect_shutdown`
- Run with marker: `pytest -m "pocketoption"`
- Run with output: `pytest -s`
- Run with coverage: `pytest --cov=BinaryOptionsToolsV2`

### Rust Tests

- Run all tests: `cargo test`
- Run specific crate tests: `cargo test -p binary_options_tools`
- Run specific test: `cargo test test_name`
- Run with output: `cargo test -- --nocapture`

### Note on Test Setup

Python tests rely on `POCKET_OPTION_SSID` environment variable (optional). Tests skip if not set. Tests create a `test_run` subdirectory. Configuration in `BinaryOptionsToolsV2/pyproject.toml` sets `asyncio_mode = "auto"` and testpaths to `../tests`.

## Lint/Format Commands

### Python (Ruff)

- Lint all: `ruff check .`
- Lint with fix: `ruff check --fix .`
- Format: `ruff format .`
- Check formatting only: `ruff format --check .`
- Lint staged files (git): `lint-staged` (configured in package.json)
- Target Python: 3.8+, line length: 120

### Rust

- Format: `cargo fmt` or `rustfmt`
- Check formatting: `cargo fmt -- --check`
- Lint: `cargo clippy --all-targets --all-features -- -D warnings`
- Lint specific crate: `cargo clippy -p binary_options_tools`

### Markdown

- Lint: `markdownlint-cli2 "**/*.md"` or `markdownlint-cli "**/*.md"`

## Code Style Guidelines

### Rust

- **Edition**: 2021
- **Formatting**: rustfmt with default settings (`.rustfmt.toml` minimal)
- **Imports**: Group std, external crates, internal modules; use `use` statements at top of file
- **Naming**: snake_case for functions/variables, CamelCase for types, SCREAMING_SNAKE_CASE for constants
- **Error Handling**: Use `thiserror` for custom errors, `anyhow::Result<T>` for application-level
- **Types**: Prefer explicit types in public APIs, type inference allowed in local scope
- **Async**: Use `tokio` runtime; mark test functions with `#[tokio::test]`
- **Modules**: One module per file, use `mod.rs` for submodules
- **Traits**: Use `async_trait` for async trait methods
- **Logging**: Use `tracing` crate with `debug!`, `info!`, `warn!`, `error!`
- **Serialization**: Use `serde` with `derive` feature; custom serializers in `utils/serialize.rs`
- **Dependencies**: Keep minimal; prefer `reqwest` with `rustls-tls` (no native deps)
- **Stub generation**: Add `pyo3-stub-gen` as optional dependency with `stubgen` feature; use build script to generate `.pyi` files automatically on build
- **Proc-macros**: Use `darling` for attribute parsing; follow existing patterns in `crates/macros`

### Python

- **Version**: 3.8+
- **Formatter/Linter**: Ruff (fast, replaces black + flake8 + isort)
- **Line length**: 120
- **Imports**: ruff handles ordering (standard library, third-party, local)
- **Naming**: snake_case for functions/variables, PascalCase for classes, UPPER_SNAKE_CASE for constants
- **Type hints**: Use type annotations for function signatures; prefer explicit imports from `typing`
- **Error handling**: Use try/except with specific exception types; avoid bare `except:`
- **Async**: Use `async def` and `await`; `pytest-asyncio` for tests with `@pytest.mark.asyncio` optional (asyncio_mode=auto)
- **Docstrings**: Google or NumPy style recommended; minimum: brief description + args/returns
- **Logging**: Use `tracing` bridge via `BinaryOptionsToolsV2.tracing` module; avoid `print()` in library code
- **Stub files**: Generated `.pyi` files from Rust provide type hints; Python wrapper should be thin

### Cross-language (Rust <-> Python)

- Python bindings via PyO3; API surface defined in Rust crates
- Keep Python wrapper thin; business logic in Rust
- Use `#[pyfunction]`, `#[pymodule]`, `#[pyclass]` attributes
- Convert errors with `PyErr::new::<PyRuntimeError, _>(msg)`
- Use `maturin` for build/distribution; version managed in Cargo.toml
- Stub generation enabled via `stubgen` feature flag on `BinaryOptionsToolsV2` crate
- Stub files placed in python package dir and included via maturin `include` config

## Project Structure

- `crates/`: Rust crates (core, macros, binary_options_tools)
- `BinaryOptionsToolsV2/`: Python package with maturin build
- `BinaryOptionsToolsUni/`: UniFFI bindings for multi-language support
- `tests/`: Python tests (mirrors package structure)
- `data/`: Test fixtures and JSON data
- `docs/`: MkDocs documentation

## Conventions Observed

- Rust macros in `crates/macros` with `darling` for attribute parsing
- Timeout handling via `#[timeout(secs)]` macro
- Config structs derive `Config` macro for validation
- Regions/actions use derive macros `RegionImpl`, `ActionImpl`
- WebSocket client uses tokio-tungstenite with rustls
- Tests often skip without live credentials; use mocking where possible
- Python tests use fixtures from `conftest.py`; module-scoped fixtures for connection reuse
- CI builds wheels for Linux (manylinux, musllinux), Windows, macOS; runs pytest on x86

## Running a Single Test

- Python: `pytest tests/python/pocketoption/test_synchronous.py::test_sync_manual_connect_shutdown`
- Rust: `cargo test test_deserialize_macro --package binary_options_tools`

## Pre-commit Hooks

husky + lint-staged configured to run:

- Python: `ruff check --fix` then `ruff format`
- Rust: `rustfmt`

Install with `pnpm install` (package.json present for tooling).

## Notes

- No Cursor rules or Copilot instructions found in repository.
- Keep Rust edition consistent (2021).
- Use maturin for Python packaging; do not manually compile extension modules.
- For CI, tests run in isolated `test_run` directory; follow this pattern for integration tests.
