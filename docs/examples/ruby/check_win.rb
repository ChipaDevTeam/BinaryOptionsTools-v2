# Check trade result example
require_relative 'binary_options_tools_uni'

client = BinaryOptionsToolsUni::PocketOption.new("your-session-id")
sleep 5

deal = client.buy("EURUSD_otc", 60, 1.0)
puts "Trade placed with ID: #{deal.id}"

puts "Waiting for trade to complete..."
sleep 65

result = client.check_win(deal.id)
puts "Trade result: #{result}"
