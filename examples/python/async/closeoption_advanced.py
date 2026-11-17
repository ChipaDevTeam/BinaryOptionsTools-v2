#!/usr/bin/env python3
"""
CloseOption Advanced Async Example

Demonstrates advanced features:
- Asset management and payout checking
- Trade history and deal tracking
- Multiple subscription patterns
- Error handling
- Connection management

Usage:
    python closeoption_advanced.py
"""

import asyncio
import os
import sys
from BinaryOptionsToolsV2 import CloseOptionAsync


async def demo_asset_operations(client: CloseOptionAsync):
    """Demonstrate asset-related operations."""
    print("\n=== Asset Operations ===")

    # Get active assets
    assets = await client.active_assets()
    print(f"Total active assets: {len(assets)}")

    if assets:
        # Show first few assets with details
        for asset in assets[:5]:
            symbol = asset.get("symbol", "unknown")
            bid = asset.get("bid", 0)
            ask = asset.get("ask", 0)
            spread = ask - bid if ask > bid else 0

            # Get payout for this asset
            try:
                payout = await client.payout(symbol)
                print(
                    f"  {symbol}: bid={bid:.5f}, ask={ask:.5f}, spread={spread:.5f}, payout={payout * 100:.1f}%"
                )
            except Exception as e:
                print(
                    f"  {symbol}: bid={bid:.5f}, ask={ask:.5f}, payout check failed: {e}"
                )

    # Find specific asset
    eur_usd = None
    for asset in assets:
        if (
            "EUR" in asset.get("symbol", "").upper()
            and "USD" in asset.get("symbol", "").upper()
        ):
            eur_usd = asset
            break

    if eur_usd:
        print(f"\nFound EURUSD: {eur_usd}")
        return eur_usd["symbol"]
    return None


async def demo_candle_operations(client: CloseOptionAsync):
    """Demonstrate candle data retrieval."""
    print("\n=== Candle Operations ===")

    asset = "EURUSD"

    # Get historical candles with different periods
    periods = [
        (60, "1 minute"),
        (300, "5 minutes"),
        (900, "15 minutes"),
        (1800, "30 minutes"),
    ]

    for period, label in periods:
        try:
            candles = await client.get_candles(asset, period, 5)
            print(f"  {label}: {len(candles)} candles retrieved")
            if candles:
                print(
                    f"    Latest: timestamp={candles[-1].get('timestamp', 'N/A')}, value={candles[-1].get('value', 'N/A')}"
                )
        except Exception as e:
            print(f"  {label}: Error - {e}")


async def demo_subscription(client: CloseOptionAsync):
    """Demonstrate subscription patterns."""
    print("\n=== Subscription Demo ===")

    # Test raw subscription
    print("Creating raw subscription...")
    try:
        sub = await client.subscribe_raw()
        print(f"  Raw subscription created: {sub}")
    except Exception as e:
        print(f"  Raw subscription error: {e}")

    # Test symbol subscription
    print("Creating symbol subscription for EURUSD...")
    try:
        sub = await client.subscribe_symbol("EURUSD")
        print(f"  Symbol subscription created: {sub}")
    except Exception as e:
        print(f"  Symbol subscription error: {e}")


async def demo_account_info(client: CloseOptionAsync):
    """Demonstrate account information retrieval."""
    print("\n=== Account Info ===")

    # Balance
    try:
        balance = await client.balance()
        print(f"  Current balance: {balance}")
    except Exception as e:
        print(f"  Balance error: {e}")

    # Server time
    try:
        server_time = await client.get_server_time()
        print(f"  Server time: {server_time}")
    except Exception as e:
        print(f"  Server time error: {e}")

    # History
    try:
        history = await client.history(limit=10)
        print(f"  Trade history: {len(history)} records")
    except Exception as e:
        print(f"  History error: {e}")

    # Opened deals
    try:
        opened = await client.opened_deals()
        print(f"  Opened deals: {len(opened)}")
    except Exception as e:
        print(f"  Opened deals error: {e}")

    # Closed deals
    try:
        closed = await client.closed_deals()
        print(f"  Closed deals: {len(closed)}")
    except Exception as e:
        print(f"  Closed deals error: {e}")


async def demo_connection_management(client: CloseOptionAsync):
    """Demonstrate connection management."""
    print("\n=== Connection Management ===")

    # Get server time before reconnect
    try:
        time_before = await client.get_server_time()
        print(f"  Time before reconnect: {time_before}")
    except Exception as e:
        print(f"  Pre-reconnect error: {e}")

    # Test reconnect (may fail if not connected)
    try:
        await client.reconnect()
        print("  Reconnect successful")
    except Exception as e:
        print(f"  Reconnect error (expected if not connected): {type(e).__name__}")


async def main():
    """Main demonstration function."""
    # Get SSID from environment
    ssid = os.environ.get("CLOSEOPTION_SSID")

    if not ssid:
        print("No CLOSEOPTION_SSID environment variable set.")
        print("Running in demo mode (API structure verification only)...")
        print("Format: token|sid|demo|public_code|hidden_code")
        print()

        # Create client without connection to verify API
        client = CloseOptionAsync("demo_token|demo_sid|true|demo_public|demo_hidden")

        # Verify all methods exist
        methods = [
            "buy",
            "sell",
            "check_win",
            "balance",
            "get_candles",
            "active_assets",
            "payout",
            "history",
            "opened_deals",
            "closed_deals",
            "get_server_time",
            "send_raw",
            "subscribe_symbol",
            "subscribe_raw",
            "get_candles_live",
            "raw_handler",
            "reconnect",
            "shutdown",
        ]

        print("=== API Structure Verification ===")
        for method in methods:
            has_method = hasattr(client, method)
            status = "✓" if has_method else "✗"
            print(f"  {status} {method}")

        if all(hasattr(client, m) for m in methods):
            print("\nAll API methods present!")
        else:
            print("\nSome methods missing!")
            return 1

        return 0

    # Real connection demo
    print("Connecting to CloseOption...")

    try:
        async with CloseOptionAsync(ssid) as client:
            print("Connected successfully!\n")

            await demo_asset_operations(client)
            await demo_candle_operations(client)
            await demo_subscription(client)
            await demo_account_info(client)
            await demo_connection_management(client)

            print("\n=== Demo Complete ===")
            return 0

    except Exception as e:
        print(f"Error: {e}")
        return 1


if __name__ == "__main__":
    exit_code = asyncio.run(main())
    sys.exit(exit_code)
