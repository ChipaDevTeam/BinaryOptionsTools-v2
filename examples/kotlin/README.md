# Kotlin Examples for BinaryOptionsTools

Example Kotlin programs demonstrating UniFFI bindings usage.

## Prerequisites

- Kotlin and Gradle/Maven
- UniFFI bindings
- Native library

## Getting Your SSID

Visit [PocketOption](https://pocketoption.com), open DevTools (F12), find `ssid` cookie.

## Examples

- `Basic.kt` - Initialize and get balance
- `Balance.kt` - Get account balance
- `Buy.kt` - Place buy trade
- `Sell.kt` - Place sell trade
- `CheckWin.kt` - Check trade results
- `Subscribe.kt` - Subscribe to real-time data

## Important

Always wait 5 seconds after initialization:
```kotlin
val client = PocketOption.new("your-session-id")
delay(5000)  // Critical!
```
