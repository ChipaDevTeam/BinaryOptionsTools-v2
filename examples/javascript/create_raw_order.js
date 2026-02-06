const { PocketOption } = require("./binary-options-tools.node");
const { Validator } = require("./binary-options-tools.node");

async function main(ssid) {
  // Initialize the API client
  const api = new PocketOption(ssid);

  // Wait for connection to establish
  await new Promise((resolve) => setTimeout(resolve, 5000));

  // Basic raw order example
  try {
    // Create a validator for successful responses
    const basicValidator = Validator.contains('"status":"success"');

    // Create a raw handler with the validator
    const rawHandler = await api.create_raw_handler(basicValidator, null);

    // Send a message using the raw handler
    const basicResponse = await rawHandler.send_and_wait(
      '42["signals/subscribe"]',
    );
    console.log(`Basic raw order response: ${basicResponse}`);
  } catch (error) {
    console.log(`Basic raw order failed: ${error}`);
  }

  // Raw order with timeout example
  try {
    // Create a validator for signal data
    const timeoutValidator = Validator.regex(
      /{\"type\":\"signal\",\"data\":.*}/,
    );

    // Create a raw handler with the validator
    const rawHandler = await api.create_raw_handler(timeoutValidator, null);

    // Send a message with timeout
    const timeoutResponse = await rawHandler.send_and_wait_with_timeout(
      '42["signals/load"]',
      5000, // 5 seconds
    );
    console.log(`Raw order with timeout response: ${timeoutResponse}`);
  } catch (error) {
    if (error.name === "TimeoutError") {
      console.log("Order timed out after 5 seconds");
    } else {
      console.log(`Order with timeout failed: ${error}`);
    }
  }

  // Raw order with keep-alive message example
  try {
    // Create a validator for trade completion
    const keepAliveValidator = Validator.all([
      Validator.contains('"type":"trade"'),
      Validator.contains('"status":"completed"'),
    ]);

    // Create a keep-alive message
    const keepAliveMessage = '42["ping"]';

    // Create a raw handler with the validator and keep-alive message
    const rawHandler = await api.create_raw_handler(
      keepAliveValidator,
      keepAliveMessage,
    );

    // Send a message using the raw handler
    const keepAliveResponse = await rawHandler.send_and_wait(
      '42["trade/subscribe"]',
    );
    console.log(`Raw order with keep-alive response: ${keepAliveResponse}`);
  } catch (error) {
    console.log(`Order with keep-alive failed: ${error}`);
  }
}

// Check if ssid is provided as command line argument
const ssid = "";

main(ssid).catch(console.error);
