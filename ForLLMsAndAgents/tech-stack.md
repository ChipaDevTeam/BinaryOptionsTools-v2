# Tech Stack: BinaryOptionsTools-v2

## Languages

- **Rust**: Core logic, performance-critical components, and WebSocket handling.
- **Python**: Primary user interface via high-level bindings (3.8 - 3.13 support).
- **JavaScript/TypeScript**: Used for documentation tooling and potential future bindings.

## Frameworks & Libraries

### Rust Core

- **Async Runtime**: `tokio`
- **Serialization**: `serde`, `serde_json`
- **Python Bindings**: `pyo3`, `pyo3-async-runtimes`
- **WebSockets**: `tungstenite`
- **Error Handling**: `thiserror`
- **Logging/Tracing**: `tracing`, `tracing-subscriber`
- **Time/Date**: `chrono`
- **Decimals**: `rust_decimal`
- **Cross-Platform**: `UniFFI` (for Kotlin, Swift, Go, Ruby, C#)

### Python Bindings

- **Build System**: `maturin`
- **Testing**: `pytest`, `pytest-asyncio`
- **Linting/Formatting**: `ruff`

## Infrastructure & Tooling

- **Version Control**: Git (GitHub)
- **CI/CD**: GitHub Actions
- **Documentation**: MkDocs (Material theme)
- **Containerization**: Docker (multi-platform builds)
- **Dependency Management**:
  - Rust: `cargo`
  - Python: `pip`, `uv.lock`
  - JS: `pnpm`
- **Quality Control**: `husky`, `lint-staged`, `rustfmt`, `prettier`, `markdownlint`
