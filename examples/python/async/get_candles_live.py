import asyncio
import os
import sys
from pathlib import Path
from dotenv import load_dotenv
from contextlib import aclosing

# Ensure we can import BinaryOptionsToolsV2 from the local python/ directory
sys.path.insert(0, str(Path(__file__).resolve().parents[3] / "python"))

from BinaryOptionsToolsV2.pocketoption.asynchronous import PocketOptionAsync

async def main():
    # Load .env file
    env_path = Path(__file__).resolve().parents[3] / ".env"
    if env_path.exists():
        load_dotenv(env_path)
    
    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        print("Error: POCKET_OPTION_SSID not found in .env")
        sys.exit(1)

    print("Connecting to Pocket Option using SSID...")
    async with PocketOptionAsync(ssid) as client:
        # Wait a moment for connection stabilization
        await asyncio.sleep(2)
        print(f"Connected: {client.is_connected()}")
        print(f"Is Demo Account: {client.is_demo()}")
        print(f"Current Balance: {await client.balance()}")

        print("\nStreaming live candles for EURUSD_otc...")
        # Stream live candles (yields a tuple: closed_candles list, current_forming_candle dict)
        generator = client.get_candles_live("EURUSD_otc", period=60, hours=1.0, max_rows=10)
        
        # We use aclosing to ensure the generator is closed and its finally block runs
        # before the client context is exited.
        count = 0
        async with aclosing(generator) as stream:
            async for closed, forming in stream:
                print(f"\nYield #{count + 1}:")
                print(f"Closed candles count: {len(closed)}")
                if closed:
                    print(f"Last closed candle: {closed[-1]}")
                if forming:
                    print(f"Currently forming candle: {forming}")
                
                count += 1
                if count >= 3:
                    break

if __name__ == "__main__":
    asyncio.run(main())
