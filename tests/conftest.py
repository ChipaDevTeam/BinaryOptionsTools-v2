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
    # Force removal of source directory from sys.path to ensure we test the installed package
    import sys
    import os

    original_path = sys.path[:]
    sys.path = [
        p
        for p in sys.path
        if not p.endswith("BinaryOptionsToolsV2/python")
        and "BinaryOptionsToolsV2/python" not in p
    ]

    import BinaryOptionsToolsV2  # noqa: E402
    from BinaryOptionsToolsV2.pocketoption.asynchronous import PocketOptionAsync  # noqa: E402
    from BinaryOptionsToolsV2.pocketoption.synchronous import PocketOption  # noqa: E402

    print(
        f"\n[TEST_ENV] BinaryOptionsToolsV2 loaded from: {BinaryOptionsToolsV2.__file__}"
    )
    if "BinaryOptionsToolsV2/python" in BinaryOptionsToolsV2.__file__:
        print(
            "[TEST_ENV] WARNING: Loading from source directory instead of installed package!"
        )
        print(f"[TEST_ENV] current sys.path: {sys.path}")
except Exception as e:
    print(f"\n[TEST_ENV] Failed to load BinaryOptionsToolsV2: {e}")
    print(f"\n[TEST_ENV] Original sys.path was: {original_path}")


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
