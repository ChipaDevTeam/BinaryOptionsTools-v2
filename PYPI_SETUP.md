# PyPI Publishing Setup Guide

This guide will help you set up automated publishing to PyPI for your BinaryOptionsToolsV2 package.

## Prerequisites

1. **PyPI Account**: Create an account at [PyPI](https://pypi.org/account/register/)
2. **Test PyPI Account** (optional but recommended): Create an account at [Test PyPI](https://test.pypi.org/account/register/)

## Setting up Trusted Publishing (Recommended)

Trusted publishing is the modern, secure way to publish to PyPI without storing API tokens.

### Step 1: Configure PyPI Trusted Publishing

1. Go to your PyPI account settings: <https://pypi.org/manage/account/>
2. Navigate to "Publishing" tab
3. Click "Add a new pending publisher"
4. Fill in the details:
   - **PyPI Project Name**: `BinaryOptionsToolsV2`
   - **Owner**: `ChipaDevTeam`
   - **Repository name**: `BinaryOptionsTools-v2`
   - **Workflow name**: `release.yml`
   - **Environment name**: `pypi`

### Step 2: Set up GitHub Environment

1. Go to your GitHub repository settings
2. Navigate to "Environments" in the sidebar
3. Click "New environment"
4. Name it `pypi`
5. Add environment protection rules if desired (e.g., require manual approval)

## Alternative: API Token Method

If you prefer using API tokens instead of trusted publishing:

### Step 1: Generate API Token

1. Go to PyPI Account Settings: <https://pypi.org/manage/account/>
2. Scroll to "API tokens" section
3. Click "Add API token"
4. Give it a name (e.g., "GitHub Actions")
5. Set scope to "Entire account" or specific to your project
6. Copy the generated token (starts with `pypi-`)

### Step 2: Add Token to GitHub Secrets

1. Go to your GitHub repository settings
2. Navigate to "Secrets and variables" → "Actions"
3. Click "New repository secret"
4. Name: `PYPI_API_TOKEN`
5. Value: Your PyPI API token

### Step 3: Update Workflow

If using API tokens, update the release workflow to use:

```yaml
- name: Publish to PyPI
  env:
    TWINE_USERNAME: __token__
    TWINE_PASSWORD: ${{ secrets.PYPI_API_TOKEN }}
  run: |
    pip install twine
    twine upload dist/*
```

## How to Release

### Method 1: GitHub Releases (Recommended)

1. Go to your GitHub repository
2. Click "Releases" → "Create a new release"
3. Choose or create a new tag (e.g., `v0.1.9`)
4. Fill in release title and description
5. Click "Publish release"
6. The release workflow will automatically trigger and publish to PyPI

### Method 2: Git Tags

```bash
git tag v0.1.9
git push origin v0.1.9
```

### Method 3: Manual Trigger

1. Go to "Actions" tab in your GitHub repository
2. Select "Release" workflow
3. Click "Run workflow"
4. Enter the tag name you want to release

## Testing Before Release

### Test on Test PyPI First

1. Set up Test PyPI the same way as PyPI
2. Create a test workflow pointing to Test PyPI:

   ```yaml
   - name: Publish to Test PyPI
     uses: pypa/gh-action-pypi-publish@release/v1
     with:
       repository-url: https://test.pypi.org/legacy/
   ```

### Local Testing

```bash
cd BinaryOptionsToolsV2
maturin build --release
pip install target/wheels/BinaryOptionsToolsV2-*.whl
python -c "import BinaryOptionsToolsV2; print('Success!')"
```

## Supported Platforms

The current setup builds wheels for:

### Linux

- x86_64 (Intel/AMD 64-bit)
- x86 (Intel/AMD 32-bit)
- aarch64 (ARM 64-bit)
- armv7 (ARM 32-bit)
- s390x (IBM Z)
- ppc64le (PowerPC 64-bit Little Endian)

### Linux (musl libc)

- x86_64
- x86
- aarch64
- armv7

### Windows

- x64 (Intel/AMD 64-bit)
- x86 (Intel/AMD 32-bit)

### macOS

- x86_64 (Intel)
- aarch64 (Apple Silicon)

## Troubleshooting

### Common Issues

1. **Build Failures**: Check the Actions logs for specific error messages
2. **PyPI Upload Errors**: Ensure version number is incremented in `Cargo.toml`
3. **Permission Denied**: Check that trusted publishing is set up correctly
4. **Missing Dependencies**: Ensure all Rust dependencies are properly specified

### Checking Build Status

Monitor your builds at: `https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/actions`

### Version Management

Remember to update the version in `BinaryOptionsToolsV2/Cargo.toml` before each release:

```toml
[package]
version = "0.1.9"  # Increment this
```

## Security Notes

- Never commit API tokens to your repository
- Use trusted publishing when possible
- Regularly rotate API tokens
- Monitor your PyPI project for unauthorized changes
