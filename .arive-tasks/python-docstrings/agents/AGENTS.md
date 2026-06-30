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

### Rust

```bash
cargo test                               # All tests
cargo test -p binary_options_tools       # Specific crate
cargo test test_name                     # By name
cargo test -- --nocapture                # Show output
cargo test --package binary_options_tools --lib framework::tests  # Framework tests
```

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

## Cross-language Conventions

- Business logic lives in Rust; Python wrapper is thin
- Keep API surface consistent between sync and async Python variants
- Errors surface as Python exceptions via PyO3 conversion
- Version managed in Cargo.toml; maturin reads it for Python package
- Stub generation: `stubgen` feature on `BinaryOptionsToolsV2` crate → `.pyi` in `python/BinaryOptionsToolsV2/`

## Commit Messages

- Present tense, imperative mood: "Add feature" not "Added feature"
- First line ≤ 72 chars
- Reference issues: `Fixes #123`

## CI

- Builds wheels for Linux (manylinux, musllinux), Windows, macOS
- Runs pytest on x86
- Integration tests use isolated `test_run` directory

## Environment

- Package manager: `bun@1.3.10` (for dev tooling only — husky, lint-staged, markdownlint)
- Virtual env: `.venv/` (gitignored)
- `.env` file for secrets (`POCKET_OPTION_SSID`); never commit it
