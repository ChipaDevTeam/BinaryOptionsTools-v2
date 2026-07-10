const { PocketOption } = require("./binary-options-tools.node");

async function main(ssid) {
  // Initialize the API client
  const api = new PocketOption(ssid);

  // Wait for connection to establish
  await new Promise((resolve) => setTimeout(resolve, 5000));

  // Get balance before trade
  const balanceBefore = await api.balance();
  console.log(`Balance before trade: $${balanceBefore.toFixed(2)}`);

  // Place a sell trade
  const [orderId, deal] = await api.sell("EURUSD_otc", 1.0, 60);
  console.log(`\nTrade placed successfully!`);
  console.log(`Deal ID: ${orderId}`);
  console.log(`Deal data:`, deal);

  // Wait for trade to complete
  console.log("\nWaiting for trade to complete (65 seconds)...");
  await new Promise((resolve) => setTimeout(resolve, 65000));

  // Get balance after trade
  const balanceAfter = await api.balance();
  console.log(`\nBalance after trade: $${balanceAfter.toFixed(2)}`);
  console.log(`Profit/Loss: $${(balanceAfter - balanceBefore).toFixed(2)}`);
}

// Check if ssid is provided as command line argument
const ssid = "";

main(ssid).catch(console.error);
