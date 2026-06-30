// Example showing how to check trade results
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
	
	deal, err := client.Buy("EURUSD_otc", 60, 1.0)
	if err != nil {
		panic(err)
	}
	fmt.Printf("Trade placed with ID: %s\n", deal.ID)
	
	fmt.Println("\nWaiting for trade to complete (65 seconds)...")
	time.Sleep(65 * time.Second)
	
	result, err := client.CheckWin(deal.ID)
	if err != nil {
		panic(err)
	}
	fmt.Printf("\n=== Trade Result ===\n%+v\n", result)
}
