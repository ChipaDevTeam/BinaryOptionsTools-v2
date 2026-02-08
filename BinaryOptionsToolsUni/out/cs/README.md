# BinaryOptionsTools - C# Bindings

C# bindings for BinaryOptionsTools, providing async access to PocketOption trading platform via UniFFI.

## 🚀 Features

- ✅ **Async Trading Operations**: Place buy/sell trades
- ✅ **Account Management**: Get balance and account information
- ✅ **Real-time Data**: Subscribe to asset price feeds
- ✅ **Trade Monitoring**: Check trade results and opened deals

## 📦 Installation

```bash
# Add the binary_options_tools_uni.cs file to your project
# Make sure to include the native library in your build output
```

## 🔧 Quick Start

### Basic Example

```csharp
using BinaryOptionsToolsUni;

class Program
{
    static async Task Main(string[] args)
    {
        // Initialize client with your session ID
        var client = await PocketOption.NewAsync("your-session-id");

        // IMPORTANT: Wait for connection to establish
        await Task.Delay(5000);

        // Get account balance
        var balance = await client.BalanceAsync();
        Console.WriteLine($"Account Balance: ${balance}");

        // Place a buy trade
        var deal = await client.BuyAsync("EURUSD_otc", 60, 1.0);
        Console.WriteLine($"Trade placed: {deal}");
    }
}
```

## 📖 Detailed Examples

### Buy Trade Example

```csharp
using BinaryOptionsToolsUni;

public class BuyTradeExample
{
    public static async Task Run()
    {
        // Initialize client
        var client = await PocketOption.NewAsync("your-session-id");
        await Task.Delay(5000);  // Wait for connection

        // Place a buy trade on EURUSD for 60 seconds with $1
        var deal = await client.BuyAsync(
            asset: "EURUSD_otc",
            time: 60,
            amount: 1.0
        );

        Console.WriteLine("Trade placed successfully!");
        Console.WriteLine($"Deal data: {deal}");
    }
}
```

### Sell Trade Example

```csharp
using BinaryOptionsToolsUni;

public class SellTradeExample
{
    public static async Task Run()
    {
        // Initialize client
        var client = await PocketOption.NewAsync("your-session-id");
        await Task.Delay(5000);  // Wait for connection

        // Place a sell trade on EURUSD for 60 seconds with $1
        var deal = await client.SellAsync(
            asset: "EURUSD_otc",
            time: 60,
            amount: 1.0
        );

        Console.WriteLine("Trade placed successfully!");
        Console.WriteLine($"Deal data: {deal}");
    }
}
```

### Check Balance Example

```csharp
using BinaryOptionsToolsUni;

public class BalanceExample
{
    public static async Task Run()
    {
        // Initialize client
        var client = await PocketOption.NewAsync("your-session-id");
        await Task.Delay(5000);  // Wait for connection

        // Get current balance
        var balance = await client.BalanceAsync();
        Console.WriteLine($"Your current balance is: ${balance}");
    }
}
```

### Check Trade Result Example

```csharp
using BinaryOptionsToolsUni;

public class CheckWinExample
{
    public static async Task Run()
    {
        // Initialize client
        var client = await PocketOption.NewAsync("your-session-id");
        await Task.Delay(5000);  // Wait for connection

        // Place a trade
        var deal = await client.BuyAsync("EURUSD_otc", 60, 1.0);
        var tradeId = deal.Id;  // Extract trade ID from deal

        // Wait for trade to complete
        await Task.Delay(65000);

        // Check the result
        var result = await client.CheckWinAsync(tradeId);
        Console.WriteLine($"Trade result: {result}");
    }
}
```

### Subscribe to Real-time Data

```csharp
using BinaryOptionsToolsUni;

public class SubscribeExample
{
    public static async Task Run()
    {
        // Initialize client
        var client = await PocketOption.NewAsync("your-session-id");
        await Task.Delay(5000);  // Wait for connection

        // Subscribe to real-time candle data for EURUSD
        // Duration in seconds for each candle
        var subscription = await client.SubscribeAsync("EURUSD_otc", 60);

        Console.WriteLine("Listening for real-time candles...");
        // Process subscription stream
    }
}
```

## 🔑 Important Notes

### Connection Initialization

**Always wait 5 seconds after creating the client** to allow the WebSocket connection to establish:

```csharp
var client = await PocketOption.NewAsync("your-session-id");
await Task.Delay(5000);  // Critical!
```

### Getting Your SSID

1. Go to [PocketOption](https://pocketoption.com)
2. Open Developer Tools (F12)
3. Go to Application/Storage → Cookies
4. Find the cookie named `ssid`
5. Copy its value

### Supported Assets

Common assets include:

- `EURUSD_otc` - Euro/US Dollar (OTC)
- `GBPUSD_otc` - British Pound/US Dollar (OTC)
- `USDJPY_otc` - US Dollar/Japanese Yen (OTC)
- `AUDUSD_otc` - Australian Dollar/US Dollar (OTC)

Use `_otc` suffix for over-the-counter (24/7 available) assets.

## 📚 Additional Resources

- **Full Documentation**: [https://chipadevteam.github.io/BinaryOptionsTools-v2/](https://chipadevteam.github.io/BinaryOptionsTools-v2/)
- **Discord Community**: [Join us](https://discord.gg/p7YyFqSmAz)

## ⚠️ Risk Warning

Trading binary options involves substantial risk and may result in the loss of all invested capital. This library is provided for educational purposes only. Always trade responsibly and never invest more than you can afford to lose.
