---
sidebar_position: 9
slug: /examples/csharp
---

# C# Examples for BinaryOptionsTools

Example C# programs demonstrating the BinaryOptionsTools library with async/await.

## Prerequisites

- .NET 6+
- BinaryOptionsTools NuGet package

```bash
dotnet add package BinaryOptionsToolsV2
```

## Getting Your SSID

Visit [PocketOption](https://pocketoption.com), open DevTools, find `ssid` cookie.

## Running Examples

```bash
dotnet run --project basic.csproj
dotnet run --project balance.csproj
```

## Examples

- `Basic.cs` - Initialize and get balance
- `Balance.cs` - Get account balance
- `Buy.cs` - Place buy trade
- `Sell.cs` - Place sell trade
- `CheckWin.cs` - Check trade results
- `Subscribe.cs` - Subscribe to real-time data

## Important

```csharp
using BinaryOptionsToolsV2;

var client = await PocketOption.InitAsync("your-ssid");
await Task.Delay(2000);  // Critical!

var balance = await client.BalanceAsync();
Console.WriteLine($"Balance: ${balance}");

await client.ShutdownAsync();
```