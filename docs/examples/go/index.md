---
sidebar_position: 7
slug: /examples/go
---

# Go Examples for BinaryOptionsTools

Example Go programs demonstrating the BinaryOptionsTools library.

## Prerequisites

- Go 1.20+
- BinaryOptionsTools Go bindings

## Getting Your SSID

Visit [PocketOption](https://pocketoption.com), open DevTools, find `ssid` cookie.

## Running Examples

```bash
go run basic.go
go run balance.go
```

## Examples

- `basic.go` - Initialize and get balance
- `balance.go` - Get account balance
- `buy.go` - Place buy trade
- `sell.go` - Place sell trade
- `check_win.go` - Check trade results
- `subscribe.go` - Subscribe to real-time data

## Important

```go
package main

import (
    "fmt"
    "time"
    bot "binaryoptionstools"
)

func main() {
    client, _ := bot.PocketOptionInit("your-ssid")
    time.Sleep(2 * time.Second)  // Critical!
    
    balance := client.Balance()
    fmt.Printf("Balance: $%.2f\n", balance)
    
    client.Shutdown()
}
```