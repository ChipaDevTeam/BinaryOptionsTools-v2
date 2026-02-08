// Subscribe to real-time data example
import binary_options_tools_uni.*
import kotlinx.coroutines.*

suspend fun main() {
    val client = PocketOption.new("your-session-id")
    delay(5000)
    
    val subscription = client.subscribe("EURUSD_otc", 60u)
    println("Listening for real-time candles...")
    println("Subscription created successfully!")
}
