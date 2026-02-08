// Example showing how to check trade results
using System;
using System.Threading.Tasks;
using BinaryOptionsToolsUni;

class CheckWinExample
{
    static async Task Main(string[] args)
    {
        try
        {
            // Initialize client
            var client = await PocketOption.NewAsync("your-session-id");
            
            // IMPORTANT: Wait for connection to establish
            await Task.Delay(5000);
            
            // Place a buy trade
            Console.WriteLine("Placing trade...");
            var deal = await client.BuyAsync("EURUSD_otc", 60, 1.0);
            var tradeId = deal.Id; // Extract trade ID from deal
            Console.WriteLine($"Trade placed with ID: {tradeId}");
            
            // Wait for trade to complete
            Console.WriteLine("\nWaiting for trade to complete (65 seconds)...");
            await Task.Delay(65000);
            
            // Check the result
            var result = await client.CheckWinAsync(tradeId);
            Console.WriteLine("\n=== Trade Result ===");
            Console.WriteLine(result);
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Error: {ex.Message}");
        }
    }
}
