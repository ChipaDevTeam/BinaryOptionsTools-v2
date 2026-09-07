#!/usr/bin/env python3
"""
CloseOption Basic Sync Example

This example demonstrates how to use the CloseOption synchronous client
to connect to CloseOption and perform basic operations.

Usage:
    python closeoption_basic.py

Note: You need a valid SSID in format "token|sid|demo|public_code|hidden_code"
"""

import os
from BinaryOptionsToolsV2 import CloseOption


def main():
    # Get SSID from environment variable or use a placeholder
    # Format: token|sid|demo|public_code|hidden_code
    ssid = os.environ.get(
        "CLOSEOPTION_SSID", "your_token|your_sid|true|your_public_code|your_hidden_code"
    )

    if "your_token" in ssid:
        print("Please set CLOSEOPTION_SSID environment variable with your credentials")
        print("Format: token|sid|demo|public_code|hidden_code")
        return

    print("Connecting to CloseOption...")

    with CloseOption(ssid) as client:
        print("Connected!")

        # Get active assets
        print("\nFetching active assets...")
        assets = client.active_assets()
        print(f"Found {len(assets)} assets")
        for asset in assets[:5]:  # Show first 5
            print(f"  - {asset['symbol']}: bid={asset['bid']}, ask={asset['ask']}")

        # Get balance
        print("\nFetching balance...")
        balance = client.balance()
        print(f"Balance: {balance}")

        # Get server time
        print("\nFetching server time...")
        server_time = client.get_server_time()
        print(f"Server time: {server_time}")

        # Get candles for EURUSD
        print("\nFetching candles for EURUSD (1 minute)...")
        candles = client.get_candles("EURUSD", 60, 10)
        print(f"Got {len(candles)} candles")
        for candle in candles[:3]:
            print(f"  - {candle}")

        # Subscribe to price updates
        print("\nSubscribing to EURUSD price updates...")
        subscription = client.subscribe_symbol("EURUSD")
        print("Subscription created. Receiving 5 updates...")

        count = 0
        for update in subscription:
            print(f"  Price update: {update}")
            count += 1
            if count >= 5:
                break

        print("\nDone!")


if __name__ == "__main__":
    main()
