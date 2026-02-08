// Subscribe to real-time data example
import BinaryOptionsToolsUni

func subscribe() async throws {
    let client = try await PocketOption(ssid: "your-session-id")
    try await Task.sleep(nanoseconds: 5_000_000_000)
    
    let subscription = try await client.subscribe(asset: "EURUSD_otc", durationSecs: 60)
    print("Listening for real-time candles...")
    print("Subscription created successfully!")
}

Task { try await subscribe() }
