// Example showing how to place trades and log trading activity
using System;
using System.Threading.Tasks;
using BinaryOptionsToolsUni;

class LogsExample
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
            Console.WriteLine($"Initial Balance: ${balanceBefore:F2}");
            
            // Place a buy (call) trade
            Console.WriteLine("\n--- Opening Buy Trade ---");
            Console.WriteLine("Asset: EURUSD_otc, Duration: 60s, Amount: $1.0");
            var buyDeal = await client.BuyAsync("EURUSD_otc", 60, 1.0);
            Console.WriteLine($"Buy deal ID: {buyDeal.Id}");
            Console.WriteLine($"Buy deal open time: {buyDeal.OpenTime}");
            
            // Place a sell (put) trade
            Console.WriteLine("\n--- Opening Sell Trade ---");
            Console.WriteLine("Asset: EURUSD_otc, Duration: 60s, Amount: $1.0");
            var sellDeal = await client.SellAsync("EURUSD_otc", 60, 1.0);
            Console.WriteLine($"Sell deal ID: {sellDeal.Id}");
            Console.WriteLine($"Sell deal open time: {sellDeal.OpenTime}");
            
            // Log all opened deals
            Console.WriteLine("\n--- Current Opened Deals ---");
            var openedDeals = await client.GetOpenedDealsAsync();
            foreach (var deal in openedDeals)
            {
                Console.WriteLine($"Deal {deal.Id}: Asset={deal.Asset}, Amount=${deal.Amount}, Profit={deal.Profit}");
            }
            
            Console.WriteLine("\nLogs example completed successfully!");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Error: {ex.Message}");
        }
    }
}
