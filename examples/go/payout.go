// Example showing how to get payout information for assets
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

	// Get full payout information for all active assets
	assets, err := client.ActiveAssets()
	if err != nil {
		panic(err)
	}
	fmt.Println("=== Full Payout (All Active Assets) ===")
	for _, asset := range assets {
		fmt.Printf("%s: %d%% payout\n", asset.Symbol, asset.Payout)
	}

	// Get payout for specific assets
	fmt.Println("\n=== Partial Payout (Selected Assets) ===")
	selectedAssets := []string{"EURUSD_otc", "EURUSD", "AEX25"}
	for _, asset := range selectedAssets {
		payout := client.Payout(asset)
		if payout != nil {
			fmt.Printf("%s: %.0f%% payout\n", asset, *payout*100)
		} else {
			fmt.Printf("%s: no payout info available\n", asset)
		}
	}

	// Get single asset payout
	fmt.Println("\n=== Single Payout ===")
	singlePayout := client.Payout("EURUSD_otc")
	if singlePayout != nil {
		fmt.Printf("EURUSD_otc: %.0f%%\n", *singlePayout*100)
	} else {
		fmt.Println("EURUSD_otc: no payout info available")
	}
}
