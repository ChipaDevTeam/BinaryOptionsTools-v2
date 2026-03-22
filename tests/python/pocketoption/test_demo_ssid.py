"""
Test the library with a demo SSID to verify is_connected, max_subscriptions,
subscription, and candle fetching work correctly.

Set the POCKET_OPTION_SSID environment variable before running:
    export POCKET_OPTION_SSID='42["auth",{"session":"...","isDemo":1,...}]'
    pytest tests/python/pocketoption/test_demo_ssid.py -v -s
"""

import os
import asyncio
import pytest

SSID = os.getenv("POCKET_OPTION_SSID")


@pytest.fixture(scope="module")
def event_loop():
    loop = asyncio.new_event_loop()
    yield loop
    loop.close()


@pytest.fixture(scope="module")
async def api():
    """Create async client with SSID from environment."""
    if not SSID:
        pytest.skip("POCKET_OPTION_SSID not set")
    from BinaryOptionsToolsV2.pocketoption.asynchronous import PocketOptionAsync

    config = {
        "connection_initialization_timeout_secs": 30,
        "max_allowed_loops": 10,
        "timeout_secs": 60,
    }
    client = PocketOptionAsync(SSID, config=config)
    await asyncio.sleep(3)
    yield client
    await client.shutdown()


class TestDemoConnection:
    @pytest.mark.asyncio
    async def test_is_connected(self, api):
        """Test is_connected returns True after connection."""
        connected = api.is_connected()
        print(f"  is_connected: {connected}")
        assert connected is True, "Expected to be connected"

    @pytest.mark.asyncio
    async def test_is_demo(self, api):
        """Test is_demo returns True for demo SSID."""
        is_demo = api.is_demo()
        print(f"  is_demo: {is_demo}")
        assert is_demo is True, "Expected demo account"

    @pytest.mark.asyncio
    async def test_max_subscriptions_default(self, api):
        """Test max_subscriptions returns default value of 4."""
        max_subs = api.max_subscriptions()
        print(f"  max_subscriptions: {max_subs}")
        assert max_subs == 4, f"Expected 4, got {max_subs}"

    @pytest.mark.asyncio
    async def test_balance(self, api):
        """Test getting balance."""
        balance = await api.balance()
        print(f"  balance: {balance}")
        assert balance > 0, "Expected positive balance"

    @pytest.mark.asyncio
    async def test_candles(self, api):
        """Test fetching candles."""
        candles = await api.candles("EURUSD_otc", 60)
        print(f"  Got {len(candles)} candles")
        assert len(candles) > 0, "Expected at least one candle"
        first = candles[0]
        print(f"  First candle: {first}")

    @pytest.mark.asyncio
    async def test_subscribe(self, api):
        """Test subscribing to a symbol and receiving data."""
        stream = await api.subscribe_symbol("EURUSD_otc")
        try:
            candle = await asyncio.wait_for(stream.__anext__(), timeout=10.0)
            print(f"  Received candle: {candle}")
        except asyncio.TimeoutError:
            pytest.fail("Timed out waiting for subscription data")


@pytest.mark.asyncio
async def test_custom_max_subscriptions():
    """Test configuring max_subscriptions via config."""
    if not SSID:
        pytest.skip("POCKET_OPTION_SSID not set")
    from BinaryOptionsToolsV2.pocketoption.asynchronous import PocketOptionAsync

    config = {
        "connection_initialization_timeout_secs": 30,
        "max_allowed_loops": 10,
        "timeout_secs": 60,
        "max_subscriptions": 8,
    }
    client = PocketOptionAsync(SSID, config=config)
    await asyncio.sleep(3)

    max_subs = client.max_subscriptions()
    print(f"  max_subscriptions: {max_subs}")
    assert max_subs == 8, f"Expected 8, got {max_subs}"

    connected = client.is_connected()
    print(f"  is_connected: {connected}")
    assert connected is True

    await client.shutdown()


if __name__ == "__main__":
    pytest.main([__file__, "-v", "-s"])
