// Basic example showing how to initialize the client and get balance
using System;
using System.Threading.Tasks;
using BinaryOptionsToolsUni;

class BasicExample
{
    static async Task Main(string[] args)
    {
        try
        {
            // Initialize client with your session ID
            var client = await PocketOption.NewAsync("your-session-id");
            
            // IMPORTANT: Wait for connection to establish
            await Task.Delay(5000);
            
            // Get account balance
            var balance = await client.BalanceAsync();
            Console.WriteLine($"Current Balance: ${balance}");
            
            Console.WriteLine("Basic example completed successfully!");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Error: {ex.Message}");
        }
    }
}
