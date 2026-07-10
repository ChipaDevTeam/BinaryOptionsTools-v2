# Start logging, place trades, and check results example
require_relative 'binary_options_tools_uni'

client = BinaryOptionsToolsUni::PocketOption.new("your-session-id")
sleep 5

balance_before = client.balance
puts "Balance before: $#{balance_before}"

# Place a buy trade
deal = client.buy("EURUSD_otc", 300, 1.0)
puts "Buy trade placed with ID: #{deal.id}"

# Place a sell trade
sell_deal = client.sell("EURUSD_otc", 300, 1.0)
puts "Sell trade placed with ID: #{sell_deal.id}"

puts "Waiting for trades to complete..."
sleep 305

# Check results of both trades
buy_result = client.check_win(deal.id)
puts "Buy trade result: #{buy_result}"

sell_result = client.check_win(sell_deal.id)
puts "Sell trade result: #{sell_result}"

balance_after = client.balance
puts "Balance after: $#{balance_after}"
puts "Profit/Loss: $#{balance_after - balance_before}"
