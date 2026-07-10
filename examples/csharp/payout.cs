// Example showing how to get payout information for assets
using System;
using System.Threading.Tasks;
using BinaryOptionsToolsUni;

class PayoutExample
{
    static async Task Main(string[] args)
    {
        try
        {
            // Initialize client
            var client = await PocketOption.NewAsync("your-session-id");
            
            // IMPORTANT: Wait for connection to establish
            await Task.Delay(5000);
            
            // Get payout for a specific asset
            Console.WriteLine("Fetching payout for EURUSD_otc...");
            var payout = await client.PayoutAsync("EURUSD_otc");
            if (payout.HasValue)
            {
                Console.WriteLine($"Payout for EURUSD_otc: {payout.Value * 100:F1}%");
            }
            else
            {
                Console.WriteLine("Payout info not available for EURUSD_otc");
            }
            
            // Get payout for a second asset
            Console.WriteLine("\nFetching payout for GBPUSD_otc...");
            payout = await client.PayoutAsync("GBPUSD_otc");
            if (payout.HasValue)
            {
                Console.WriteLine($"Payout for GBPUSD_otc: {payout.Value * 100:F1}%");
            }
            else
            {
                Console.WriteLine("Payout info not available for GBPUSD_otc");
            }
            
            // Iterate all available assets and display their payouts
            Console.WriteLine("\n--- All Available Assets ---");
            var assets = await client.AssetsAsync();
            if (assets != null)
            {
                foreach (var asset in assets)
                {
                    if (asset.IsActive)
                    {
                        Console.WriteLine($"{asset.Symbol}: Payout={asset.Payout}%, Active={asset.IsActive}");
                    }
                }
            }
            else
            {
                Console.WriteLine("Asset list not yet loaded. Try waiting longer.");
            }
            
            Console.WriteLine("\nPayout example completed successfully!");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Error: {ex.Message}");
        }
    }
}
