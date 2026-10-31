---
sidebar_position: 6
slug: /examples/kotlin
---

# Kotlin Examples for BinaryOptionsTools

Example Kotlin programs demonstrating UniFFI bindings usage with coroutines.

## Prerequisites

- Kotlin 1.8+
- Gradle or Maven
- UniFFI bindings, generated from this repo

> **Not currently generatable:** `uniffi-bindgen --language kotlin` fails with
> `Async primary constructors not supported but PocketOption has one`. Swift and
> Ruby emit successfully; Kotlin is blocked until the constructor is made sync.
> See [`crates/bindings_uniffi/README.md`](../../../crates/bindings_uniffi/README.md).

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

Expected shape once the Kotlin generator supports the async constructor — `new` is
suspending and returns only after the auth handshake, so no fixed delay is needed.

```kotlin
import kotlinx.coroutines.runBlocking

fun main() = runBlocking {
    val client = PocketOption.new("your-ssid")
    println("Balance: ${client.balance()}")  // suspend, like new()
    client.shutdown()
}
```

---

## Build strategies faster

- **✨ [ChipaEditor](https://chipaeditor.com/?utm_source=docs&utm_medium=examples-lang&utm_campaign=BinaryOptionsToolsV2)** — AI-powered algorithmic **trading strategy builder**: describe your
  edge, get a working **CHTL** strategy, backtest it on historical data, deploy it to a live broker.
  Free tier, browser and Android. _(A trading platform — not a general-purpose code editor.)_
- **📈 [ChipaX](https://exchange.chipatrade.com/trade/BTC?ref=Z1RN8GBS)** — hybrid crypto exchange: perpetuals, spot and margin, with demo mode.
- **[Chipa Ecosystem overview](/ecosystem)** — how these fit together with this library.
