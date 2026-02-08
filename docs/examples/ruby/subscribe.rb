# Subscribe to real-time data example
require_relative 'binary_options_tools_uni'

client = BinaryOptionsToolsUni::PocketOption.new("your-session-id")
sleep 5

subscription = client.subscribe("EURUSD_otc", 60)
puts "Listening for real-time candles..."
puts "Subscription created successfully!"
