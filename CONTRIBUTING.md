# Contributing to BinaryOptionsTools v2

First off, thank you for considering contributing to BinaryOptionsTools v2! It's people like you that make this library a great tool for the community.

## Code of Conduct

This project and everyone participating in it is governed by our [Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code. Please report unacceptable behavior to our [Discord community](https://discord.gg/p7YyFqSmAz).

## How Can I Contribute?

### Reporting Bugs

Before creating bug reports, please check the [existing issues](https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/issues) as you might find that you don't need to create one. When you are creating a bug report, please include as many details as possible:

* **Use a clear and descriptive title** for the issue
* **Describe the exact steps to reproduce the problem** with as many details as possible
* **Provide specific examples** to demonstrate the steps
* **Describe the behavior you observed** and what behavior you expected to see
* **Include code samples and error messages** if applicable
* **Specify your environment**: OS, Python version, library version, etc.

### Suggesting Enhancements

Enhancement suggestions are tracked as [GitHub issues](https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/issues). When creating an enhancement suggestion, please include:

* **Use a clear and descriptive title**
* **Provide a detailed description of the suggested enhancement**
* **Provide specific examples** to demonstrate the use case
* **Explain why this enhancement would be useful** to most users

### Pull Requests

Please follow these steps for sending us your pull requests:

1. **Fork the repository** and create your branch from `master`
2. **Make your changes** following our coding standards
3. **Add tests** for any new functionality
4. **Ensure all tests pass** (`cargo test` and `pytest`)
5. **Update documentation** if you're changing functionality
6. **Write clear commit messages** describing your changes
7. **Create a pull request** with a clear description of your changes

## Development Setup

### Prerequisites

* **Rust**: 1.70 or later (install via [rustup](https://rustup.rs/))
* **Python**: 3.8 or later
* **Maturin**: `pip install maturin`

### Setting Up Your Development Environment

```bash
# Clone your fork
git clone https://github.com/YOUR_USERNAME/BinaryOptionsTools-v2.git
cd BinaryOptionsTools-v2

# Build the Rust core
cd crates/binary_options_tools
cargo build
cargo test

# Build Python bindings
cd ../../BinaryOptionsToolsV2
maturin develop --release

# Run Python tests
pytest ../tests/
```

## Coding Standards

### Rust Code

* Follow the [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/)
* Run `cargo fmt` before committing
* Run `cargo clippy` and fix all warnings
* Write tests for new functionality
* Document public APIs with doc comments

### Python Code

* Follow [PEP 8](https://www.python.org/dev/peps/pep-0008/) style guide
* Use type hints where appropriate
* Write docstrings for all public functions and classes
* Keep line length under 120 characters (as configured in pyproject.toml)

### Commit Messages

* Use the present tense ("Add feature" not "Added feature")
* Use the imperative mood ("Move cursor to..." not "Moves cursor to...")
* Limit the first line to 72 characters or less
* Reference issues and pull requests liberally after the first line

Example:
```
Add WebSocket reconnection with exponential backoff

- Implement exponential backoff strategy for reconnection
- Add max retry configuration
- Update tests for reconnection logic

Fixes #123
```

## Testing

### Running Tests

```bash
# Rust tests
cd crates/binary_options_tools
cargo test

# Python tests
cd BinaryOptionsToolsV2
pytest ../tests/

# Run specific test
pytest ../tests/test_connection.py -v
```

### Writing Tests

* Write unit tests for all new functionality
* Ensure tests are deterministic and don't require external services when possible
* Use mocking for WebSocket connections when appropriate
* Add integration tests for critical paths

## Documentation

* Update the README.md if you change functionality
* Add examples for new features in the `examples/` directory
* Update relevant documentation in the `docs/` directory
* Ensure all public APIs have docstrings/doc comments

## Community

* Join our [Discord server](https://discord.gg/p7YyFqSmAz) for discussions
* Be respectful and constructive in all interactions
* Help others when you can
* Share your use cases and experiences

## License

By contributing, you agree that your contributions will be licensed under the same license as the project (see [LICENSE](LICENSE) file).

## Questions?

Don't hesitate to ask questions on our [Discord server](https://discord.gg/p7YyFqSmAz) or by opening an issue. We're here to help!

Thank you for your contributions! 🎉
