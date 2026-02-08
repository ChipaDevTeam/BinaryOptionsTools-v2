// Check trade result example
import binary_options_tools_uni.*
import kotlinx.coroutines.*

suspend fun main() {
    val client = PocketOption.new("your-session-id")
    delay(5000)
    
    val deal = client.buy("EURUSD_otc", 60u, 1.0)
    println("Trade placed with ID: ${deal.id}")
    
    println("Waiting for trade to complete...")
    delay(65000)
    
    val result = client.checkWin(deal.id)
    println("Trade result: $result")
}
