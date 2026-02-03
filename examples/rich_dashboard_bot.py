import asyncio
import os
import json
from datetime import datetime
from BinaryOptionsToolsV2 import RawPocketOption, PyBot, PyStrategy, start_tracing
from dotenv import load_dotenv
from rich.console import Console
from rich.table import Table
from rich.live import Live
from rich.panel import Panel
from rich.layout import Layout

load_dotenv()

console = Console()

class DashboardStrategy(PyStrategy):
    def __init__(self):
        super().__init__()
        self.last_candles = {}
        self.trades = []
        self.balance = 0.0

    def on_start(self, ctx):
        self.start_time = datetime.now()

    def on_candle(self, ctx, asset, candle_json):
        candle = json.loads(candle_json)
        self.last_candles[asset] = candle
        
        # Random simulation of balance check
        asyncio.create_task(self.update_balance(ctx))

    async def update_balance(self, ctx):
        self.balance = await ctx.client.balance()

    def make_layout(self):
        layout = Layout()
        layout.split_column(
            Layout(name="header", size=3),
            Layout(name="main"),
            Layout(name="footer", size=3)
        )
        layout["main"].split_row(
            Layout(name="market"),
            Layout(name="trades")
        )
        return layout

    def generate_table(self):
        table = Table(title="Market Overview")
        table.add_column("Asset")
        table.add_column("Price")
        table.add_column("High")
        table.add_column("Low")
        table.add_column("Time")

        for asset, candle in self.last_candles.items():
            table.add_row(
                asset,
                f"{candle['close']:.5f}",
                f"{candle['high']:.5f}",
                f"{candle['low']:.5f}",
                datetime.fromtimestamp(candle['timestamp']).strftime("%H:%M:%S")
            )
        return table

async def main():
    # start_tracing("warn") # Keep tracing quiet for dashboard
    
    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        print("Set POCKET_OPTION_SSID in .env")
        return

    client = await RawPocketOption.create(ssid)
    strategy = DashboardStrategy()
    bot = PyBot(client, strategy)
    bot.add_asset("EURUSD_otc", 60)
    bot.add_asset("GBPUSD_otc", 60)

    layout = strategy.make_layout()
    
    with Live(layout, refresh_per_second=4, screen=True):
        layout["header"].update(Panel(f"BinaryOptionsTools Bot Dashboard | Balance: ${strategy.balance:.2f}"))
        layout["footer"].update(Panel("Press Ctrl+C to exit"))
        
        # Start bot in background
        bot_task = asyncio.create_task(bot.run())
        
        try:
            while True:
                layout["main"]["market"].update(strategy.generate_table())
                layout["header"].update(Panel(f"BinaryOptionsTools Bot Dashboard | Balance: ${strategy.balance:.2f} | Uptime: {datetime.now() - strategy.start_time}"))
                await asyncio.sleep(0.5)
        except asyncio.CancelledError:
            pass

if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        pass
