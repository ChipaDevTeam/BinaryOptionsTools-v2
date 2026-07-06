import asyncio
from datetime import timedelta

from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync
from BinaryOptionsToolsV2.tracing import start_logs


# Main part of the code
async def main(ssid: str):
    # Use context manager to ensure connection is authenticated and assets are loaded
    start_logs(".", "INFO")
    async with PocketOptionAsync(ssid) as api:
        stream = await api.subscribe_symbol_timed(
            "EURUSD_otc", timedelta(seconds=5)
        )  # Returns a candle obtained from combining candles that are inside a specific time range

        # This should run forever so you will need to force close the program
        async for candle in stream:
            print(f"Candle: {candle}")  # Each candle is in format of a dictionary

if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    asyncio.run(main(ssid))
