import asyncio
import os
import json
from BinaryOptionsToolsV2 import (
    RawPocketOption,
    PyBot,
    PyStrategy,
    PyContext,
    start_tracing,
)
from dotenv import load_dotenv

load_dotenv()


class MyRSIStrategy(PyStrategy):
    def __init__(self, rsi_period=14):
        super().__init__()
        self.rsi_period = rsi_period
        self.prices = {}

    def on_start(self, ctx):
        print("Strategy started!")

    def on_candle(self, ctx, asset, candle_json):
        candle = json.loads(candle_json)
        print(f"New candle for {asset}: {candle['close']}")

        # Simple logic: if price ends in .5, buy (just for demo)
        if str(candle["close"]).endswith("5"):
            print(f"Signal detected on {asset}! Buying...")
            # ctx.buy returns a future
            asyncio.create_task(self.execute_trade(ctx, asset))

    async def execute_trade(self, ctx, asset):
        try:
            result = await ctx.buy(asset, 1.0, 60)
            print(f"Trade executed: {result}")
        except Exception as e:
            print(f"Trade failed: {e}")


async def main():
    start_tracing("info")

    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        print("Please set POCKET_OPTION_SSID in .env file")
        return

    print("Connecting to PocketOption...")
    client = await RawPocketOption.create(ssid)

    strategy = MyRSIStrategy()
    bot = PyBot(client, strategy)

    # Add assets to monitor (60s candles)
    bot.add_asset("EURUSD_otc", 60)

    print("Running bot... Press Ctrl+C to stop.")
    await bot.run()


if __name__ == "__main__":
    asyncio.run(main())
