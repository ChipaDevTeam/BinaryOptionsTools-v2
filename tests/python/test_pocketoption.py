import pytest
import os
from BinaryOptionsToolsV2 import PocketOption, PocketOptionAsync, Config

# Demo SSID for testing - mirrors Rust tests
DEMO_SSID = "swap-ssid-for-testing-1234567890abcdef"


@pytest.fixture
def ssid():
    return os.environ.get("POCKETOPTION_SSID", DEMO_SSID)


def test_sync_connection_basic(ssid):
    """Test synchronous connection and basic info retrieval."""
    try:
        api = PocketOption(ssid)

        # Test basic properties
        assert api.is_demo() is True

        # Test balance (should be positive for a real demo account)
        balance = api.balance()
        assert float(balance) >= 0

        # Test server time
        server_time = api.server_time()
        assert server_time > 0

        api.close()
    except Exception as e:
        if "Authentication rejected" in str(e) or "swap-ssid" in ssid:
            pytest.skip(f"Skipping test: No valid credentials provided. Error: {e}")
        else:
            raise


@pytest.mark.asyncio
async def test_async_connection_basic(ssid):
    """Test asynchronous connection and basic info retrieval."""
    try:
        async with PocketOptionAsync(ssid) as api:
            # Test basic properties
            assert api.is_demo() is True

            # Test balance
            balance = await api.balance()
            assert float(balance) >= 0

            # Test server time
            server_time = await api.server_time()
            assert server_time > 0
    except Exception as e:
        if "Authentication rejected" in str(e) or "swap-ssid" in ssid:
            pytest.skip(f"Skipping test: No valid credentials provided. Error: {e}")
        else:
            raise


def test_config_parity():
    """Verify that Python Config maps correctly to Rust PyConfig."""
    config = Config(max_allowed_loops=10, timeout_secs=30)
    assert config.max_allowed_loops == 10
    assert config.timeout_secs == 30

    # Check that it produces a valid pyconfig
    pyconfig = config.pyconfig
    assert pyconfig.max_allowed_loops == 10
    assert pyconfig.timeout_secs == 30
