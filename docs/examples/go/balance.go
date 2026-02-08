// Example showing how to get account balance
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
	
	balance, err := client.Balance()
	if err != nil {
		panic(err)
	}
	fmt.Printf("Your current balance is: $%.2f\n", balance)
}
