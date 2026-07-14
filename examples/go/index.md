# Go Examples for BinaryOptionsTools

Example Go programs demonstrating how to use the BinaryOptionsTools UniFFI bindings.

## Prerequisites

- Go installed ([Download Go](https://golang.org/dl/))
- The UniFFI bindings for Go
- The native library

## Getting Your SSID

1. Go to [PocketOption](https://pocketoption.com)
2. Open Developer Tools (F12)
3. Go to Application/Storage → Cookies
4. Find the cookie named `ssid`
5. Copy its value

## Running Examples

```bash
go run basic.go
go run balance.go
go run buy.go
```

## Available Examples

- `basic.go` - Initialize client and get balance
- `balance.go` - Get account balance
- `buy.go` - Place a buy trade
- `sell.go` - Place a sell trade
- `check_win.go` - Check trade results
- `history.go` - Get candle history
- `logs.go` - Start logging and place trades
- `payout.go` - Get payout information
- `raw_send.go` - Send raw WebSocket messages
- `subscribe_symbol.go` - Subscribe to real-time data
- `validator.go` - Demonstrate validator usage

## Important: Always wait 5 seconds after creating the client!

```go
client, _ := binary_options_tools_uni.NewPocketOption("your-session-id")
time.Sleep(5 * time.Second)  // Critical!
```
