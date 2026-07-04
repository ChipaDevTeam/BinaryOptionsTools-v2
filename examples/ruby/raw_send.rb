# Send raw WebSocket messages example
require_relative 'binary_options_tools_uni'

client = BinaryOptionsToolsUni::PocketOption.new("your-session-id")
sleep 5

# Subscribe to signals
client.raw_send('42["signals/subscribe"]')
puts "Sent signals subscription message"

# Subscribe to price updates
client.raw_send('42["price/subscribe"]')
puts "Sent price subscription message"

# Custom message example
custom_message = '42["custom/event",{"param":"value"}]'
client.raw_send(custom_message)
puts "Sent custom message: #{custom_message}"

# Send multiple messages in sequence
messages = [
  '42["chart/subscribe",{"asset":"EURUSD"}]',
  '42["trades/subscribe"]',
  '42["notifications/subscribe"]'
]

messages.each do |msg|
  client.raw_send(msg)
  puts "Sent message: #{msg}"
  sleep 1
end
