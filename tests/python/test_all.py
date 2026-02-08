import pytest
import os
import sys

import asyncio
from BinaryOptionsToolsV2.pocketoption.asynchronous import PocketOptionAsync

# Get SSID from environment variable
SSID = os.getenv("POCKET_OPTION_SSID")


@pytest.fixture
async def api():
    if not SSID:
        pytest.skip("POCKET_OPTION_SSID not set")

    # Use context manager which waits for assets automatically
    # Add timeout for connection initialization
    config = {"connection_initialization_timeout_secs": 60, "timeout_secs": 20}
    async with PocketOptionAsync(SSID, config=config) as client:
        yield client


@pytest.mark.asyncio
async def test_balance(api):
    """Test retrieving balance."""
    try:
        balance = await api.balance()
        assert isinstance(balance, (int, float))
        print(f"Balance: {balance}")
    except Exception as e:
        pytest.fail(f"Failed to get balance: {e}")


@pytest.mark.asyncio
async def test_server_time(api):
    """Test retrieving server time."""
    try:
        # Give the websocket 2 seconds to receive the time sync packet
        await asyncio.sleep(2)

        time = await asyncio.wait_for(api.get_server_time(), timeout=10.0)
        assert isinstance(time, (int, float))
        assert time > 1577836800  # 2020-01-01
        print(f"Server time: {time}")
    except asyncio.TimeoutError:
        pytest.fail(
            "Timed out getting server time - server time may not be initialized"
        )
    except Exception as e:
        pytest.fail(f"Failed to get server time: {e}")


@pytest.mark.asyncio
async def test_is_demo(api):
    """Test checking if account is demo."""
    try:
        is_demo = api.is_demo()
        assert isinstance(is_demo, bool)
        print(f"Is Demo: {is_demo}")
    except Exception as e:
        pytest.fail(f"Failed to check is_demo: {e}")


@pytest.mark.asyncio
async def test_buy_and_check_win(api):
    """Test buying an asset and checking the result."""
    if not api.is_demo():
        pytest.skip("Skipping trade test on real account to avoid losing money")

    asset = "EURUSD_otc"  # OTC is usually available on weekends too
    amount = 1.0
    duration = 5

    # Check if we can get payout for this asset to ensure it's valid
    try:
        payout = await api.payout(asset)
        if not payout:
            pytest.skip(f"Asset {asset} not available or no payout")
    except Exception:
        pytest.skip(f"Could not check payout for {asset}")

    print(f"Buying {asset} for {duration} seconds...")
    try:
        # Buy without waiting for result first
        trade_id, trade_info = await api.buy(asset, amount, duration, check_win=False)
        assert trade_id
        assert isinstance(trade_info, dict)
        print(f"Trade placed: {trade_id}")

        # Now wait for result using check_win
        print(f"Waiting for trade result (timeout: {duration + 60.0}s)...")
        try:
            # Use a reasonable timeout to prevent hanging - should be at least duration + buffer
            result = await asyncio.wait_for(
                api.check_win(trade_id),
                timeout=duration + 20.0,
            )
            assert isinstance(result, dict)
            assert "result" in result
            assert result["result"] in ["win", "loss", "draw"]
            print(f"Trade result: {result}")
        except asyncio.TimeoutError:
            print(f"Timeout occurred for trade_id: {trade_id}")
            pytest.fail(f"Timed out waiting for trade result for trade_id: {trade_id}")
        except Exception as e:
            print(f"Error during check_win: {e}")
            pytest.fail(f"Error during check_win: {e}")

    except Exception as e:
        print(f"Trade failed: {e}")
        pytest.fail(f"Trade failed: {e}")


@pytest.mark.asyncio
async def test_buy_without_waiting(api):
    """Test buying an asset without waiting for the result (faster test)."""
    if not api.is_demo():
        pytest.skip("Skipping trade test on real account to avoid losing money")

    asset = "EURUSD_otc"
    amount = 1.0
    duration = 5

    # Check if we can get payout for this asset to ensure it's valid
    try:
        payout = await api.payout(asset)
        if not payout:
            pytest.skip(f"Asset {asset} not available or no payout")
    except Exception:
        pytest.skip(f"Could not check payout for {asset}")

    print(f"Buying {asset} without waiting for result...")
    try:
        # Buy with check_win=False to not wait for result
        trade_id, trade_info = await api.buy(asset, amount, duration, check_win=False)
        assert trade_id
        assert isinstance(trade_info, dict)
        print(f"Trade placed: {trade_id}, Info: {trade_info}")

    except Exception as e:
        pytest.fail(f"Trade placement failed: {e}")


@pytest.mark.asyncio
async def test_get_candles(api):
    """Test retrieving historical candle data."""
    asset = "EURUSD_otc"
    period = 60  # 1-minute candles

    print(f"Fetching candles for {asset} with period {period}...")
    try:
        # Some assets might not be available, so we check payout first
        payout = await api.payout(asset)
        if not payout:
            pytest.skip(f"Asset {asset} not available")

        # api.candles() uses HistoricalDataApiModule
        candles = await asyncio.wait_for(api.candles(asset, period), timeout=20.0)
        assert isinstance(candles, list)
        assert len(candles) > 0
        print(f"Received {len(candles)} candles.")
        for candle in candles[:2]:  # Print first 2 for verification
            print(f"Candle: {candle}")
            assert "time" in candle or "timestamp" in candle
            assert "open" in candle
            assert "close" in candle
    except asyncio.TimeoutError:
        pytest.fail("Timed out waiting for candles")
    except Exception as e:
        pytest.fail(f"Failed to get candles: {e}")


@pytest.mark.asyncio
async def test_history(api):
    """Test retrieving historical candle data using the history method."""
    asset = "EURUSD_otc"
    period = 60

    print(f"Fetching history for {asset} with period {period}...")
    try:
        payout = await api.payout(asset)
        if not payout:
            pytest.skip(f"Asset {asset} not available")

        # api.history() is a wrapper for candles()
        history = await asyncio.wait_for(api.history(asset, period), timeout=20.0)
        assert isinstance(history, list)
        assert len(history) > 0
        print(f"Received {len(history)} candles from history.")
    except asyncio.TimeoutError:
        pytest.fail("Timed out waiting for history")
    except Exception as e:
        pytest.fail(f"Failed to get history: {e}")


if __name__ == "__main__":
    sys.exit(pytest.main(["-v", __file__]))
