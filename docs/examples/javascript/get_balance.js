const { PocketOption } = require("./binary-options-tools.node");

async function main(ssid) {
  // Initialize the API client
  const api = new PocketOption(ssid);

  // Wait for connection to establish
  await new Promise((resolve) => setTimeout(resolve, 5000));

  // Get balance
  const balance = await api.balance();
  console.log(`Balance: ${balance}`);
}

// Check if ssid is provided as command line argument
const ssid = "";

main(ssid).catch(console.error);
