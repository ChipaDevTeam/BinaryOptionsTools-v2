// Get balance example
import BinaryOptionsToolsUni

func getBalance() async throws {
    let client = try await PocketOption(ssid: "your-session-id")
    try await Task.sleep(nanoseconds: 5_000_000_000)
    
    let balance = try await client.balance()
    print("Your current balance is: $\(balance)")
}

Task { try await getBalance() }
