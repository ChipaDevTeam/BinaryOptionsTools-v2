// Example showing how to get account balance
using System;
using System.Threading.Tasks;
using BinaryOptionsToolsUni;

class BalanceExample
{
    static async Task Main(string[] args)
    {
        try
        {
            // Initialize client
            var client = await PocketOption.NewAsync("your-session-id");
            
            // IMPORTANT: Wait for connection to establish
            await Task.Delay(5000);
            
            // Get current balance
            var balance = await client.BalanceAsync();
            Console.WriteLine($"Your current balance is: ${balance:F2}");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Error: {ex.Message}");
        }
    }
}
