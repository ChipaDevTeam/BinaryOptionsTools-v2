// Sell trade example
import binary_options_tools_uni.*
import kotlinx.coroutines.*

suspend fun main() {
    val client = PocketOption.new("your-session-id")
    delay(5000)
    
    val balanceBefore = client.balance()
    println("Balance before: $$balanceBefore")
    
    val deal = client.sell("EURUSD_otc", 60u, 1.0)
    println("Trade placed: $deal")
    
    delay(65000)
    
    val balanceAfter = client.balance()
    println("Balance after: $$balanceAfter")
    println("Profit/Loss: $${balanceAfter - balanceBefore}")
}
