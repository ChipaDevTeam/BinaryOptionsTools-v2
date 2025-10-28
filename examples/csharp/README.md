# C# Examples for BinaryOptionsTools

This directory contains example C# programs demonstrating how to use the BinaryOptionsTools UniFFI bindings.

## Prerequisites

1. .NET SDK installed ([Download .NET](https://dotnet.microsoft.com/download))
2. The `binary_options_tools_uni.cs` file from the UniFFI bindings
3. The native library (`.dll` on Windows, `.so` on Linux, `.dylib` on macOS)

## Getting Your SSID

1. Go to [PocketOption](https://pocketoption.com)
2. Open Developer Tools (F12)
3. Go to Application/Storage → Cookies
4. Find the cookie named `ssid`
5. Copy its value and replace `"your-session-id"` in the examples

## Running the Examples

Compile and run each example using:

```bash
csc /reference:binary_options_tools_uni.dll Basic.cs
./Basic.exe
```

Or create a .csproj file and run with:
```bash
dotnet run
```

## Available Examples

### `Basic.cs`
Basic example showing:
- Client initialization
- Getting account balance

### `Balance.cs`
Simple example showing how to get your account balance.

### `Buy.cs`
Example demonstrating how to place a buy trade and check profit/loss.

### `Sell.cs`
Example demonstrating how to place a sell trade and check profit/loss.

### `CheckWin.cs`
Example showing how to check trade results after completion.

### `Subscribe.cs`
Example demonstrating how to subscribe to real-time candle data.

## Important Notes

### Connection Initialization

**Always wait 5 seconds after creating the client** to allow the WebSocket connection to establish:

```csharp
var client = await PocketOption.NewAsync("your-session-id");
await Task.Delay(5000);  // Critical!
```

### Error Handling

All examples use try-catch blocks for proper error handling. Make sure to handle errors appropriately in production code.

### Async/Await

All examples use async/await pattern. Make sure your `Main` method is marked as `async Task Main`.

## Common Assets

- `EURUSD_otc` - Euro/US Dollar (OTC)
- `GBPUSD_otc` - British Pound/US Dollar (OTC)
- `USDJPY_otc` - US Dollar/Japanese Yen (OTC)
- `AUDUSD_otc` - Australian Dollar/US Dollar (OTC)

Use `_otc` suffix for over-the-counter (24/7 available) assets.

## Additional Resources

- **Full Documentation**: [https://chipadevteam.github.io/BinaryOptionsTools-v2/](https://chipadevteam.github.io/BinaryOptionsTools-v2/)
- **Discord Community**: [Join us](https://discord.gg/p7YyFqSmAz)

## ⚠️ Risk Warning

Trading binary options involves substantial risk and may result in the loss of all invested capital. These examples are provided for educational purposes only. Always trade responsibly and never invest more than you can afford to lose.
