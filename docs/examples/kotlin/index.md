---
sidebar_position: 6
slug: /examples/kotlin
---

# Kotlin Examples for BinaryOptionsTools

Example Kotlin programs demonstrating UniFFI bindings usage with coroutines.

## Prerequisites

- Kotlin 1.8+
- Gradle or Maven
- UniFFI bindings

## Getting Your SSID

Visit [PocketOption](https://pocketoption.com), open DevTools, find `ssid` cookie.

## Running Examples

### Gradle
```bash
./gradlew run --args="your-ssid"
```

### Maven
```bash
mvn exec:java -Dexec.args="your-ssid"
```

## Examples

- `Basic.kt` - Initialize and get balance
- `Balance.kt` - Get account balance
- `Buy.kt` - Place buy trade
- `Sell.kt` - Place sell trade
- `CheckWin.kt` - Check trade results
- `Subscribe.kt` - Subscribe to real-time data

## Important

Always wait 2 seconds after initialization:

```kotlin
val client = PocketOption.init("your-ssid")
delay(2000)  // Critical!
```