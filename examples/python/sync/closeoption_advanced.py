#!/usr/bin/env python3
"""
CloseOption Advanced Sync Example

Demonstrates advanced features with synchronous client:
- Asset management and payout checking
- Trade history and deal tracking
- Multiple subscription patterns
- Error handling
- Connection management

Usage:
    python closeoption_advanced.py
"""

import os
import sys
from BinaryOptionsToolsV2 import CloseOption


def demo_asset_operations(client):
    """Demonstrate asset-related operations."""
    print("\n=== Asset Operations ===")

    assets = client.active_assets()
    print(f"Total active assets: {len(assets)}")

    if assets:
        for asset in assets[:5]:
            symbol = asset.get("symbol", "unknown")
            bid = asset.get("bid", 0)
            ask = asset.get("ask", 0)
            spread = ask - bid if ask > bid else 0

            try:
                payout = client.payout(symbol)
                print(
                    f"  {symbol}: bid={bid:.5f}, ask={ask:.5f}, spread={spread:.5f}, payout={payout * 100:.1f}%"
                )
            except Exception as e:
                print(f"  {symbol}: bid={bid:.5f}, ask={ask:.5f}, payout error: {e}")

    return assets[0]["symbol"] if assets else None


def demo_candle_operations(client, asset="EURUSD"):
    """Demonstrate candle data retrieval."""
    print("\n=== Candle Operations ===")

    periods = [
        (60, "1 minute"),
        (300, "5 minutes"),
        (900, "15 minutes"),
        (1800, "30 minutes"),
    ]

    for period, label in periods:
        try:
            candles = client.get_candles(asset, period, 5)
            print(f"  {label}: {len(candles)} candles retrieved")
            if candles:
                print(
                    f"    Latest: timestamp={candles[-1].get('timestamp', 'N/A')}, value={candles[-1].get('value', 'N/A')}"
                )
        except Exception as e:
            print(f"  {label}: Error - {e}")


def demo_subscription(client):
    """Demonstrate subscription patterns."""
    print("\n=== Subscription Demo ===")

    try:
        sub = client.subscribe_raw()
        print(f"  Raw subscription created: {sub}")
    except Exception as e:
        print(f"  Raw subscription error: {e}")

    try:
        sub = client.subscribe_symbol("EURUSD")
        print(f"  Symbol subscription created: {sub}")
    except Exception as e:
        print(f"  Symbol subscription error: {e}")


def demo_account_info(client):
    """Demonstrate account information retrieval."""
    print("\n=== Account Info ===")

    try:
        balance = client.balance()
        print(f"  Current balance: {balance}")
    except Exception as e:
        print(f"  Balance error: {e}")

    try:
        server_time = client.get_server_time()
        print(f"  Server time: {server_time}")
    except Exception as e:
        print(f"  Server time error: {e}")

    try:
        history = client.history(limit=10)
        print(f"  Trade history: {len(history)} records")
    except Exception as e:
        print(f"  History error: {e}")

    try:
        opened = client.opened_deals()
        print(f"  Opened deals: {len(opened)}")
    except Exception as e:
        print(f"  Opened deals error: {e}")

    try:
        closed = client.closed_deals()
        print(f"  Closed deals: {len(closed)}")
    except Exception as e:
        print(f"  Closed deals error: {e}")


def demo_connection_management(client):
    """Demonstrate connection management."""
    print("\n=== Connection Management ===")

    try:
        time_before = client.get_server_time()
        print(f"  Time before reconnect: {time_before}")
    except Exception as e:
        print(f"  Pre-reconnect error: {e}")

    try:
        client.reconnect()
        print("  Reconnect successful")
    except Exception as e:
        print(f"  Reconnect error (expected if not connected): {type(e).__name__}")


def main():
    """Main demonstration function."""
    ssid = os.environ.get("CLOSEOPTION_SSID")

    if not ssid:
        print("No CLOSEOPTION_SSID environment variable set.")
        print("Running in demo mode (API structure verification only)...")
        print("Format: token|sid|demo|public_code|hidden_code")
        print()

        client = CloseOption(
            "demo_token|demo_sid|true|demo_public|demo_hidden", connect_on_init=False
        )

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

    print("Connecting to CloseOption...")

    try:
        with CloseOption(ssid) as client:
            print("Connected successfully!\n")

            demo_asset_operations(client)
            demo_candle_operations(client)
            demo_subscription(client)
            demo_account_info(client)
            demo_connection_management(client)

            print("\n=== Demo Complete ===")
            return 0

    except Exception as e:
        print(f"Error: {e}")
        return 1


if __name__ == "__main__":
    exit_code = main()
    sys.exit(exit_code)
