---
sidebar_position: 5
slug: /examples/swift
---

# Swift Examples for BinaryOptionsTools

Example Swift programs for iOS/macOS demonstrating UniFFI bindings usage.

## Prerequisites

- Xcode and Swift
- UniFFI bindings
- Native library

## Getting Your SSID

Visit [PocketOption](https://pocketoption.com), open DevTools, find `ssid` cookie.

## Running Examples

Add files to your Xcode project and run, or use Swift Package Manager:

```bash
swift Basic.swift
swift Balance.swift
```

## Examples

- `Basic.swift` - Initialize and get balance
- `Balance.swift` - Get account balance
- `Buy.swift` - Place buy trade
- `Sell.swift` - Place sell trade
- `CheckWin.swift` - Check trade results
- `Subscribe.swift` - Subscribe to real-time data

## Important

The initializer awaits the auth handshake, so no fixed delay is needed:

```swift
let client = try await PocketOption(ssid: "your-session-id")
let balance = try await client.balance()
print("Balance: \(balance)")
```

## SwiftUI Integration

Generate the Swift sources with `uniffi-bindgen ... --language swift --out-dir out/swift` (see [`crates/bindings_uniffi/README.md`](../../../crates/bindings_uniffi/README.md)).

---

## Build strategies faster

- **✨ [ChipaEditor](https://chipaeditor.com/?utm_source=docs&utm_medium=examples-lang&utm_campaign=BinaryOptionsToolsV2)** — AI-powered algorithmic **trading strategy builder**: describe your
  edge, get a working **CHTL** strategy, backtest it on historical data, deploy it to a live broker.
  Free tier, browser and Android. _(A trading platform — not a general-purpose code editor.)_
- **📈 [ChipaX](https://exchange.chipatrade.com/trade/BTC?ref=Z1RN8GBS)** — hybrid crypto exchange: perpetuals, spot and margin, with demo mode.
- **[Chipa Ecosystem overview](/ecosystem)** — how these fit together with this library.
