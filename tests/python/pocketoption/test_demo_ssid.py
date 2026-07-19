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
import pytest_asyncio

SSID = os.getenv("POCKET_OPTION_SSID")


@pytest_asyncio.fixture(loop_scope="module")  # type: ignore[misc]
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
        "max_allowed_loops": 0,  # Unlimited reconnection attempts
        "timeout_secs": 60,
    }
    client = PocketOptionAsync(SSID, config=config)
    # Wait for connection to stabilize and assets to load
    await asyncio.sleep(5)
    # Verify connection is established
    if not client.is_connected():
        print("Warning: Client not connected after 5 seconds")
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
        """Test is_demo returns a boolean."""
        is_demo = api.is_demo()
        print(f"  is_demo: {is_demo}")
        assert isinstance(is_demo, bool), "Expected boolean from is_demo()"

    @pytest.mark.asyncio
    async def test_balance(self, api):
        """Test getting balance."""
        import asyncio

        # Wait for balance to update from -1 to actual value
        balance = -1.0
        for _ in range(10):  # Try for up to 10 seconds
            balance = await api.balance()
            if balance >= 0:
                break
            await asyncio.sleep(1)
        print(f"  balance: {balance}")
        assert isinstance(balance, (int, float)), "Expected numeric balance"
        assert balance >= 0, f"Expected non-negative balance, got {balance}"

    @pytest.mark.asyncio
    async def test_candles(self, api):
        """Test fetching candles."""
        candles = await api.candles("EURUSD_otc", 60)
        print(f"  Got {len(candles)} candles")
        assert len(candles) > 0, "Expected at least one candle"
        first = candles[0]
        print(f"  First candle: {first}")

    @pytest.mark.asyncio
    async def test_compile_candles(self, api):
        """Test compiling custom-period candles from tick history."""
        candles = await api.compile_candles("EURUSD_otc", 20, 300)
        print(f"  Compiled {len(candles)} custom 20s candles")
        assert isinstance(candles, list), "Expected list of candles"
        assert len(candles) > 0, "Expected at least one compiled candle"
        first = candles[0]
        print(f"  First compiled candle: {first}")
        assert "timestamp" in first or "time" in first, (
            "Expected timestamp/time in compiled candle"
        )
        for key in ["open", "high", "low", "close"]:
            assert key in first, f"Expected key '{key}' in compiled candle"

    @pytest.mark.asyncio
    async def test_subscribe(self, api):
        """Test subscribing to a symbol and receiving data."""
        stream = await api.subscribe_symbol("EURUSD_otc")
        try:
            candle = await asyncio.wait_for(stream.__anext__(), timeout=10.0)
            print(f"  Received candle: {candle}")
        except asyncio.TimeoutError:
            pytest.fail("Timed out waiting for subscription data")
        finally:
            if hasattr(stream, "cancel"):
                stream.cancel()

    @pytest.mark.asyncio
    async def test_many_subscriptions(self, api):
        """Test subscribing to many symbols concurrently."""
        assets = ["EURUSD_otc", "AUDJPY_otc", "USDCAD_otc", "XNGUSD_otc", "ETHUSD_otc"]
        streams = []
        try:
            for asset in assets:
                print(f"  Subscribing to {asset}...")
                stream = await api.subscribe_symbol(asset)
                streams.append(stream)
            print(f"  Successfully subscribed to {len(streams)} assets concurrently.")
            assert len(streams) == len(assets), "Expected to subscribe to all assets"
        finally:
            for stream in streams:
                if hasattr(stream, "cancel"):
                    stream.cancel()


if __name__ == "__main__":
    pytest.main([__file__, "-v", "-s"])
