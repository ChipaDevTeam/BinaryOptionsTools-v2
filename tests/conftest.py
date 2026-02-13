import sys
import os
import pytest
import asyncio

# Manual .env loader
env_path = os.path.join(os.path.dirname(__file__), "../.env")
if not os.path.exists(env_path):
    env_path = os.path.join(os.path.dirname(__file__), "../@.env")

if os.path.exists(env_path):
    print(f"\n[TEST_ENV] Loading environment from: {env_path}")
    with open(env_path, "r") as f:
        for line in f:
            line = line.strip()
            if not line or line.startswith("#"):
                continue
            if "=" in line:
                key, value = line.split("=", 1)
                # Remove potential quotes
                if (value.startswith("'") and value.endswith("'")) or (
                    value.startswith('"') and value.endswith('"')
                ):
                    value = value[1:-1]
                os.environ[key] = value
                if key == "POCKET_OPTION_SSID":
                    print(
                        f"[TEST_ENV] Found POCKET_OPTION_SSID (starts with {value[:10]}...)"
                    )
else:
    print(f"\n[TEST_ENV] No .env file found at {env_path}")

# Debug helper to verify import source
try:
    import BinaryOptionsToolsV2
    from BinaryOptionsToolsV2.pocketoption.asynchronous import PocketOptionAsync
    from BinaryOptionsToolsV2.pocketoption.synchronous import PocketOption

    print(
        f"\n[TEST_ENV] BinaryOptionsToolsV2 loaded from: {BinaryOptionsToolsV2.__file__}"
    )
except ImportError:
    print(
        "\n[TEST_ENV] BinaryOptionsToolsV2 not found in site-packages, attempting to load from source..."
    )
    # Add source directory to sys.path as a fallback
    source_path = os.path.join(
        os.path.dirname(__file__), "../BinaryOptionsToolsV2/python"
    )
    if source_path not in sys.path:
        sys.path.insert(0, source_path)

    try:
        import BinaryOptionsToolsV2
        from BinaryOptionsToolsV2.pocketoption.asynchronous import PocketOptionAsync
        from BinaryOptionsToolsV2.pocketoption.synchronous import PocketOption

        print(
            f"[TEST_ENV] BinaryOptionsToolsV2 loaded from source: {BinaryOptionsToolsV2.__file__}"
        )
    except ImportError as e:
        print(f"[TEST_ENV] CRITICAL: Failed to load BinaryOptionsToolsV2: {e}")
        print(f"[TEST_ENV] sys.path: {sys.path}")
        raise


@pytest.fixture(scope="module")
async def api():
    """Module-scoped fixture to reuse the PocketOption connection."""
    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        pytest.skip("POCKET_OPTION_SSID not set")

    config = {
        "connection_initialization_timeout_secs": 30,  # Reduced from 60
        "max_allowed_loops": 10,
        "timeout_secs": 60,
        "terminal_logging": False,
        "log_level": "WARN",
    }

    # We use PocketOptionAsync directly from the package
    async with PocketOptionAsync(ssid, config=config) as client:
        # Wait a bit for background modules to sync
        await asyncio.sleep(0.5)
        yield client


@pytest.fixture(scope="module")
def api_sync():
    """Module-scoped fixture to reuse the sync PocketOption connection."""
    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        pytest.skip("POCKET_OPTION_SSID not set")

    config = {
        "connection_initialization_timeout_secs": 30,
        "max_allowed_loops": 10,
        "timeout_secs": 60,
        "terminal_logging": False,
        "log_level": "WARN",
    }

    with PocketOption(ssid, config=config) as client:
        yield client
