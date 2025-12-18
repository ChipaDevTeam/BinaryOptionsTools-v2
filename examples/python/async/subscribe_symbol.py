import asyncio

from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync


# Main part of the code
async def main(ssid: str):
    # The api automatically detects if the 'ssid' is for real or demo account
    api = PocketOptionAsync(ssid)
    stream = await api.subscribe_symbol("EURUSD_otc")

    # This should run forever so you will need to force close the program
    async for candle in stream:
        print(f"Candle: {candle}")  # Each candle is in format of a dictionary


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    asyncio.run(main(ssid))
