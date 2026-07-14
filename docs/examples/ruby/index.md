---
sidebar_position: 8
slug: /examples/ruby
---

# Ruby Examples for BinaryOptionsTools

Example Ruby programs demonstrating the BinaryOptionsTools library with Async fiber support.

## Prerequisites

- Ruby 3.0+
- BinaryOptionsTools Ruby gem
- Async gem

```bash
gem install binaryoptionstoolsv2 async
```

## Getting Your SSID

Visit [PocketOption](https://pocketoption.com), open DevTools, find `ssid` cookie.

## Running Examples

```bash
ruby basic.rb
ruby balance.rb
```

## Examples

- `basic.rb` - Initialize and get balance
- `balance.rb` - Get account balance
- `buy.rb` - Place buy trade
- `sell.rb` - Place sell trade
- `check_win.rb` - Check trade results
- `subscribe.rb` - Subscribe to real-time data

## Important

```ruby
require 'async'
require 'binaryoptionstoolsv2'

Async do
  client = BinaryOptionsToolsV2::PocketOption.init('your-ssid')
  sleep 2  # Critical!

  balance = client.balance
  puts "Balance: $#{balance}"

  client.shutdown
end
```