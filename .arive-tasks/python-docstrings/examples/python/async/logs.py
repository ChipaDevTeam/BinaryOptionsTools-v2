import asyncio

from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync
from BinaryOptionsToolsV2.tracing import start_logs


# Main part of the code
async def main(ssid: str):
    # Start logs, it works perfectly on async code
    start_logs(
        path=".", level="DEBUG", terminal=True
    )  # If false then the logs will only be written to the log files
    # The api automatically detects if the 'ssid' is for real or demo account
    api = PocketOptionAsync(ssid)
    (buy_id, _) = await api.buy(
        asset="EURUSD_otc", amount=1.0, time=300, check_win=False
    )
    (sell_id, _) = await api.sell(
        asset="EURUSD_otc", amount=1.0, time=300, check_win=False
    )
    print(buy_id, sell_id)
    # This is the same as setting checkw_win to true on the api.buy and api.sell functions
    buy_data = await api.check_win(buy_id)
    sell_data = await api.check_win(sell_id)
    print(f"Buy trade result: {buy_data['result']}\nBuy trade data: {buy_data}")
    print(f"Sell trade result: {sell_data['result']}\nSell trade data: {sell_data}")


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    asyncio.run(main(ssid))
