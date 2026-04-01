"""
Example script demonstrating the new connection control and raw handler features.
"""

import asyncio
import os

import pytest

from BinaryOptionsToolsV2 import PocketOption, PocketOptionAsync
from BinaryOptionsToolsV2.validator import Validator


@pytest.mark.asyncio
async def test_async_connection_control():
    """Test async connection control methods."""
    print("=== Testing Async Connection Control ===")

    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        pytest.skip("POCKET_OPTION_SSID not set")

    # Use context manager or manual
    async with PocketOptionAsync(ssid) as client:
        # Test disconnect and connect
        print("Disconnecting...")
        await client.disconnect()
        print("✓ Disconnected")

        await asyncio.sleep(0.5)

        print("Reconnecting...")
        await client.connect()
        print("✓ Connected")

        # Test reconnect
        print("Testing reconnect...")
        await client.reconnect()
        print("✓ Reconnected")


@pytest.mark.asyncio
async def test_async_raw_handler(api):
    """Test async raw handler functionality."""
    pytest.skip("Raw handler subscription test - stream may not receive matching messages")

@pytest.mark.asyncio
async def test_async_unsubscribe(api):
    """Test unsubscribing from asset streams."""
    print("\n=== Testing Async Unsubscribe ===")

    # Subscribe to an asset
    print("Subscribing to EURUSD_otc...")
    subscription = await api.subscribe_symbol("EURUSD_otc")

    # Get a few updates
    count = 0
    async for candle in subscription:
        print(f"✓ Candle {count + 1}: {candle}")
        count += 1
        if count >= 3:
            break

    # Unsubscribe
    print("Unsubscribing from EURUSD_otc...")
    await api.unsubscribe("EURUSD_otc")
    print("✓ Unsubscribed")


def test_sync_connection_control():
    """Test sync connection control methods."""
    print("\n=== Testing Sync Connection Control ===")

    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        pytest.skip("POCKET_OPTION_SSID not set")

    # Use custom config with reduced timeout
    config = {"connection_initialization_timeout_secs": 30}
    client = PocketOption(ssid, config=config)

    # Test disconnect and connect
    print("Disconnecting...")
    client.disconnect()
    print("✓ Disconnected")

    import time

    time.sleep(0.5)

    print("Reconnecting...")
    client.connect()
    print("✓ Connected")

    # Test reconnect
    print("Testing reconnect...")
    client.reconnect()
    print("✓ Reconnected")


def test_sync_raw_handler(api_sync):
    """Test sync raw handler functionality."""
    pytest.skip("Raw handler subscription test - stream may not receive matching messages")
def test_sync_unsubscribe(api_sync):
    """Test unsubscribing from asset streams (sync)."""
    print("\n=== Testing Sync Unsubscribe ===\n")

    # Subscribe to an asset
    print("Subscribing to EURUSD_otc...")
    subscription = api_sync.subscribe_symbol("EURUSD_otc")

    # Get a few updates
    count = 0
    for candle in subscription:
        print(f"✓ Candle {count + 1}: {candle}")
        count += 1
        if count >= 3:
            break

    # Unsubscribe
    print("Unsubscribing from EURUSD_otc...")
    api_sync.unsubscribe("EURUSD_otc")
    print("✓ Unsubscribed")


async def main():
    """Run all tests."""
    print("=" * 60)
    print("Testing New Features")
    print("=" * 60)

    # Choose which tests to run
    # Comment out the ones you don't want to test

    # Async tests
    # await test_async_connection_control()
    # await test_async_raw_handler()
    # await test_async_unsubscribe()

    # Sync tests
    # test_sync_connection_control()
    # test_sync_raw_handler()
    # test_sync_unsubscribe()

    print("\n" + "=" * 60)
    print("All tests completed!")
    print("=" * 60)


if __name__ == "__main__":
    if not os.getenv("POCKET_OPTION_SSID"):
        print("NOTE: Set POCKET_OPTION_SSID environment variable before running!")
        print()

    # Uncomment to run tests
    # asyncio.run(main())
