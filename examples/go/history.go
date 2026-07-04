// Example showing how to get candle history for an asset
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

	// Get candle history for EURUSD_otc with 3600 second (1 hour) candles
	candles, err := client.History("EURUSD_otc", 3600)
	if err != nil {
		panic(err)
	}
	fmt.Printf("Number of candles: %d\n", len(candles))
	for _, candle := range candles {
		fmt.Printf("Symbol: %s, Time: %d, Open: %.5f, High: %.5f, Low: %.5f, Close: %.5f, Volume: %v\n",
			candle.Symbol, candle.Timestamp, candle.Open, candle.High, candle.Low, candle.Close, candle.Volume)
	}
}
