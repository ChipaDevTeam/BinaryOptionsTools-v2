// Example showing how to place a buy trade
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
	
	balanceBefore, _ := client.Balance()
	fmt.Printf("Balance before trade: $%.2f\n", balanceBefore)
	
	deal, err := client.Buy("EURUSD_otc", 60, 1.0)
	if err != nil {
		panic(err)
	}
	fmt.Printf("\nTrade placed successfully!\nDeal data: %+v\n", deal)
	
	fmt.Println("\nWaiting for trade to complete (65 seconds)...")
	time.Sleep(65 * time.Second)
	
	balanceAfter, _ := client.Balance()
	fmt.Printf("Balance after trade: $%.2f\n", balanceAfter)
	fmt.Printf("Profit/Loss: $%.2f\n", balanceAfter-balanceBefore)
}
