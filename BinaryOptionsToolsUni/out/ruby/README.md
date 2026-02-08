# BinaryOptionsTools - Ruby Bindings

Ruby bindings for BinaryOptionsTools, providing access to PocketOption trading platform via UniFFI.

## 🚀 Features

- ✅ **Trading Operations**: Place buy/sell trades
- ✅ **Account Management**: Get balance and account information
- ✅ **Real-time Data**: Subscribe to asset price feeds
- ✅ **Trade Monitoring**: Check trade results and opened deals

## 📦 Installation

```bash
# Add the binary_options_tools_uni.rb file to your project
# Make sure the native library is accessible
```

## 🔧 Quick Start

### Basic Example

```ruby
require_relative 'binary_options_tools_uni'

# Initialize client with your session ID
client = BinaryOptionsToolsUni::PocketOption.new("your-session-id")

# IMPORTANT: Wait for connection to establish
sleep 5

# Get account balance
balance = client.balance
puts "Account Balance: $#{balance}"

# Place a buy trade
deal = client.buy("EURUSD_otc", 60, 1.0)
puts "Trade placed: #{deal}"
```

## 📖 Detailed Examples

### Buy Trade Example

```ruby
require_relative 'binary_options_tools_uni'

# Initialize client
client = BinaryOptionsToolsUni::PocketOption.new("your-session-id")
sleep 5  # Wait for connection

# Place a buy trade on EURUSD for 60 seconds with $1
deal = client.buy(
  asset: "EURUSD_otc",
  time: 60,
  amount: 1.0
)

puts "Trade placed successfully!"
puts "Deal data: #{deal}"
```

### Sell Trade Example

```ruby
require_relative 'binary_options_tools_uni'

# Initialize client
client = BinaryOptionsToolsUni::PocketOption.new("your-session-id")
sleep 5  # Wait for connection

# Place a sell trade on EURUSD for 60 seconds with $1
deal = client.sell(
  asset: "EURUSD_otc",
  time: 60,
  amount: 1.0
)

puts "Trade placed successfully!"
puts "Deal data: #{deal}"
```

### Check Balance Example

```ruby
require_relative 'binary_options_tools_uni'

# Initialize client
client = BinaryOptionsToolsUni::PocketOption.new("your-session-id")
sleep 5  # Wait for connection

# Get current balance
balance = client.balance
puts "Your current balance is: $#{balance}"
```

### Check Trade Result Example

```ruby
require_relative 'binary_options_tools_uni'

# Initialize client
client = BinaryOptionsToolsUni::PocketOption.new("your-session-id")
sleep 5  # Wait for connection

# Place a trade
deal = client.buy("EURUSD_otc", 60, 1.0)
trade_id = deal.id  # Extract trade ID from deal

# Wait for trade to complete
sleep 65

# Check the result
result = client.check_win(trade_id)
puts "Trade result: #{result}"
```

### Subscribe to Real-time Data

```ruby
require_relative 'binary_options_tools_uni'

# Initialize client
client = BinaryOptionsToolsUni::PocketOption.new("your-session-id")
sleep 5  # Wait for connection

# Subscribe to real-time candle data for EURUSD
# Duration in seconds for each candle
subscription = client.subscribe("EURUSD_otc", 60)

puts "Listening for real-time candles..."
# Process subscription stream
```

## 🔑 Important Notes

### Connection Initialization

**Always wait 5 seconds after creating the client** to allow the WebSocket connection to establish:

```ruby
client = BinaryOptionsToolsUni::PocketOption.new("your-session-id")
sleep 5  # Critical!
```

### Getting Your SSID

1. Go to [PocketOption](https://pocketoption.com)
2. Open Developer Tools (F12)
3. Go to Application/Storage → Cookies
4. Find the cookie named `ssid`
5. Copy its value

### Supported Assets

Common assets include:

- `EURUSD_otc` - Euro/US Dollar (OTC)
- `GBPUSD_otc` - British Pound/US Dollar (OTC)
- `USDJPY_otc` - US Dollar/Japanese Yen (OTC)
- `AUDUSD_otc` - Australian Dollar/US Dollar (OTC)

Use `_otc` suffix for over-the-counter (24/7 available) assets.

## 📚 Additional Resources

- **Full Documentation**: [https://chipadevteam.github.io/BinaryOptionsTools-v2/](https://chipadevteam.github.io/BinaryOptionsTools-v2/)
- **Discord Community**: [Join us](https://discord.gg/p7YyFqSmAz)

## ⚠️ Risk Warning

Trading binary options involves substantial risk and may result in the loss of all invested capital. This library is provided for educational purposes only. Always trade responsibly and never invest more than you can afford to lose.
