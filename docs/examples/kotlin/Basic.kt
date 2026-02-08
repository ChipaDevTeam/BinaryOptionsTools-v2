// Basic example
import binary_options_tools_uni.*
import kotlinx.coroutines.*

suspend fun main() {
    val client = PocketOption.new("your-session-id")
    delay(5000) // Wait for connection
    
    val balance = client.balance()
    println("Current Balance: $$balance")
}
