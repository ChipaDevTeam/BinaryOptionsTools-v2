// Basic example
import BinaryOptionsToolsUni

Task {
    let client = try await PocketOption(ssid: "your-session-id")
    try await Task.sleep(nanoseconds: 5_000_000_000)
    
    let balance = try await client.balance()
    print("Current Balance: $\(balance)")
}
