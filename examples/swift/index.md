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
