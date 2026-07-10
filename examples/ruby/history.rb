# Get candle history example
require_relative 'binary_options_tools_uni'

client = BinaryOptionsToolsUni::PocketOption.new("your-session-id")
sleep 5

# Get candle data for an asset
candles = client.get_candles("EURUSD_otc", 60, 3600)
puts "Raw Candles: #{candles}"

# Format and display individual candle data
candles.each do |candle|
  time = Time.at(candle["time"]).to_s
  puts "Time: #{time}, Open: #{candle["open"]}, High: #{candle["high"]}, Low: #{candle["low"]}, Close: #{candle["close"]}, Volume: #{candle["volume"]}"
end
