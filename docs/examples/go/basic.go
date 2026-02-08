// Basic example showing how to initialize the client and get balance
package main

import (
	"fmt"
	"time"
	"binary_options_tools_uni"
)

func main() {
	// Initialize client with your session ID
	client, err := binary_options_tools_uni.NewPocketOption("your-session-id")
	if err != nil {
		panic(err)
	}
	
	// IMPORTANT: Wait for connection to establish
	time.Sleep(5 * time.Second)
	
	// Get account balance
	balance, err := client.Balance()
	if err != nil {
		panic(err)
	}
	fmt.Printf("Current Balance: $%.2f\n", balance)
}
