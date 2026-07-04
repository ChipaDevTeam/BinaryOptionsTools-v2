const { PocketOption } = require("./binary-options-tools.node");

async function main(ssid) {
  // Initialize the API client
  const api = new PocketOption(ssid);

  // Wait for connection to establish
  await new Promise((resolve) => setTimeout(resolve, 5000));

  // Subscribe to real-time candle data for a symbol
  const subscription = await api.subscribe("EURUSD_otc", 60);
  console.log("Listening for real-time candles...");
  console.log("Subscription created successfully!");
}

// Check if ssid is provided as command line argument
const ssid = "";

main(ssid).catch(console.error);
