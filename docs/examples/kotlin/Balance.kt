// Get balance example
import binary_options_tools_uni.*
import kotlinx.coroutines.*

suspend fun main() {
    val client = PocketOption.new("your-session-id")
    delay(5000)
    
    val balance = client.balance()
    println("Your current balance is: $$balance")
}
