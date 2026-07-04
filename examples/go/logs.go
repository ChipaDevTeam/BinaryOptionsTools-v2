// Example showing how to place trades, check results, and log activity
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

	fmt.Println("Starting trading operations...")

	// Place a buy (call) trade
	buyDeal, err := client.Buy("EURUSD_otc", 300, 1.0)
	if err != nil {
		panic(err)
	}
	fmt.Printf("Buy trade placed with ID: %s\n", buyDeal.ID)

	// Place a sell (put) trade
	sellDeal, err := client.Sell("EURUSD_otc", 300, 1.0)
	if err != nil {
		panic(err)
	}
	fmt.Printf("Sell trade placed with ID: %s\n", sellDeal.ID)

	fmt.Println("\nWaiting for trades to complete (65 seconds)...")
	time.Sleep(65 * time.Second)

	// Check results for both trades
	buyResult, err := client.CheckWin(buyDeal.ID)
	if err != nil {
		panic(err)
	}
	fmt.Printf("\nBuy trade result: %+v\n", buyResult)

	sellResult, err := client.CheckWin(sellDeal.ID)
	if err != nil {
		panic(err)
	}
	fmt.Printf("Sell trade result: %+v\n", sellResult)
}
