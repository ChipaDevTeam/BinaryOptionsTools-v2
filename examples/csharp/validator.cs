// Example showing how to use the Validator class for filtering WebSocket messages
using System;
using System.Collections.Generic;
using System.Threading.Tasks;
using BinaryOptionsToolsUni;

class ValidatorExample
{
    static async Task Main(string[] args)
    {
        try
        {
            // Initialize client
            var client = await PocketOption.NewAsync("your-session-id");
            
            // IMPORTANT: Wait for connection to establish
            await Task.Delay(5000);
            
            Console.WriteLine("=== Validator Usage Examples ===\n");
            
            // 1. StartsWith validator - match messages beginning with a prefix
            Console.WriteLine("1. Validator.StartsWith(\"42\")");
            var startsWith = Validator.StartsWith("42");
            Console.WriteLine($"   Check '42[\"ping\"]': {startsWith.Check("42[\"ping\"]")}");
            Console.WriteLine($"   Check 'hello': {startsWith.Check("hello")}");
            Console.WriteLine();
            
            // 2. EndsWith validator - match messages ending with a suffix
            Console.WriteLine("2. Validator.EndsWith(\"}\")");
            var endsWith = Validator.EndsWith("}");
            Console.WriteLine($"   Check '{{\"key\":\"value\"}}': {endsWith.Check("{\"key\":\"value\"}")}");
            Console.WriteLine($"   Check '42[\"ping\"]': {endsWith.Check("42[\"ping\"]")}");
            Console.WriteLine();
            
            // 3. Contains validator - match messages containing a substring
            Console.WriteLine("3. Validator.Contains(\"balance\")");
            var contains = Validator.Contains("balance");
            Console.WriteLine($"   Check '42[\"balance\",{{}}]': {contains.Check("42[\"balance\",{}]")}");
            Console.WriteLine($"   Check '42[\"deals\",{{}}]': {contains.Check("42[\"deals\",{}]")}");
            Console.WriteLine();
            
            // 4. Regex validator - match messages using regular expressions
            Console.WriteLine("4. Validator.Regex(@\"^\\d+\\[\")");
            var regex = Validator.Regex(@"^\d+\[");
            Console.WriteLine($"   Check '42[\"ping\"]': {regex.Check("42[\"ping\"]")}");
            Console.WriteLine($"   Check 'hello': {regex.Check("hello")}");
            Console.WriteLine();
            
            // 5. Ne (negate) validator - invert another validator
            Console.WriteLine("5. Validator.Ne(Validator.Contains(\"error\"))");
            var notError = Validator.Ne(Validator.Contains("error"));
            Console.WriteLine($"   Check 'success': {notError.Check("success")}");
            Console.WriteLine($"   Check 'error occurred': {notError.Check("error occurred")}");
            Console.WriteLine();
            
            // 6. All validator - match when ALL sub-validators match (logical AND)
            Console.WriteLine("6. Validator.All([StartsWith(\"42\"), Contains(\"balance\")])");
            var all = Validator.All(new List<Validator> {
                Validator.StartsWith("42"),
                Validator.Contains("balance"),
            });
            Console.WriteLine($"   Check '42[\"balance\",{{}}]': {all.Check("42[\"balance\",{}]")}");
            Console.WriteLine($"   Check '42[\"deals\",{{}}]': {all.Check("42[\"deals\",{}]")}");
            Console.WriteLine($"   Check 'hello balance': {all.Check("hello balance")}");
            Console.WriteLine();
            
            // 7. Any validator - match when ANY sub-validator matches (logical OR)
            Console.WriteLine("7. Validator.Any([StartsWith(\"42\"), EndsWith(\"}\")])");
            var any = Validator.Any(new List<Validator> {
                Validator.StartsWith("42"),
                Validator.EndsWith("}"),
            });
            Console.WriteLine($"   Check '42[\"ping\"]': {any.Check("42[\"ping\"]")}");
            Console.WriteLine($"   Check '{{\"key\":\"value\"}}': {any.Check("{\"key\":\"value\"}")}");
            Console.WriteLine($"   Check 'hello': {any.Check("hello")}");
            Console.WriteLine();
            
            // 8. New validator - matches all messages (no filter)
            Console.WriteLine("8. Validator.New() (accepts all)");
            var allPass = Validator.New();
            Console.WriteLine($"   Check 'any message': {allPass.Check("any message")}");
            Console.WriteLine($"   Check '': {allPass.Check("")}");
            Console.WriteLine();
            
            // Practical use: create a raw handler with a validator
            Console.WriteLine("=== Practical: Raw Handler with Validator ===");
            var handler = await client.CreateRawHandlerAsync(
                Validator.Contains("\"balance\""),
                null
            );
            var response = await handler.SendAndWaitAsync("42[\"getBalance\"]");
            Console.WriteLine($"Balance response: {response}");
            
            Console.WriteLine("\nValidator example completed successfully!");
        }
        catch (Exception ex)
        {
            Console.WriteLine($"Error: {ex.Message}");
        }
    }
}
