"""
Example script demonstrating the new connection control and raw handler features.
"""

import asyncio
import json
from BinaryOptionsToolsV2.pocketoption import (
    PocketOptionAsync,
    PocketOption,
    RawHandler,
)
from BinaryOptionsToolsV2.validator import Validator


async def test_async_connection_control():
    """Test async connection control methods."""
    print("=== Testing Async Connection Control ===")

    ssid = "your_session_id_here"
    client = PocketOptionAsync(ssid)

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


async def test_async_raw_handler():
    """Test async raw handler functionality."""
    print("\n=== Testing Async Raw Handler ===")

    ssid = "your_session_id_here"
    client = PocketOptionAsync(ssid)

    # Create a validator for balance messages
    validator = Validator.contains('"balance"')

    # Create raw handler
    print("Creating raw handler...")
    handler = await client.create_raw_handler(validator)
    print(f"✓ Handler created with ID: {handler.id()}")

    # Send a message and wait for response
    print("Sending message and waiting for response...")
    response = await handler.send_and_wait('42["getBalance"]')
    print(f"✓ Received response: {response[:100]}...")

    # Subscribe to messages
    print("Subscribing to stream...")
    stream = await handler.subscribe()

    # Read a few messages
    count = 0
    async for message in stream:
        print(f"✓ Message {count + 1}: {message[:100]}...")
        count += 1
        if count >= 3:
            break

    print("✓ Raw handler test completed")


async def test_async_unsubscribe():
    """Test unsubscribing from asset streams."""
    print("\n=== Testing Async Unsubscribe ===")

    ssid = "your_session_id_here"
    client = PocketOptionAsync(ssid)

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

    ssid = "your_session_id_here"
    client = PocketOption(ssid)

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

    ssid = "your_session_id_here"
    client = PocketOption(ssid)

    # Create a validator
    validator = Validator.contains('"payout"')

    # Create raw handler
    print("Creating raw handler...")
    handler = client.create_raw_handler(validator)
    print(f"✓ Handler created with ID: {handler.id()}")

    # Send a message and wait for response
    print("Sending message and waiting for response...")
    response = handler.send_and_wait('42["getAssets"]')
    print(f"✓ Received response: {response[:100]}...")

    # Subscribe to messages
    print("Subscribing to stream...")
    stream = handler.subscribe()

    # Read a few messages
    count = 0
    for message in stream:
        print(f"✓ Message {count + 1}: {message[:100]}...")
        count += 1
        if count >= 3:
            break

    print("✓ Raw handler test completed")


def test_sync_unsubscribe():
    """Test unsubscribing from asset streams (sync)."""
    print("\n=== Testing Sync Unsubscribe ===")

    ssid = "your_session_id_here"
    client = PocketOption(ssid)

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
    # Replace with your actual session ID
    print("NOTE: Replace 'your_session_id_here' with your actual SSID before running!")
    print()

    # Uncomment to run tests
    # asyncio.run(main())
