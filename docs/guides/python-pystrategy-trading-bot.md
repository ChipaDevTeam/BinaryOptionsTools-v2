# Python Trading Bot Guide - PyStrategy Framework

Complete guide for building an advanced Pocket Option trading bot using the PyStrategy framework with async support.

## Table of Contents
- [Overview](#overview)
- [Prerequisites](#prerequisites)
- [Core Components](#core-components)
- [Basic Bot Structure](#basic-bot-structure)
- [PyStrategy Methods](#pystrategy-methods)
- [PyContext API](#pycontext-api)
- [Advanced Features](#advanced-features)
- [Complete Trading Bot Example](#complete-trading-bot-example)
- [Best Practices](#best-practices)

---

## Overview

The PyStrategy framework provides a high-level interface for building trading bots with:
- Event-driven architecture
- Automatic candle streaming
- Built-in trade management
- Virtual market interface
- Async/await support

## Prerequisites

```python
import asyncio
import os
import json
from datetime import datetime
from BinaryOptionsToolsV2 import RawPocketOption, PyBot, PyStrategy, start_tracing
from dotenv import load_dotenv
```

Required environment variable:
- `POCKET_OPTION_SSID` - Your Pocket Option session ID

---

## Core Components

### 1. RawPocketOption
Low-level client for Pocket Option API communication.

**Initialization:**
```python
client = await RawPocketOption.create(ssid)
```

**Methods:**
- `async buy(asset: str, amount: float, time: int) -> Tuple[str, Dict]`
- `async sell(asset: str, amount: float, time: int) -> Tuple[str, Dict]`
- `async check_win(trade_id: str) -> Dict`
- `async balance() -> float`
- `async opened_deals() -> List[Dict]`
- `async closed_deals() -> List[Dict]`
- `async clear_closed_deals() -> None`
- `async payout() -> Dict[str, int]`
- `async candles(asset: str, period: int) -> List[Dict]`
- `async get_candles(asset: str, period: int, offset: int) -> List[Dict]`
- `async get_server_time() -> int`
- `is_demo() -> bool`
- `async disconnect() -> None`
- `async connect() -> None`
- `async reconnect() -> None`

### 2. PyStrategy
Abstract base class for implementing trading strategies.

**Required Methods:**
- `on_start(ctx: PyContext) -> None` - Called when bot starts
- `on_candle(ctx: PyContext, asset: str, candle_json: str) -> None` - Called on new candle
- `on_stop() -> None` - Called when bot stops

### 3. PyBot
Bot orchestrator that connects client and strategy.

**Methods:**
- `__init__(client: RawPocketOption, strategy: PyStrategy)`
- `add_asset(asset: str, timeframe: int) -> None`
- `async run() -> None`

### 4. PyContext
Context object passed to strategy callbacks.

**Properties:**
- `market: PyVirtualMarket` - Virtual market interface
- `client: RawPocketOption` - Direct client access (for balance, etc.)

**Methods:**
- `get_time() -> int` - Get current timestamp

### 5. PyVirtualMarket
Simplified trading interface (accessed via `ctx.market`).

**Methods:**
- `balance() -> float`
- `buy(asset: str, amount: float, time: int) -> Tuple[str, Any]`
- `sell(asset: str, amount: float, time: int) -> Tuple[str, Any]`
- `check_win(id: str) -> Any`

---

## Basic Bot Structure

```python
import asyncio
import os
from BinaryOptionsToolsV2 import RawPocketOption, PyBot, PyStrategy, start_tracing
from dotenv import load_dotenv

load_dotenv()

class MyStrategy(PyStrategy):
    def on_start(self, ctx):
        print("Strategy initialized")
    
    def on_candle(self, ctx, asset, candle_json):
        print(f"New candle for {asset}")
    
    def on_stop(self):
        print("Strategy stopped")

async def main():
    start_tracing("info")  # Options: "debug", "info", "warn", "error"
    
    ssid = os.getenv("POCKET_OPTION_SSID")
    client = await RawPocketOption.create(ssid)
    
    strategy = MyStrategy()
    bot = PyBot(client, strategy)
    
    bot.add_asset("EURUSD_otc", 60)  # Monitor 60s candles
    
    await bot.run()

if __name__ == "__main__":
    asyncio.run(main())
```

---

## PyStrategy Methods

### on_start(ctx: PyContext)
Called once when the bot starts. Use for initialization.

**Input:**
- `ctx: PyContext` - Context object with market access

**Common Uses:**
- Initialize variables
- Load historical data
- Set up indicators
- Get initial balance

**Example:**
```python
def on_start(self, ctx):
    self.initial_balance = ctx.client.balance()
    self.trades = []
    self.win_count = 0
    self.loss_count = 0
    print(f"Starting balance: ${self.initial_balance:.2f}")
```

### on_candle(ctx: PyContext, asset: str, candle_json: str)
Called when a new candle arrives for monitored assets.

**Inputs:**
- `ctx: PyContext` - Context object
- `asset: str` - Asset symbol (e.g., "EURUSD_otc")
- `candle_json: str` - JSON string of candle data

**Candle Data Structure:**
```json
{
    "open": 1.08523,
    "high": 1.08545,
    "low": 1.08510,
    "close": 1.08530,
    "timestamp": 1704067200,
    "volume": 12345
}
```

**Example:**
```python
def on_candle(self, ctx, asset, candle_json):
    candle = json.loads(candle_json)
    
    # Access candle data
    close_price = candle["close"]
    high = candle["high"]
    low = candle["low"]
    
    # Execute trade logic (async)
    if self.should_buy(candle):
        task = asyncio.create_task(self.execute_buy(ctx, asset))
        self._tasks.add(task)
        task.add_done_callback(self._tasks.discard)
```

### on_stop()
Called when the bot stops. Use for cleanup.

**Example:**
```python
def on_stop(self):
    print(f"Bot stopped. Total trades: {len(self.trades)}")
    print(f"Wins: {self.win_count}, Losses: {self.loss_count}")
```

---

## PyContext API

### Accessing Market (Virtual Interface)

```python
# Get balance (synchronous within strategy)
balance = ctx.market.balance()

# Place buy trade (returns tuple: trade_id, trade_data)
trade_id, trade_data = ctx.market.buy("EURUSD_otc", 1.0, 60)

# Place sell trade
trade_id, trade_data = ctx.market.sell("EURUSD_otc", 1.0, 60)

# Check trade result
result = ctx.market.check_win(trade_id)
```

### Accessing Client (Async Interface)

For operations requiring async/await, use `ctx.client`:

```python
async def execute_trade(self, ctx, asset):
    # Get balance
    balance = await ctx.client.balance()
    
    # Place trade
    trade_id, trade_data = await ctx.client.buy(asset, 1.0, 60)
    
    # Check win
    result = await ctx.client.check_win(trade_id)
    
    # Get opened deals
    opened = await ctx.client.opened_deals()
    
    # Get payout percentage
    payout = await ctx.client.payout()
```

### Get Current Time

```python
current_timestamp = ctx.get_time()
```

---

## Advanced Features

### 1. Account Management

**Get Balance:**
```python
async def update_balance(self, ctx):
    self.balance = await ctx.client.balance()
    return self.balance
```

**Check Account Type:**
```python
is_demo = ctx.client.is_demo()
```

### 2. Trade Management

**Place Trade with Check Win:**
```python
async def trade_with_result(self, ctx, asset, amount, time):
    # Place trade
    trade_id, _ = await ctx.client.buy(asset, amount, time)
    
    # Wait for result
    result = await ctx.client.check_win(trade_id)
    
    return {
        "id": trade_id,
        "result": result.get("result"),  # "win", "loss", "draw"
        "profit": result.get("profit", 0)
    }
```

**Monitor Active Trades:**
```python
async def get_active_trades(self, ctx):
    opened = await ctx.client.opened_deals()
    return opened
```

**Get Trade History:**
```python
async def get_closed_trades(self, ctx):
    closed = await ctx.client.closed_deals()
    return closed

async def clear_history(self, ctx):
    await ctx.client.clear_closed_deals()
```

### 3. Profit/Loss Tracking

```python
class TradingStrategy(PyStrategy):
    def __init__(self):
        super().__init__()
        self.initial_balance = 0.0
        self.current_balance = 0.0
        self.total_trades = 0
        self.wins = 0
        self.losses = 0
    
    def on_start(self, ctx):
        self.initial_balance = ctx.market.balance()
        self.current_balance = self.initial_balance
    
    async def track_trade(self, ctx, trade_id):
        result = await ctx.client.check_win(trade_id)
        
        self.total_trades += 1
        profit = result.get("profit", 0)
        
        if profit > 0:
            self.wins += 1
        elif profit < 0:
            self.losses += 1
        
        self.current_balance = await ctx.client.balance()
        
        win_rate = (self.wins / self.total_trades * 100) if self.total_trades > 0 else 0
        net_profit = self.current_balance - self.initial_balance
        
        print(f"Trade completed: {result.get('result')}")
        print(f"Win Rate: {win_rate:.2f}% | Net P/L: ${net_profit:.2f}")
```

### 4. Stop Loss Implementation

```python
class StopLossStrategy(PyStrategy):
    def __init__(self, stop_loss_amount=50.0):
        super().__init__()
        self.stop_loss_amount = stop_loss_amount
        self.initial_balance = 0.0
    
    def on_start(self, ctx):
        self.initial_balance = ctx.market.balance()
    
    async def check_stop_loss(self, ctx):
        current_balance = await ctx.client.balance()
        loss = self.initial_balance - current_balance
        
        if loss >= self.stop_loss_amount:
            print(f"Stop loss triggered! Loss: ${loss:.2f}")
            return True
        return False
    
    def on_candle(self, ctx, asset, candle_json):
        task = asyncio.create_task(self.execute_with_stop_loss(ctx, asset, candle_json))
        self._tasks.add(task)
        task.add_done_callback(self._tasks.discard)
    
    async def execute_with_stop_loss(self, ctx, asset, candle_json):
        if await self.check_stop_loss(ctx):
            print("Trading halted due to stop loss")
            return
        
        # Continue with trading logic
        candle = json.loads(candle_json)
        # ... your strategy logic
```

### 5. Take Profit Implementation

```python
class TakeProfitStrategy(PyStrategy):
    def __init__(self, take_profit_amount=100.0):
        super().__init__()
        self.take_profit_amount = take_profit_amount
        self.initial_balance = 0.0
    
    def on_start(self, ctx):
        self.initial_balance = ctx.market.balance()
    
    async def check_take_profit(self, ctx):
        current_balance = await ctx.client.balance()
        profit = current_balance - self.initial_balance
        
        if profit >= self.take_profit_amount:
            print(f"Take profit triggered! Profit: ${profit:.2f}")
            return True
        return False
```

### 6. Dynamic Asset Switching

```python
class MultiAssetStrategy(PyStrategy):
    def __init__(self):
        super().__init__()
        self.assets = ["EURUSD_otc", "GBPUSD_otc", "USDJPY_otc"]
        self.asset_performance = {}
    
    def on_candle(self, ctx, asset, candle_json):
        candle = json.loads(candle_json)
        
        # Track performance per asset
        if asset not in self.asset_performance:
            self.asset_performance[asset] = {"wins": 0, "losses": 0}
        
        # Select best performing asset
        best_asset = self.get_best_asset()
        
        if asset == best_asset:
            # Trade only on best performing asset
            task = asyncio.create_task(self.execute_trade(ctx, asset))
            self._tasks.add(task)
            task.add_done_callback(self._tasks.discard)
    
    def get_best_asset(self):
        best = None
        best_ratio = 0
        
        for asset, perf in self.asset_performance.items():
            total = perf["wins"] + perf["losses"]
            if total > 0:
                ratio = perf["wins"] / total
                if ratio > best_ratio:
                    best_ratio = ratio
                    best = asset
        
        return best or self.assets[0]
```

### 7. Candle History Access

```python
async def get_historical_data(self, ctx, asset, period=60, count=100):
    """Get historical candles"""
    candles = await ctx.client.get_candles(asset, period, count)
    return candles

async def calculate_sma(self, ctx, asset, period=60, length=20):
    """Calculate Simple Moving Average"""
    candles = await ctx.client.get_candles(asset, period, length)
    prices = [c["close"] for c in candles]
    return sum(prices) / len(prices) if prices else 0
```

### 8. Payout Checking

```python
async def get_asset_payout(self, ctx, asset):
    """Get payout percentage for asset"""
    payout = await ctx.client.payout()
    return payout.get(asset, 0)

async def trade_best_payout(self, ctx, assets):
    """Trade asset with highest payout"""
    payout = await ctx.client.payout()
    best_asset = max(assets, key=lambda a: payout.get(a, 0))
    return best_asset
```

---

## Complete Trading Bot Example

Full implementation with all advanced features:

```python
import asyncio
import os
import json
from datetime import datetime
from BinaryOptionsToolsV2 import RawPocketOption, PyBot, PyStrategy, start_tracing
from dotenv import load_dotenv

load_dotenv()

class AdvancedTradingStrategy(PyStrategy):
    def __init__(
        self,
        initial_amount=1.0,
        stop_loss=50.0,
        take_profit=100.0,
        max_trades=10,
        rsi_period=14
    ):
        super().__init__()
        
        # Configuration
        self.initial_amount = initial_amount
        self.stop_loss_amount = stop_loss
        self.take_profit_amount = take_profit
        self.max_trades = max_trades
        self.rsi_period = rsi_period
        
        # State tracking
        self.initial_balance = 0.0
        self.current_balance = 0.0
        self.trades = []
        self.wins = 0
        self.losses = 0
        self.draws = 0
        self.trading_enabled = True
        
        # Asset data
        self.candle_history = {}
        self.prices = {}
        
        # Task management
        self._tasks = set()
        self._balance_task = None
    
    def on_start(self, ctx):
        """Initialize strategy"""
        self.start_time = datetime.now()
        self.initial_balance = ctx.market.balance()
        self.current_balance = self.initial_balance
        
        print("=" * 60)
        print(f"Advanced Trading Bot Started")
        print(f"Initial Balance: ${self.initial_balance:.2f}")
        print(f"Account Type: {'DEMO' if ctx.client.is_demo() else 'REAL'}")
        print(f"Stop Loss: ${self.stop_loss_amount:.2f}")
        print(f"Take Profit: ${self.take_profit_amount:.2f}")
        print(f"Max Trades: {self.max_trades}")
        print("=" * 60)
    
    def on_candle(self, ctx, asset, candle_json):
        """Process new candle"""
        candle = json.loads(candle_json)
        
        # Store candle history
        if asset not in self.candle_history:
            self.candle_history[asset] = []
        self.candle_history[asset].append(candle)
        
        # Keep only last 100 candles
        if len(self.candle_history[asset]) > 100:
            self.candle_history[asset].pop(0)
        
        # Store prices for indicators
        if asset not in self.prices:
            self.prices[asset] = []
        self.prices[asset].append(candle["close"])
        if len(self.prices[asset]) > 100:
            self.prices[asset].pop(0)
        
        # Update balance periodically
        if not hasattr(self, "_balance_task") or self._balance_task.done():
            self._balance_task = asyncio.create_task(self.update_balance(ctx))
        
        # Execute trading logic
        if self.trading_enabled and len(self.trades) < self.max_trades:
            task = asyncio.create_task(self.analyze_and_trade(ctx, asset, candle))
            self._tasks.add(task)
            task.add_done_callback(self._tasks.discard)
    
    def on_stop(self):
        """Cleanup on stop"""
        duration = datetime.now() - self.start_time
        net_profit = self.current_balance - self.initial_balance
        total_trades = len(self.trades)
        win_rate = (self.wins / total_trades * 100) if total_trades > 0 else 0
        
        print("\n" + "=" * 60)
        print("Trading Bot Stopped")
        print(f"Duration: {duration}")
        print(f"Initial Balance: ${self.initial_balance:.2f}")
        print(f"Final Balance: ${self.current_balance:.2f}")
        print(f"Net P/L: ${net_profit:.2f} ({net_profit/self.initial_balance*100:.2f}%)")
        print(f"Total Trades: {total_trades}")
        print(f"Wins: {self.wins} | Losses: {self.losses} | Draws: {self.draws}")
        print(f"Win Rate: {win_rate:.2f}%")
        print("=" * 60)
    
    async def update_balance(self, ctx):
        """Update current balance"""
        try:
            self.current_balance = await ctx.client.balance()
        except Exception as e:
            print(f"Error updating balance: {e}")
    
    async def check_risk_limits(self, ctx):
        """Check stop loss and take profit"""
        await self.update_balance(ctx)
        
        net_pnl = self.current_balance - self.initial_balance
        
        # Check stop loss
        if net_pnl <= -self.stop_loss_amount:
            print(f"\n⛔ STOP LOSS TRIGGERED! Loss: ${-net_pnl:.2f}")
            self.trading_enabled = False
            return False
        
        # Check take profit
        if net_pnl >= self.take_profit_amount:
            print(f"\n✅ TAKE PROFIT TRIGGERED! Profit: ${net_pnl:.2f}")
            self.trading_enabled = False
            return False
        
        return True
    
    async def analyze_and_trade(self, ctx, asset, candle):
        """Main trading logic"""
        try:
            # Check risk limits
            if not await self.check_risk_limits(ctx):
                return
            
            # Wait for enough data
            if asset not in self.prices or len(self.prices[asset]) < self.rsi_period + 1:
                return
            
            # Calculate indicators
            rsi = self.calculate_rsi(asset)
            sma_20 = self.calculate_sma(asset, 20)
            
            # Get payout
            payout_data = await ctx.client.payout()
            payout = payout_data.get(asset, 0)
            
            if payout < 70:  # Skip if payout too low
                return
            
            # Trading signals
            signal = None
            
            # RSI strategy
            if rsi < 30 and candle["close"] > sma_20:
                signal = "buy"
            elif rsi > 70 and candle["close"] < sma_20:
                signal = "sell"
            
            # Execute trade
            if signal:
                await self.execute_trade(ctx, asset, signal, candle, rsi, sma_20, payout)
        
        except Exception as e:
            print(f"Error in analyze_and_trade: {e}")
    
    async def execute_trade(self, ctx, asset, signal, candle, rsi, sma, payout):
        """Execute and track trade"""
        try:
            amount = self.calculate_position_size()
            expiry_time = 60  # 1 minute
            
            # Place trade
            if signal == "buy":
                trade_id, _ = await ctx.client.buy(asset, amount, expiry_time)
            else:
                trade_id, _ = await ctx.client.sell(asset, amount, expiry_time)
            
            trade_info = {
                "id": trade_id,
                "asset": asset,
                "signal": signal,
                "amount": amount,
                "time": expiry_time,
                "entry_price": candle["close"],
                "rsi": rsi,
                "sma": sma,
                "payout": payout,
                "timestamp": datetime.now()
            }
            
            self.trades.append(trade_info)
            
            print(f"\n📊 Trade #{len(self.trades)}: {signal.upper()} {asset}")
            print(f"   Amount: ${amount:.2f} | RSI: {rsi:.2f} | Price: {candle['close']:.5f}")
            print(f"   Payout: {payout}% | Balance: ${self.current_balance:.2f}")
            
            # Wait for result
            await self.wait_and_check_result(ctx, trade_id, trade_info)
        
        except Exception as e:
            print(f"Error executing trade: {e}")
    
    async def wait_and_check_result(self, ctx, trade_id, trade_info):
        """Wait for trade result"""
        try:
            result = await ctx.client.check_win(trade_id)
            
            profit = result.get("profit", 0)
            
            if profit > 0:
                self.wins += 1
                outcome = "WIN ✅"
            elif profit < 0:
                self.losses += 1
                outcome = "LOSS ❌"
            else:
                self.draws += 1
                outcome = "DRAW ⚖️"
            
            trade_info["result"] = outcome
            trade_info["profit"] = profit
            
            await self.update_balance(ctx)
            
            win_rate = (self.wins / len(self.trades) * 100) if len(self.trades) > 0 else 0
            net_pnl = self.current_balance - self.initial_balance
            
            print(f"   Result: {outcome} | P/L: ${profit:.2f}")
            print(f"   Win Rate: {win_rate:.2f}% | Net P/L: ${net_pnl:.2f}")
        
        except Exception as e:
            print(f"Error checking result: {e}")
    
    def calculate_position_size(self):
        """Calculate trade amount based on balance"""
        # Risk 1% of current balance per trade
        risk_per_trade = self.current_balance * 0.01
        return max(self.initial_amount, risk_per_trade)
    
    def calculate_rsi(self, asset, period=None):
        """Calculate RSI indicator"""
        if period is None:
            period = self.rsi_period
        
        prices = self.prices.get(asset, [])
        if len(prices) < period + 1:
            return 50  # Neutral
        
        deltas = [prices[i] - prices[i-1] for i in range(1, len(prices))]
        gains = [d if d > 0 else 0 for d in deltas[-period:]]
        losses = [-d if d < 0 else 0 for d in deltas[-period:]]
        
        avg_gain = sum(gains) / period
        avg_loss = sum(losses) / period
        
        if avg_loss == 0:
            return 100
        
        rs = avg_gain / avg_loss
        rsi = 100 - (100 / (1 + rs))
        
        return rsi
    
    def calculate_sma(self, asset, period=20):
        """Calculate Simple Moving Average"""
        prices = self.prices.get(asset, [])
        if len(prices) < period:
            return prices[-1] if prices else 0
        
        return sum(prices[-period:]) / period


async def main():
    # Enable tracing
    start_tracing("info")
    
    # Get credentials
    ssid = os.getenv("POCKET_OPTION_SSID")
    if not ssid:
        print("Error: POCKET_OPTION_SSID not set in .env file")
        return
    
    print("Connecting to Pocket Option...")
    client = await RawPocketOption.create(ssid)
    
    # Wait for assets to load
    await client.wait_for_assets(timeout_secs=30.0)
    
    # Create strategy
    strategy = AdvancedTradingStrategy(
        initial_amount=1.0,
        stop_loss=50.0,
        take_profit=100.0,
        max_trades=10,
        rsi_period=14
    )
    
    # Create bot
    bot = PyBot(client, strategy)
    
    # Add assets to monitor (60 second candles)
    bot.add_asset("EURUSD_otc", 60)
    bot.add_asset("GBPUSD_otc", 60)
    bot.add_asset("USDJPY_otc", 60)
    
    print("Bot running... Press Ctrl+C to stop.\n")
    
    try:
        await bot.run()
    except KeyboardInterrupt:
        print("\nShutting down...")
    finally:
        await client.disconnect()


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        pass
```

---

## Best Practices

### 1. Task Management
Always track async tasks to prevent memory leaks:

```python
def __init__(self):
    super().__init__()
    self._tasks = set()

def on_candle(self, ctx, asset, candle_json):
    task = asyncio.create_task(self.execute_trade(ctx, asset))
    self._tasks.add(task)
    task.add_done_callback(self._tasks.discard)
```

### 2. Error Handling
Wrap async operations in try-except blocks:

```python
async def execute_trade(self, ctx, asset):
    try:
        result = await ctx.client.buy(asset, 1.0, 60)
        print(f"Trade executed: {result}")
    except Exception as e:
        print(f"Trade failed: {e}")
```

### 3. Balance Updates
Update balance periodically, not on every candle:

```python
if not hasattr(self, "_balance_task") or self._balance_task.done():
    self._balance_task = asyncio.create_task(self.update_balance(ctx))
```

### 4. Data Validation
Always validate candle data:

```python
def on_candle(self, ctx, asset, candle_json):
    try:
        candle = json.loads(candle_json)
        if "close" not in candle or "high" not in candle:
            return
        # ... rest of logic
    except json.JSONDecodeError:
        print(f"Invalid candle data for {asset}")
```

### 5. Risk Management
- Always implement stop loss
- Always implement take profit
- Never risk more than 1-2% per trade
- Set maximum daily trades limit

### 6. Logging
Use proper logging for production:

```python
start_tracing("warn")  # Use "warn" or "error" in production
```

### 7. Graceful Shutdown
Handle cleanup properly:

```python
try:
    await bot.run()
except KeyboardInterrupt:
    print("Shutting down...")
finally:
    await client.disconnect()
```

---

## Summary

**Key Points:**
- Use `PyStrategy` for event-driven trading logic
- Access market via `ctx.market` for simple operations
- Access client via `ctx.client` for async operations
- Always track tasks with `set()` and `add_done_callback()`
- Implement risk management (stop loss, take profit)
- Handle errors gracefully
- Use proper task management to avoid leaks

**Function Reference:**

| Function | Type | Description |
|----------|------|-------------|
| `ctx.market.balance()` | sync | Get current balance |
| `ctx.market.buy()` | sync | Place buy trade |
| `ctx.market.sell()` | sync | Place sell trade |
| `ctx.client.balance()` | async | Get current balance |
| `ctx.client.buy()` | async | Place buy trade |
| `ctx.client.check_win()` | async | Check trade result |
| `ctx.client.opened_deals()` | async | Get active trades |
| `ctx.client.closed_deals()` | async | Get closed trades |
| `ctx.client.payout()` | async | Get payout percentages |
| `ctx.get_time()` | sync | Get current timestamp |

**Next Steps:**
1. Set up your `.env` file with `POCKET_OPTION_SSID`
2. Start with the basic structure
3. Add your trading logic in `on_candle()`
4. Test on demo account first
5. Implement risk management
6. Monitor and optimize

---

*For more examples, see `docs/examples/python/async/` directory*
