# Guidelines: BinaryOptionsTools-v2

## Code Style

### Rust

- **Formatting**: Adhere to the [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/).
- **Tools**: Always run `cargo fmt` and `cargo clippy` before committing.
- **Warnings**: Fix all clippy warnings; no warnings allowed in the final code.
- **Documentation**: Use triple-slash (`///`) doc comments for all public APIs.

### Python

- **Formatting**: Follow [PEP 8](https://www.python.org/dev/peps/pep-0008/).
- **Line Length**: Maximum of 120 characters (enforced by `ruff`).
- **Typing**: Use type hints for all function signatures and complex variables.
- **Documentation**: Provide docstrings for all public classes, methods, and functions.

## Commit Conventions

- **Format**: [Subject Line]

 [Body]

 [Footer/Issues]

- **Subject Line**:
  - Limit to 72 characters.
  - Use imperative mood ("Add", "Fix", "Update").
  - Present tense ("Add feature", not "Added feature").
- **Body**: Detailed description of the "why" behind the change.
- **Footer**: Reference issues using "Fixes #123" or "Closes #123".

## Testing Standards

- **Rust**: Implement unit tests in each crate's `src` or `tests` directory.
- **Python**: Use `pytest` for unit and integration tests (located in `tests/`).
- **Automation**: Ensure all tests pass (`cargo test` and `pytest`) before submitting a PR.
- **Quality**: Tests must be deterministic and use mocks for network calls where appropriate.

## Workflow & PRs

- **Branching**: Create feature branches from `master`.
- **Pre-commit**: Use `husky` and `lint-staged` for automatic formatting and linting checks.
- **Documentation**: Update `docs/` and `README.md` if the change affects public behavior.
- **Reviews**: All PRs require a clear description and should pass all CI checks.
