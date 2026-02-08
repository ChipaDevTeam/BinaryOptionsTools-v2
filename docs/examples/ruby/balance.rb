# Get balance example
require_relative 'binary_options_tools_uni'

client = BinaryOptionsToolsUni::PocketOption.new("your-session-id")
sleep 5

balance = client.balance
puts "Your current balance is: $#{balance}"
