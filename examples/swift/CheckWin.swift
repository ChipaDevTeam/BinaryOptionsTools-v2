// Check trade result example
import BinaryOptionsToolsUni

func checkWin() async throws {
    let client = try await PocketOption(ssid: "your-session-id")
    try await Task.sleep(nanoseconds: 5_000_000_000)
    
    let deal = try await client.buy(asset: "EURUSD_otc", time: 60, amount: 1.0)
    print("Trade placed with ID: \(deal.id)")
    
    print("Waiting for trade to complete...")
    try await Task.sleep(nanoseconds: 65_000_000_000)
    
    let result = try await client.checkWin(tradeId: deal.id)
    print("Trade result: \(result)")
}

Task { try await checkWin() }
