# Basic example
require_relative 'binary_options_tools_uni'

client = BinaryOptionsToolsUni::PocketOption.new("your-session-id")
sleep 5 # Wait for connection

balance = client.balance
puts "Current Balance: $#{balance}"
