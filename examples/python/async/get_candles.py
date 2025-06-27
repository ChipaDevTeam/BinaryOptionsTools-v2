from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync

import pandas as pd
import asyncio
# Main part of the code
async def main(ssid: str):
    # The api automatically detects if the 'ssid' is for real or demo account
    api = PocketOptionAsync(ssid)    
    await asyncio.sleep(5)
    
    # Candles are returned in the format of a list of dictionaries
    times = [ 3600 * i for i in range(1, 11)]
    time_frames = [ 1, 5, 15, 30, 60, 300]
    for time in times:
        for frame in time_frames:
            
            candles = await api.get_candles("EURUSD_otc", 60, time)
            # print(f"Raw Candles: {candles}")
            candles_pd = pd.DataFrame.from_dict(candles)
            print(f"Candles: {candles_pd}")
    
if __name__ == '__main__':
    ssid = input('Please enter your ssid: ')
    asyncio.run(main(ssid))
    
