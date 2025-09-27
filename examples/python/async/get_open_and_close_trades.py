from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync

import asyncio


# Main part of the code
async def main(ssid: str):
    # The api automatically detects if the 'ssid' is for real or demo account
    api = PocketOptionAsync(ssid)
    _ = await api.buy(asset="EURUSD_otc", amount=1.0, time=60, check_win=False)
    _ = await api.sell(asset="EURUSD_otc", amount=1.0, time=60, check_win=False)
    # This is the same as setting checkw_win to true on the api.buy and api.sell functions
    opened_deals = await api.opened_deals()
    print(
        f"Opened deals: {opened_deals}\nNumber of opened deals: {len(opened_deals)} (should be at least 2)"
    )
    await asyncio.sleep(62)  # Wait for the trades to complete
    closed_deals = await api.closed_deals()
    print(
        f"Closed deals: {closed_deals}\nNumber of closed deals: {len(closed_deals)} (should be at least 2)"
    )


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    asyncio.run(main(ssid))
