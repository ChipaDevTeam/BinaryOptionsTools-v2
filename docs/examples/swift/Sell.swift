// Sell trade example
import BinaryOptionsToolsUni

func sellTrade() async throws {
    let client = try await PocketOption(ssid: "your-session-id")
    try await Task.sleep(nanoseconds: 5_000_000_000)
    
    let balanceBefore = try await client.balance()
    print("Balance before: $\(balanceBefore)")
    
    let deal = try await client.sell(asset: "EURUSD_otc", time: 60, amount: 1.0)
    print("Trade placed: \(deal)")
    
    try await Task.sleep(nanoseconds: 65_000_000_000)
    
    let balanceAfter = try await client.balance()
    print("Balance after: $\(balanceAfter)")
    print("Profit/Loss: $\(balanceAfter - balanceBefore)")
}

Task { try await sellTrade() }
