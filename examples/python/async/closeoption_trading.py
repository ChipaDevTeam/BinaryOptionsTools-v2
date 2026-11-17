#!/usr/bin/env python3
"""
CloseOption Trading Example

Demonstrates trading operations:
- Placing buy (CALL) orders
- Placing sell (PUT) orders
- Checking trade results
- Error handling for trading operations

Usage:
    python closeoption_trading.py
"""

import asyncio
import os
import sys
from BinaryOptionsToolsV2 import CloseOptionAsync


async def place_trade(
    client: CloseOptionAsync, asset: str, amount: float, duration: int, direction: str
):
    """Place a trade (buy/call or sell/put)."""
    print(f"\nPlacing {direction} order:")
    print(f"  Asset: {asset}")
    print(f"  Amount: ${amount}")
    print(f"  Duration: {duration} seconds")

    try:
        if direction.lower() in ("buy", "call"):
            result = await client.buy(asset, amount, duration)
        else:
            result = await client.sell(asset, amount, duration)

        print("  Order placed successfully!")
        print(f"  Result: {result}")
        return result

    except Exception as e:
        print(f"  Error placing order: {e}")
        return None


async def check_trade(client: CloseOptionAsync, order_id: str):
    """Check the result of a trade."""
    print(f"\nChecking trade result for order: {order_id}")

    try:
        result = await client.check_win(order_id)
        print(f"  Result: {result}")
        return result
    except Exception as e:
        print(f"  Error checking trade: {e}")
        return None


async def main():
    """Main trading demonstration."""
    ssid = os.environ.get("CLOSEOPTION_SSID")

    if not ssid:
        print("No CLOSEOPTION_SSID environment variable set.")
        print("Running in demo mode (API structure verification only)...")
        print()

        client = CloseOptionAsync("demo_token|demo_sid|true|demo_public|demo_hidden")

        # Verify trading methods exist
        trading_methods = ["buy", "sell", "check_win"]
        print("=== Trading API Verification ===")
        for method in trading_methods:
            has_method = hasattr(client, method)
            status = "✓" if has_method else "✗"
            print(f"  {status} {method}")

        if all(hasattr(client, m) for m in trading_methods):
            print("\nAll trading methods present!")
        else:
            print("\nSome trading methods missing!")
            return 1

        return 0

    print("Connecting to CloseOption...")

    try:
        async with CloseOptionAsync(ssid) as client:
            print("Connected successfully!\n")

            # Get assets first
            print("Fetching available assets...")
            assets = await client.active_assets()
            print(f"Found {len(assets)} assets")

            if not assets:
                print("No assets available. Exiting.")
                return 0

            # Demo trading with first asset (read-only demonstration)
            asset = assets[0]["symbol"]
            print(f"\nUsing asset: {asset}")

            # Get payout for this asset
            payout = await client.payout(asset)
            print(f"Payout for {asset}: {payout * 100:.1f}%")

            # Get current balance
            balance = await client.balance()
            print(f"Current balance: ${balance}")

            # Note: Actual trading requires valid credentials and funds
            # Uncomment below to place real trades:
            #
            # print("\n=== Placing Demo Trades ===")
            # buy_result = await place_trade(client, asset, 10.0, 60, 'buy')
            # sell_result = await place_trade(client, asset, 10.0, 60, 'sell')
            #
            # if buy_result and 'orderId' in buy_result:
            #     await check_trade(client, buy_result['orderId'])

            print("\n=== Trading Demo Complete ===")
            print(
                "To place real trades, set CLOSEOPTION_SSID and uncomment trade code above."
            )
            return 0

    except Exception as e:
        print(f"Error: {e}")
        return 1


if __name__ == "__main__":
    exit_code = asyncio.run(main())
    sys.exit(exit_code)
