// Example showing how to get candle history for an asset
using System;
using System.Threading.Tasks;
using BinaryOptionsToolsUni;

class HistoryExample
{
    static async Task Main(string[] args)
    {
        try
        {
            // Initialize client
            var client = await PocketOption.NewAsync("your-session-id");
            
            // IMPORTANT: Wait for connection to establish
            await Task.Delay(5000);
            
            // Get candle history for EURUSD (60-second candles)
            Console.WriteLine("Fetching candle history for EURUSD_otc...");
            var candles = await client.HistoryAsync("EURUSD_otc", 60);
            
            Console.WriteLine($"\nReceived {candles.Count} candles:");
            Console.WriteLine("====================================");
            
            // Display the first few candles
            int count = Math.Min(candles.Count, 5);
            for (int i = 0; i < count; i++)
            {
                var candle = candles[i];
                Console.WriteLine($"Candle #{i + 1}:");
                Console.WriteLine($"  Time: {candle.Time}");
                Console.WriteLine($"  Open: {candle.Open}");
                Console.WriteLine($"  High: {candle.High}");
                Console.WriteLine($"  Low: {candle.Low}");
                Console.WriteLine($"  Close: {candle.Close}");
                Console.WriteLine();
            }
            
            if (candles.Count > 5)
                Console.WriteLine($"... and {candles.Count - 5} more candles\n");
            
            Console.WriteLine("History example completed successfully!");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Error: {ex.Message}");
        }
    }
}
