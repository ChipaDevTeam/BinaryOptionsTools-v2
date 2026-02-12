"""
Example script demonstrating the new connection control and raw handler features.
"""

import asyncio
import os
import pytest

from BinaryOptionsToolsV2 import (
    PocketOption,
    PocketOptionAsync,
)
from BinaryOptionsToolsV2.validator import Validator


@pytest.mark.asyncio
async def test_async_connection_control():
    """Test async connection control methods."""
    print("=== Testing Async Connection Control ===")

    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        print("Error: POCKET_OPTION_SSID environment variable not set")
        return

    # Use context manager or manual
    async with PocketOptionAsync(ssid) as client:
        # Test disconnect and connect
        print("Disconnecting...")
        await client.disconnect()
        print("✓ Disconnected")

        await asyncio.sleep(2)

        print("Reconnecting...")
        await client.connect()
        print("✓ Connected")

        # Test reconnect
        print("Testing reconnect...")
        await client.reconnect()
        print("✓ Reconnected")


@pytest.mark.asyncio
async def test_async_raw_handler():
    """Test async raw handler functionality."""
    print("\n=== Testing Async Raw Handler ===")

    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        print("Error: POCKET_OPTION_SSID environment variable not set")
        return

    async with PocketOptionAsync(ssid) as client:
        # Create a validator that matches EURUSD_otc updates
        # These are very frequent and reliable for testing
        validator = Validator.contains("EURUSD_otc")

        # Create raw handler
        print("Creating raw handler...")
        handler = await client.create_raw_handler(validator)
        print(f"✓ Handler created with ID: {handler.id()}")

        # Wait for any EURUSD_otc message
        print("Waiting for EURUSD_otc update...")
        try:
            response = await asyncio.wait_for(handler.wait_next(), timeout=30.0)
            print(f"✓ Received response: {response[:200]}...")
            assert "EURUSD_otc" in response
        except asyncio.TimeoutError:
            print("✗ Timeout waiting for EURUSD_otc update")
            raise

        # Now try subscription
        print("Subscribing to stream...")
        stream = await handler.subscribe()

        # Read a few messages from stream
        print("Waiting for messages from stream...")
        for i in range(3):
            try:
                message = await asyncio.wait_for(stream.__anext__(), timeout=30.0)
                print(f"✓ Stream message {i + 1}: {message[:100]}...")
                assert "EURUSD_otc" in message
            except asyncio.TimeoutError:
                print(f"✗ Timeout waiting for stream message {i + 1}")
                raise

    print("✓ Raw handler test completed")


@pytest.mark.asyncio
async def test_async_unsubscribe():
    """Test unsubscribing from asset streams."""
    print("\n=== Testing Async Unsubscribe ===")

    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        print("Error: POCKET_OPTION_SSID environment variable not set")
        return

    async with PocketOptionAsync(ssid) as client:
        # Subscribe to an asset
        print("Subscribing to EURUSD_otc...")
        subscription = await client.subscribe_symbol("EURUSD_otc")

        # Get a few updates
        count = 0
        async for candle in subscription:
            print(f"✓ Candle {count + 1}: {candle}")
            count += 1
            if count >= 3:
                break

        # Unsubscribe
        print("Unsubscribing from EURUSD_otc...")
        await client.unsubscribe("EURUSD_otc")
        print("✓ Unsubscribed")


def test_sync_connection_control():
    """Test sync connection control methods."""
    print("\n=== Testing Sync Connection Control ===")

    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        print("Error: POCKET_OPTION_SSID environment variable not set")
        return

    # Use custom config with increased timeout
    config = {"connection_initialization_timeout_secs": 20}
    client = PocketOption(ssid, config=config)

    # Test disconnect and connect
    print("Disconnecting...")
    client.disconnect()
    print("✓ Disconnected")

    import time

    time.sleep(2)

    print("Reconnecting...")
    client.connect()
    print("✓ Connected")

    # Test reconnect
    print("Testing reconnect...")
    client.reconnect()
    print("✓ Reconnected")


def test_sync_raw_handler():
    """Test sync raw handler functionality."""
    print("\n=== Testing Sync Raw Handler ===")

    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        print("Error: POCKET_OPTION_SSID environment variable not set")
        return

    # Use custom config with increased timeout
    config = {"connection_initialization_timeout_secs": 20}
    with PocketOption(ssid, config=config) as client:
        # Use EURUSD_otc validator as it's reliable
        validator = Validator.contains("EURUSD_otc")

        # Create raw handler
        print("Creating raw handler...")
        handler = client.create_raw_handler(validator)
        print(f"✓ Handler created with ID: {handler.id()}")

        # Wait for any EURUSD_otc message
        print("Waiting for EURUSD_otc update...")
        try:
            response = handler.wait_next()
            print(f"✓ Received response: {response[:100]}...")
            assert "EURUSD_otc" in response
        except Exception as e:
            print(f"✗ Failed to receive message: {e}")
            raise

        # Test subscription
        print("Subscribing to stream...")
        stream = handler.subscribe()

        # Read a few messages from stream
        print("Waiting for messages from stream...")
        for i in range(3):
            message = next(stream)
            print(f"✓ Stream message {i + 1}: {message[:100]}...")
            assert "EURUSD_otc" in message

    print("✓ Sync raw handler test completed")


def test_sync_unsubscribe():
    """Test unsubscribing from asset streams (sync)."""
    print("\n=== Testing Sync Unsubscribe ===")

    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        print("Error: POCKET_OPTION_SSID environment variable not set")
        return

    # Use custom config with increased timeout
    config = {"connection_initialization_timeout_secs": 20}
    client = PocketOption(ssid, config=config)

    # Subscribe to an asset
    print("Subscribing to EURUSD_otc...")
    subscription = client.subscribe_symbol("EURUSD_otc")

    # Get a few updates
    count = 0
    for candle in subscription:
        print(f"✓ Candle {count + 1}: {candle}")
        count += 1
        if count >= 3:
            break

    # Unsubscribe
    print("Unsubscribing from EURUSD_otc...")
    client.unsubscribe("EURUSD_otc")
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
