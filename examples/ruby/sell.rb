# Sell trade example
require_relative 'binary_options_tools_uni'

client = BinaryOptionsToolsUni::PocketOption.new("your-session-id")
sleep 5

balance_before = client.balance
puts "Balance before: $#{balance_before}"

deal = client.sell("EURUSD_otc", 60, 1.0)
puts "Trade placed: #{deal}"

sleep 65

balance_after = client.balance
puts "Balance after: $#{balance_after}"
puts "Profit/Loss: $#{balance_after - balance_before}"
