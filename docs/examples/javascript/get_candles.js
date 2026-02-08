const { PocketOption } = require("./binary-options-tools.node");

async function main(ssid) {
  // Initialize the API client
  const api = new PocketOption(ssid);

  // Wait for connection to establish
  await new Promise((resolve) => setTimeout(resolve, 5000));

  // Define time ranges and frames
  const times = Array.from({ length: 10 }, (_, i) => 3600 * (i + 1));
  const timeFrames = [1, 5, 15, 30, 60, 300];

  // Get candles for each combination
  for (const time of times) {
    for (const frame of timeFrames) {
      const candles = await api.get_candles("EURUSD_otc", 60, time);
      console.log(`Candles for time ${time} and frame ${frame}:`, candles);
    }
  }
}

// Check if ssid is provided as command line argument
const ssid = "";

main(ssid).catch(console.error);
