// Example showing how to subscribe to real-time candle data
using System;
using System.Threading.Tasks;
using BinaryOptionsToolsUni;

class SubscribeExample
{
    static async Task Main(string[] args)
    {
        try
        {
            // Initialize client
            var client = await PocketOption.NewAsync("your-session-id");
            
            // IMPORTANT: Wait for connection to establish
            await Task.Delay(5000);
            
            // Subscribe to real-time candle data for EURUSD
            Console.WriteLine("Subscribing to real-time candles...");
            var subscription = await client.SubscribeAsync("EURUSD_otc", 60);
            
            Console.WriteLine("Listening for real-time candles...");
            Console.WriteLine("Press Ctrl+C to stop\n");
            
            // Process subscription stream
            // Note: The exact API for consuming the subscription stream
            // depends on the UniFFI binding implementation
            Console.WriteLine("Subscription created successfully!");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Error: {ex.Message}");
        }
    }
}
