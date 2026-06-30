# Pull Request

## Description

Provide a clear and concise description of the changes introduced by this PR. Detail the motivation, context, and any architectural or behavioral impacts of the change.

## Related Issues

Fixes # (issue number)
Closes # (issue number)

## Type of Change

- [ ] Bug fix (non-breaking change which fixes an issue)
- [ ] New feature (non-breaking change which adds functionality)
- [ ] Breaking change (fix or feature that would cause existing functionality to not work as expected)
- [ ] Documentation / Examples update
- [ ] Performance improvement / Code refactoring
- [ ] CI/CD / Build system configuration

## Validation and Testing

Describe the tests performed to verify your changes. Provide instructions so others can reproduce. Please also list any relevant details for your testing configuration.

- **Test Suite**: Run `cargo test` and `pytest` locally to verify changes.
- **Evidence**: Paste test run outputs, benchmark results, or code snippets verifying the fix/feature here.

```bash
# Example verification command used
pytest tests/python/pocketoption/
```

## Checklist

- [ ] My code follows the project's coding style guidelines (Ruff for Python, `cargo fmt` for Rust).
- [ ] I have performed a self-review of my own code.
- [ ] I have commented my code, particularly in hard-to-understand areas.
- [ ] I have made corresponding changes to the documentation.
- [ ] My changes generate no new compiler or linter warnings.
- [ ] I have added tests that prove my fix is effective or that my feature works.
- [ ] New and existing unit/integration tests pass locally.

## AI Usage Disclosure

- [ ] I used AI assistance to write or understand this PR/issue.
      If yes, please specify which tool(s) and what parts were AI-assisted:
