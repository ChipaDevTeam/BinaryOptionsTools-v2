import os
import sys
import argparse

TEMPLATE = """import asyncio
import os
import json
from BinaryOptionsToolsV2 import RawPocketOption, PyBot, PyStrategy, start_tracing
from dotenv import load_dotenv

load_dotenv()

class MyStrategy(PyStrategy):
    def on_start(self, ctx):
        print("Strategy initialized and ready.")

    def on_candle(self, ctx, asset, candle_json):
        candle = json.loads(candle_json)
        print(f"[{asset}] Candle closed at: {candle['close']}")
        # Add your logic here!

async def main():
    start_tracing("info")
    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        print("Error: POCKET_OPTION_SSID not found in .env")
        return

    client = await RawPocketOption.create(ssid)
    bot = PyBot(client, MyStrategy())
    
    # Configure assets and timeframes (in seconds)
    bot.add_asset("EURUSD_otc", 60)
    
    print("Bot is running...")
    await bot.run()

if __name__ == "__main__":
    asyncio.run(main())
"""

DOTENV_TEMPLATE = """POCKET_OPTION_SSID=your_ssid_here
"""

def init_project(name):
    if os.path.exists(name):
        print(f"Error: Directory {name} already exists.")
        return

    os.makedirs(name)
    with open(os.path.join(name, "bot.py"), "w") as f:
        f.write(TEMPLATE)
    
    with open(os.path.join(name, ".env"), "w") as f:
        f.write(DOTENV_TEMPLATE)
    
    print(f"Project {name} initialized successfully!")
    print(f"Next steps:")
    print(f"  1. cd {name}")
    print(f"  2. Edit .env and add your POCKET_OPTION_SSID")
    print(f"  3. Run your bot: python bot.py")

def main():
    parser = argparse.ArgumentParser(description="BinaryOptionsTools Bot CLI")
    subparsers = parser.add_subparsers(dest="command")

    init_parser = subparsers.add_parser("init", help="Initialize a new bot project")
    init_parser.add_argument("name", help="Name of the project directory")

    args = parser.parse_args()

    if args.command == "init":
        init_project(args.name)
    else:
        parser.print_help()

if __name__ == "__main__":
    main()
