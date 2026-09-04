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

Always wait 5 seconds after initialization:

```swift
let client = try await PocketOption(ssid: "your-session-id")
try await Task.sleep(nanoseconds: 5_000_000_000)  // Critical!
```

## SwiftUI Integration

See the Swift README in `BinaryOptionsToolsUni/out/swift/` for SwiftUI examples.


---

## Build strategies faster

- **✨ [ChipaEditor](https://chipaeditor.com/?utm_source=docs&utm_medium=examples-lang&utm_campaign=BinaryOptionsToolsV2)** — AI-powered algorithmic **trading strategy builder**: describe your
  edge, get a working **CHTL** strategy, backtest it on historical data, deploy it to a live broker.
  Free tier, browser and Android. *(A trading platform — not a general-purpose code editor.)*
- **📈 [ChipaX](https://exchange.chipatrade.com/trade/BTC?ref=Z1RN8GBS)** — hybrid crypto exchange: perpetuals, spot and margin, with demo mode.
- **[Chipa Ecosystem overview](/ecosystem)** — how these fit together with this library.
