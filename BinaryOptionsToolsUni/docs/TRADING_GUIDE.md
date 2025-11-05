# Trading Guide - BinaryOptionsToolsUni

Complete guide to trading binary options using BinaryOptionsToolsUni across all supported languages.

## Table of Contents
- [Getting Started](#getting-started)
- [Trading Basics](#trading-basics)
- [Advanced Trading Strategies](#advanced-trading-strategies)
- [Risk Management](#risk-management)
- [Common Patterns](#common-patterns)
- [Troubleshooting](#troubleshooting)

---

## Getting Started

### Prerequisites

Before you start trading, ensure you have:

1. **PocketOption SSID**: Your session ID from PocketOption Quick Trading
2. **Demo Account**: Start with demo account to test strategies
3. **Stable Internet**: Reliable connection for real-time trading
4. **Risk Management Plan**: Never risk more than you can afford to lose

### Your First Trade

Here's a complete example of placing your first trade:

<details>
<summary><b>Python</b></summary>

```python
import asyncio
from binaryoptionstoolsuni import PocketOption

async def first_trade():
    # Initialize client
    client = await PocketOption.init("your_ssid")
    await asyncio.sleep(2)  # Wait for initialization
    
    # Check account type
    if not client.is_demo():
        print("⚠️ WARNING: Using REAL account!")
        return
    
    # Check balance
    balance = await client.balance()
    print(f"Balance: ${balance:.2f}")
    
    # Place a small test trade
    trade = await client.buy("EURUSD_otc", 60, 1.0)
    print(f"Trade placed! ID: {trade.id}")
    
    # Wait for result (60 seconds + buffer)
    await asyncio.sleep(65)
    
    # Check result
    result = await client.result(trade.id)
    if result.profit > 0:
        print(f"✅ WIN! Profit: ${result.profit:.2f}")
    else:
        print(f"❌ LOSS! Loss: ${abs(result.profit):.2f}")
    
    # Shutdown
    await client.shutdown()

asyncio.run(first_trade())
```
</details>

<details>
<summary><b>Kotlin</b></summary>

```kotlin
import com.chipadevteam.binaryoptionstoolsuni.*
import kotlinx.coroutines.*

suspend fun firstTrade() = coroutineScope {
    // Initialize client
    val client = PocketOption.init("your_ssid")
    delay(2000)
    
    // Check account type
    if (!client.isDemo()) {
        println("⚠️ WARNING: Using REAL account!")
        return@coroutineScope
    }
    
    // Check balance
    val balance = client.balance()
    println("Balance: $$balance")
    
    // Place a small test trade
    val trade = client.buy("EURUSD_otc", 60u, 1.0)
    println("Trade placed! ID: ${trade.id}")
    
    // Wait for result
    delay(65000)
    
    // Check result
    val result = client.result(trade.id)
    if (result.profit > 0) {
        println("✅ WIN! Profit: $${result.profit}")
    } else {
        println("❌ LOSS! Loss: $${kotlin.math.abs(result.profit)}")
    }
    
    // Shutdown
    client.shutdown()
}
```
</details>

---

## Trading Basics

### Trade Types

#### Call (Buy) Trade
Predict that the price will go **UP** at expiration.

```python
trade = await client.buy("EURUSD_otc", 60, 1.0)
```

#### Put (Sell) Trade
Predict that the price will go **DOWN** at expiration.

```python
trade = await client.sell("EURUSD_otc", 60, 1.0)
```

### Trade Parameters

| Parameter | Type | Description | Example |
|-----------|------|-------------|---------|
| `asset` | String | Trading pair/asset | `"EURUSD_otc"` |
| `time` | Integer | Expiration time in seconds | `60`, `120`, `300` |
| `amount` | Float | Trade amount in USD | `1.0`, `5.0`, `10.0` |

### Common Expiration Times

- **60 seconds**: Fast scalping
- **120 seconds (2 minutes)**: Quick trades
- **300 seconds (5 minutes)**: Short-term analysis
- **600 seconds (10 minutes)**: Medium-term analysis
- **900 seconds (15 minutes)**: Longer-term analysis

---

## Advanced Trading Strategies

### 1. Martingale Strategy

⚠️ **HIGH RISK**: Can deplete balance quickly!

```python
async def martingale_strategy(client, asset, initial_amount=1.0, max_rounds=5):
    """
    Double bet after each loss to recover losses + profit.
    WARNING: Very risky! Use only on demo account.
    """
    amount = initial_amount
    
    for round in range(max_rounds):
        # Place trade
        trade = await client.buy(asset, 60, amount)
        print(f"Round {round + 1}: ${amount:.2f}")
        
        # Wait for result
        await asyncio.sleep(65)
        
        # Check result
        result = await client.result(trade.id)
        
        if result.profit > 0:
            print(f"✅ WIN! Profit: ${result.profit:.2f}")
            return True  # Success!
        else:
            print(f"❌ LOSS! Loss: ${abs(result.profit):.2f}")
            amount *= 2  # Double the bet
            
            # Check if we have enough balance
            balance = await client.balance()
            if balance < amount:
                print("⚠️ Insufficient balance!")
                return False
    
    print("❌ Max rounds reached. Strategy failed.")
    return False
```

### 2. Trend Following

```python
async def trend_following(client, asset, period=60):
    """
    Follow the trend based on recent candles.
    """
    # Get recent candles
    candles = await client.get_candles(asset, period, 10)
    
    # Calculate trend
    closes = [c.close for c in candles]
    trend = "UP" if closes[-1] > closes[0] else "DOWN"
    
    # Trade with the trend
    if trend == "UP":
        trade = await client.buy(asset, period, 1.0)
        print(f"📈 Trend UP - Placed CALL")
    else:
        trade = await client.sell(asset, period, 1.0)
        print(f"📉 Trend DOWN - Placed PUT")
    
    return trade
```

### 3. Multiple Asset Trading

```python
async def multi_asset_trading(client, assets, amount=1.0):
    """
    Trade multiple assets simultaneously for diversification.
    """
    trades = []
    
    for asset in assets:
        # Analyze each asset
        candles = await client.get_candles(asset, 60, 5)
        
        # Simple momentum strategy
        if candles[-1].close > candles[-2].close:
            trade = await client.buy(asset, 60, amount)
            trades.append((asset, "CALL", trade))
        else:
            trade = await client.sell(asset, 60, amount)
            trades.append((asset, "PUT", trade))
    
    # Wait for all trades to complete
    await asyncio.sleep(65)
    
    # Check results
    total_profit = 0
    for asset, action, trade in trades:
        result = await client.result(trade.id)
        total_profit += result.profit
        status = "WIN" if result.profit > 0 else "LOSS"
        print(f"{asset} ({action}): {status} ${result.profit:.2f}")
    
    print(f"Total Profit: ${total_profit:.2f}")
    return total_profit

# Usage
assets = ["EURUSD_otc", "GBPUSD_otc", "USDJPY_otc"]
await multi_asset_trading(client, assets)
```

---

## Risk Management

### 1. Never Risk More Than 2% Per Trade

```python
async def safe_trade_size(client, risk_percentage=0.02):
    """
    Calculate safe trade size based on balance.
    """
    balance = await client.balance()
    max_trade_size = balance * risk_percentage
    
    print(f"Balance: ${balance:.2f}")
    print(f"Max trade size (2%): ${max_trade_size:.2f}")
    
    return max_trade_size
```

### 2. Set Daily Loss Limit

```python
class TradingSession:
    def __init__(self, client, max_daily_loss=10.0):
        self.client = client
        self.max_daily_loss = max_daily_loss
        self.daily_pnl = 0.0
        
    async def can_trade(self):
        """Check if we haven't hit daily loss limit."""
        if abs(self.daily_pnl) >= self.max_daily_loss:
            print("⚠️ Daily loss limit reached!")
            return False
        return True
    
    async def trade(self, asset, action, time, amount):
        """Place trade with loss limit check."""
        if not await self.can_trade():
            return None
        
        # Place trade
        if action == "buy":
            trade = await self.client.buy(asset, time, amount)
        else:
            trade = await self.client.sell(asset, time, amount)
        
        # Update P&L after trade completes
        # (simplified - you'd wait for result in real code)
        return trade
```

### 3. Position Sizing

```python
def calculate_position_size(balance, risk_per_trade, win_rate):
    """
    Kelly Criterion for optimal position sizing.
    """
    if win_rate <= 0.5:
        return balance * 0.01  # Minimum 1%
    
    # Simplified Kelly formula
    kelly = win_rate - ((1 - win_rate) / 1.8)  # Assuming 80% payout
    
    # Use half-Kelly for safety
    safe_kelly = kelly / 2
    
    return balance * min(safe_kelly, 0.02)  # Cap at 2%
```

---

## Common Patterns

### 1. Retry Pattern for Network Issues

```python
async def trade_with_retry(client, asset, action, time, amount, max_retries=3):
    """
    Retry trade placement if it fails.
    """
    for attempt in range(max_retries):
        try:
            if action == "buy":
                trade = await client.buy(asset, time, amount)
            else:
                trade = await client.sell(asset, time, amount)
            return trade
        except Exception as e:
            print(f"Attempt {attempt + 1} failed: {e}")
            if attempt < max_retries - 1:
                await asyncio.sleep(2)
                await client.reconnect()
                await asyncio.sleep(2)
    
    raise Exception("Failed after max retries")
```

### 2. Trade Monitoring

```python
async def monitor_trade(client, trade_id, timeout=120):
    """
    Monitor trade and get result with timeout.
    """
    start_time = asyncio.get_event_loop().time()
    
    while True:
        # Check if timeout reached
        if asyncio.get_event_loop().time() - start_time > timeout:
            print("⚠️ Timeout waiting for result")
            return None
        
        # Try to get result
        try:
            result = await client.result(trade_id)
            if result.profit != 0:  # Trade completed
                return result
        except Exception as e:
            pass  # Trade not finished yet
        
        # Wait before checking again
        await asyncio.sleep(5)
```

### 3. Batch Trading

```python
async def batch_trade(client, signals):
    """
    Execute multiple trades from signals.
    
    signals = [
        ("EURUSD_otc", "buy", 60, 1.0),
        ("GBPUSD_otc", "sell", 60, 1.0),
    ]
    """
    trades = []
    
    for asset, action, time, amount in signals:
        try:
            if action == "buy":
                trade = await client.buy(asset, time, amount)
            else:
                trade = await client.sell(asset, time, amount)
            
            trades.append(trade)
            print(f"✅ {asset} {action.upper()} placed")
            
            # Small delay to avoid rate limiting
            await asyncio.sleep(0.5)
            
        except Exception as e:
            print(f"❌ {asset} {action.upper()} failed: {e}")
    
    return trades
```

---

## Troubleshooting

### Common Issues

#### 1. "Connection Failed" Error

**Problem**: Can't connect to PocketOption servers.

**Solutions**:
- Verify your SSID is correct and not expired
- Check internet connection
- Try reconnecting: `await client.reconnect()`
- Ensure PocketOption Quick Trading is working in browser

#### 2. "Trade Not Placed" Error

**Problem**: Trade placement fails.

**Solutions**:
- Check if market is open (avoid weekends for non-OTC assets)
- Verify asset name is correct (e.g., "EURUSD_otc")
- Ensure sufficient balance
- Try with smaller amount first

#### 3. "Result Not Found" Error

**Problem**: Can't get trade result.

**Solutions**:
- Wait longer - trade may not have expired yet
- Use `result_with_timeout()` instead of `result()`
- Check trade ID is correct
- Verify trade actually completed

#### 4. Slow Performance

**Problem**: API calls are very slow.

**Solutions**:
- Ensure 2-second initialization wait after creating client
- Don't create multiple clients - reuse one client
- Check network latency
- Avoid making too many rapid API calls

### Debug Mode

```python
# Enable detailed logging
import logging
logging.basicConfig(level=logging.DEBUG)

# Now all API calls will show debug information
```

---

## Best Practices Summary

### ✅ DO

- Always wait 2 seconds after initialization
- Start with demo account
- Use small trade sizes (1-2% of balance)
- Set daily loss limits
- Test strategies thoroughly
- Shutdown client when done
- Handle errors gracefully
- Keep track of P&L

### ❌ DON'T

- Risk more than 2% per trade
- Use Martingale on real money
- Trade without a strategy
- Chase losses
- Trade while emotional
- Ignore risk management
- Leave clients running indefinitely
- Trade during high news volatility

---

## Complete Example: Trading Bot

```python
import asyncio
from binaryoptionstoolsuni import PocketOption

class TradingBot:
    def __init__(self, ssid, max_daily_loss=10.0, risk_per_trade=0.02):
        self.ssid = ssid
        self.client = None
        self.max_daily_loss = max_daily_loss
        self.risk_per_trade = risk_per_trade
        self.daily_pnl = 0.0
        
    async def start(self):
        """Initialize the bot."""
        self.client = await PocketOption.init(self.ssid)
        await asyncio.sleep(2)
        print("✅ Bot started")
        
        # Verify demo account
        if not self.client.is_demo():
            print("⚠️ WARNING: Using REAL account!")
            response = input("Continue? (yes/no): ")
            if response.lower() != "yes":
                await self.stop()
                return False
        
        balance = await self.client.balance()
        print(f"Balance: ${balance:.2f}")
        return True
    
    async def can_trade(self):
        """Check if we can still trade today."""
        if abs(self.daily_pnl) >= self.max_daily_loss:
            print(f"⚠️ Daily loss limit reached: ${self.daily_pnl:.2f}")
            return False
        return True
    
    async def calculate_trade_size(self):
        """Calculate safe trade size."""
        balance = await self.client.balance()
        return balance * self.risk_per_trade
    
    async def analyze_market(self, asset, period=60):
        """Simple market analysis."""
        candles = await self.client.get_candles(asset, period, 5)
        
        # Simple trend detection
        closes = [c.close for c in candles]
        if closes[-1] > closes[0]:
            return "buy"
        else:
            return "sell"
    
    async def execute_trade(self, asset, period=60):
        """Execute a single trade."""
        if not await self.can_trade():
            return None
        
        # Analyze market
        action = await self.analyze_market(asset, period)
        amount = await self.calculate_trade_size()
        
        # Place trade
        if action == "buy":
            trade = await self.client.buy(asset, period, amount)
        else:
            trade = await self.client.sell(asset, period, amount)
        
        print(f"📊 {asset} {action.upper()} ${amount:.2f}")
        
        # Wait for result
        await asyncio.sleep(period + 5)
        
        # Get result
        result = await self.client.result(trade.id)
        self.daily_pnl += result.profit
        
        status = "WIN" if result.profit > 0 else "LOSS"
        print(f"{status}: ${result.profit:.2f} | Daily P&L: ${self.daily_pnl:.2f}")
        
        return result
    
    async def run(self, assets, trades_per_asset=5):
        """Run the trading bot."""
        if not await self.start():
            return
        
        try:
            for asset in assets:
                for i in range(trades_per_asset):
                    if not await self.can_trade():
                        break
                    
                    await self.execute_trade(asset)
                    await asyncio.sleep(5)  # Cooldown
        
        finally:
            await self.stop()
    
    async def stop(self):
        """Stop the bot."""
        if self.client:
            await self.client.shutdown()
        print(f"Bot stopped. Final P&L: ${self.daily_pnl:.2f}")

# Usage
async def main():
    bot = TradingBot(
        ssid="your_ssid",
        max_daily_loss=10.0,
        risk_per_trade=0.02
    )
    
    assets = ["EURUSD_otc", "GBPUSD_otc"]
    await bot.run(assets, trades_per_asset=3)

asyncio.run(main())
```

---

## Support

- **Discord**: [Join our community](https://discord.gg/p7YyFqSmAz)
- **GitHub Issues**: [Report problems](https://github.com/ChipaDevTeam/BinaryOptionsTools-v2/issues)

**Remember**: Trading binary options involves significant risk. Never trade with money you cannot afford to lose.
