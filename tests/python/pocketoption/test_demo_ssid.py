"""
Test the library with a demo SSID to verify is_connected, max_subscriptions,
subscription, and candle fetching work correctly.

Set the POCKET_OPTION_SSID environment variable before running:
    export POCKET_OPTION_SSID='42["auth",{"session":"...","isDemo":1,...}]'
    pytest tests/python/pocketoption/test_demo_ssid.py -v -s
"""

import contextlib
import os
import asyncio
import pytest
import pytest_asyncio

# These tests open a real PocketOption session, so they need both a valid
# POCKET_OPTION_SSID and a reachable broker. Marking the module keeps the
# offline suite runnable with `-k "not integration"`, as documented in
# pytest.ini.
pytestmark = pytest.mark.integration

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
    async def test_get_candles_live(self, api):
        """Test streaming live candles using get_candles_live."""
        from contextlib import aclosing

        generator = api.get_candles_live(
            "EURUSD_otc", period=60, hours=1.0, max_rows=10
        )
        async with aclosing(generator) as stream:
            try:
                closed, forming = await asyncio.wait_for(stream.__anext__(), timeout=20)
            except asyncio.TimeoutError:
                pytest.fail("Timed out waiting for first get_candles_live item")
                return
            assert isinstance(closed, list), "Expected list of closed candles"
            assert len(closed) > 0, "Expected at least one closed candle"
            assert forming is None or isinstance(forming, dict), (
                "Expected forming candle to be dict or None"
            )
            print(f"  get_candles_live closed count: {len(closed)}, forming: {forming}")

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

    @pytest.mark.timeout(60)
    @pytest.mark.asyncio
    async def test_get_candles_live_multiple_assets_concurrently(self, api):
        """Concurrent get_candles_live for several assets must each return history.

        Regression test for #141: the shared response queue used to make
        concurrent requests consume each other's responses, yielding empty
        closed-candle lists for every asset.
        """
        import asyncio
        from contextlib import aclosing

        assets = ["EURUSD_otc", "GBPUSD_otc", "USDJPY_otc", "XAUUSD_otc"]

        async def first_item(asset):
            generator = api.get_candles_live(asset, period=60, hours=1.0, max_rows=20)
            async with aclosing(generator) as stream:
                return await asyncio.wait_for(stream.__anext__(), timeout=30)

        results = await asyncio.gather(
            *(first_item(asset) for asset in assets), return_exceptions=True
        )
        for asset, result in zip(assets, results):
            assert not isinstance(result, Exception), f"{asset} failed: {result!r}"
            closed, forming = result
            print(f"  {asset}: {len(closed)} closed, forming={bool(forming)}")
            assert len(closed) > 0, f"Expected closed candles for {asset}"

    @pytest.mark.timeout(90)
    @pytest.mark.asyncio
    async def test_multi_asset_candles_repeated_with_abandoned_request(self, api):
        """Polling several assets repeatedly works, even after a poll is abandoned.

        Regression test for #141: concurrent candle requests used to consume
        each other's responses, returning empty history. A bot that polls on a
        timer also abandons in-flight requests (timeouts, cancellation), which
        must not break the polls that follow.
        """
        assets = ["EURUSD_otc", "GBPUSD_otc", "USDJPY_otc"]

        async def poll(asset):
            return await asyncio.wait_for(api.candles(asset, 60, count=20), timeout=30)

        for round_index in range(2):
            if round_index == 1:
                # Abandon a poll in flight, like a cancelled timer or shutdown.
                abandoned = asyncio.create_task(poll(assets[0]))
                await asyncio.sleep(1.0)
                abandoned.cancel()
                with contextlib.suppress(asyncio.CancelledError):
                    await abandoned

            results = await asyncio.gather(
                *(poll(asset) for asset in assets), return_exceptions=True
            )
            for asset, result in zip(assets, results):
                assert not isinstance(result, Exception), (
                    f"round {round_index} {asset} failed: {result!r}"
                )
                assert len(result) > 0, (
                    f"round {round_index}: expected candles for {asset}"
                )


if __name__ == "__main__":
    pytest.main([__file__, "-v", "-s"])
