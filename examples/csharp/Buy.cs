// Example showing how to place a buy trade
using System;
using System.Threading.Tasks;
using BinaryOptionsToolsUni;

class BuyExample
{
    static async Task Main(string[] args)
    {
        try
        {
            // Initialize client
            var client = await PocketOption.NewAsync("your-session-id");
            
            // IMPORTANT: Wait for connection to establish
            await Task.Delay(5000);
            
            // Get initial balance
            var balanceBefore = await client.BalanceAsync();
            Console.WriteLine($"Balance before trade: ${balanceBefore:F2}");
            
            // Place a buy trade on EURUSD for 60 seconds with $1
            Console.WriteLine("\nPlacing buy trade...");
            var deal = await client.BuyAsync("EURUSD_otc", 60, 1.0);
            Console.WriteLine("Trade placed successfully!");
            Console.WriteLine($"Deal data: {deal}");
            
            // Wait for trade to complete
            Console.WriteLine("\nWaiting for trade to complete (65 seconds)...");
            await Task.Delay(65000);
            
            // Get final balance
            var balanceAfter = await client.BalanceAsync();
            Console.WriteLine($"Balance after trade: ${balanceAfter:F2}");
            Console.WriteLine($"Profit/Loss: ${balanceAfter - balanceBefore:F2}");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Error: {ex.Message}");
        }
    }
}
