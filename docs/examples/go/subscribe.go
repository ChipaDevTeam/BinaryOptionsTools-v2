// Example showing how to subscribe to real-time candle data
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
	
	subscription, err := client.Subscribe("EURUSD_otc", 60)
	if err != nil {
		panic(err)
	}
	
	fmt.Println("Listening for real-time candles...")
	fmt.Println("Press Ctrl+C to stop\n")
	
	// Process subscription stream
	_ = subscription
	fmt.Println("Subscription created successfully!")
}
