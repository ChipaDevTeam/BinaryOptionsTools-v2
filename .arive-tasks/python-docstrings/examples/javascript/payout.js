const { PocketOption } = require("./binary-options-tools.node");

async function main(ssid) {
  // Initialize the API client
  const api = new PocketOption(ssid);

  // Wait for connection to establish
  await new Promise((resolve) => setTimeout(resolve, 5000));

  // The payout method does not exist in the current API implementation
  // Please refer to the documentation for available methods
  console.log(
    "The payout method is not available in the current API implementation.",
  );
}

// Check if ssid is provided as command line argument
const ssid = "";

main(ssid).catch(console.error);
