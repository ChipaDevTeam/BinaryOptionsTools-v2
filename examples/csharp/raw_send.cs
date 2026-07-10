// Example showing how to send raw WebSocket messages
using System;
using System.Threading.Tasks;
using BinaryOptionsToolsUni;

class RawSendExample
{
    static async Task Main(string[] args)
    {
        try
        {
            // Initialize client
            var client = await PocketOption.NewAsync("your-session-id");
            
            // IMPORTANT: Wait for connection to establish
            await Task.Delay(5000);
            
            // Create a raw handler that accepts all messages
            Console.WriteLine("Creating raw handler (no filter)...");
            var handler = await client.CreateRawHandlerAsync(Validator.New(), null);
            
            // Send a raw text message (ping)
            Console.WriteLine("Sending ping message...");
            await handler.SendTextAsync("42[\"ping\"]");
            Console.WriteLine("Ping sent!");
            
            // Create a raw handler with a validator to filter responses
            Console.WriteLine("\nCreating raw handler with balance filter...");
            var balanceHandler = await client.CreateRawHandlerAsync(
                Validator.Contains("\"balance\""),
                null
            );
            
            // Send a raw message and wait for the matching response
            Console.WriteLine("Sending getBalance and waiting for response...");
            var response = await balanceHandler.SendAndWaitAsync("42[\"getBalance\"]");
            Console.WriteLine($"Response: {response}");
            
            // Create a raw handler for receiving candle data
            Console.WriteLine("\nCreating raw handler with keep-alive subscription...");
            var keepAliveMsg = "42[\"subscribeSymbol\",\"EURUSD_otc\"]";
            var candleHandler = await client.CreateRawHandlerAsync(
                Validator.Contains("\"candle\""),
                keepAliveMsg
            );
            
            Console.WriteLine("Waiting for next candle message (10 second timeout)...");
            var candleMsg = await candleHandler.WaitNextAsync();
            Console.WriteLine($"Candle message: {candleMsg}");
            
            Console.WriteLine("\nRaw send example completed successfully!");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Error: {ex.Message}");
        }
    }
}
