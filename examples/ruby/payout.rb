# Get payout information example
require_relative 'binary_options_tools_uni'

client = BinaryOptionsToolsUni::PocketOption.new("your-session-id")
sleep 5

# Get full payout for all assets
full_payout = client.payout
puts "Full Payout: #{full_payout}"

# Get payout for specific assets
partial_payout = client.payout(["EURUSD_otc", "EURUSD", "AEX25"])
puts "Partial Payout: #{partial_payout}"

# Get payout for a single asset
single_payout = client.payout("EURUSD_otc")
puts "Single Payout: #{single_payout}"
