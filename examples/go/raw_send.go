// Example showing how to send raw WebSocket messages
package main

import (
	"fmt"
	"time"
	"binary_options_tools_uni"
)

func main() {
	client, err := binary_options_tools_uni.NewPocketOption("your-session-id")
	if err != nil {
		panic(err)
	}
	time.Sleep(5 * time.Second)

	// Create a validator that accepts all messages
	validator := binary_options_tools_uni.NewValidator()

	// Create a raw handler with no keep-alive message
	handler, err := client.CreateRawHandler(validator, nil)
	if err != nil {
		panic(err)
	}

	// Subscribe to signals
	err = handler.SendText("42[\"signals/subscribe\"]")
	if err != nil {
		fmt.Printf("Error sending signals subscription: %v\n", err)
	}
	fmt.Println("Sent signals subscription message")

	// Subscribe to price updates
	err = handler.SendText("42[\"price/subscribe\"]")
	if err != nil {
		fmt.Printf("Error sending price subscription: %v\n", err)
	}
	fmt.Println("Sent price subscription message")

	// Custom message example
	customMessage := "42[\"custom/event\",{\"param\":\"value\"}]"
	err = handler.SendText(customMessage)
	if err != nil {
		fmt.Printf("Error sending custom message: %v\n", err)
	}
	fmt.Printf("Sent custom message: %s\n", customMessage)
}
