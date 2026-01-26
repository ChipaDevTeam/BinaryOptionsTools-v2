import asyncio
import time

from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync


# Main part of the code
async def main(ssid: str):
    # The api automatically detects if the 'ssid' is for real or demo account
    api = PocketOptionAsync(ssid)

    time.sleep(5)

    (buy_id, buy) = await api.buy(
        asset="EURUSD_otc", amount=1.0, time=60, check_win=False
    )
    print(f"Buy trade id: {buy_id}\nBuy trade data: {buy}")
    (sell_id, sell) = await api.sell(
        asset="EURUSD_otc", amount=1.0, time=60, check_win=False
    )
    print(f"Sell trade id: {sell_id}\nSell trade data: {sell}")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    asyncio.run(main(ssid))
