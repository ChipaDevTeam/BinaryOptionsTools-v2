from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync
import asyncio
import os

# Function to read assets from a file
def read_assets(filename):
    with open(filename, 'r') as f:
        assets = [line.strip() for line in f if line.strip()]
    return assets

# Function to write assets to a file
def write_asset(filename, asset):
    with open(filename, 'a') as f:
        f.write(asset + '\n')

async def main(ssid: str):
    api = PocketOptionAsync(ssid)

    # Define file paths
    assets_file = 'tests/assets.txt'
    tested_assets_file = 'assets.tested.txt'
    not_working_assets_file = 'not-working-assets.txt'

    # Clear previous test results if files exist
    if os.path.exists(tested_assets_file):
        os.remove(tested_assets_file)
    if os.path.exists(not_working_assets_file):
        os.remove(not_working_assets_file)

    # Read assets from assets.txt
    assets_to_test = read_assets(assets_file)

    for asset in assets_to_test:
        print(f"Attempting to trade on asset: {asset}")
        try:
            # Attempt a buy trade with a small amount and short time
            # Setting check_win to False for initial trade attempt to quickly determine if it's a valid asset
            (trade_id, _) = await api.buy(asset=asset, amount=1.0, time=15, check_win=False)
            if trade_id:
                print(f"Trade successful for {asset}. Trade ID: {trade_id}")
                write_asset(tested_assets_file, asset)
                # Optionally, you might want to cancel the trade if you only want to test validity
                # await api.cancel_trade(trade_id)
            else:
                print(f"Trade failed for {asset}. No trade ID returned.")
                write_asset(not_working_assets_file, asset)
        except Exception as e:
            print(f"An error occurred while trading {asset}: {e}")
            write_asset(not_working_assets_file, asset)
        await asyncio.sleep(1) # Add a small delay to avoid overwhelming the API

if __name__ == '__main__':
    ssid = input('Please enter your ssid: ')
    asyncio.run(main(ssid))