# Ruby Examples for BinaryOptionsTools

Example Ruby scripts demonstrating UniFFI bindings usage.

## Prerequisites

- Ruby installed
- UniFFI bindings file
- Native library

## Getting Your SSID

Visit [PocketOption](https://pocketoption.com), open DevTools (F12), find `ssid` cookie.

## Running Examples

```bash
ruby basic.rb
ruby balance.rb
ruby buy.rb
```

## Examples

- `basic.rb` - Initialize and get balance
- `balance.rb` - Get account balance
- `buy.rb` - Place buy trade
- `sell.rb` - Place sell trade
- `check_win.rb` - Check trade results
- `subscribe.rb` - Subscribe to real-time data

## Important

Always wait 5 seconds after initialization:

```ruby
client = BinaryOptionsToolsUni::PocketOption.new("your-session-id")
sleep 5  # Critical!
```
