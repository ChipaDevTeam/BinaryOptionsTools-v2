// Sends raw messages and waits for the responses a validator accepts.
//
//   node create_raw_order.js "<ssid>"

const { PocketOption, Validator } = require("../../nodejs");

async function main(ssid) {
  const api = await PocketOption.create(ssid);

  // A handler keeps every message its validator accepts, so responses that
  // arrive before send_and_wait returns are not lost.
  try {
    const validator = Validator.contains('"status":"success"');
    const handler = await api.createRawHandler(validator);
    const response = await handler.sendAndWait('42["signals/subscribe"]');
    console.log(`Basic raw order response: ${response}`);
  } catch (error) {
    console.log(`Basic raw order failed: ${error.message}`);
  }

  // Validators take patterns as strings, including regular expressions.
  try {
    const validator = Validator.regex('\\{"type":"signal","data":.*\\}');
    const handler = await api.createRawHandler(validator);
    const response = await handler.sendAndWaitWithTimeout('42["signals/load"]', 5000);
    console.log(`Raw order with timeout response: ${response}`);
  } catch (error) {
    // Timeouts are reported as `TimeoutError: ...`.
    console.log(`Order with timeout failed: ${error.message}`);
  }

  // The second argument is a keep-alive message, re-sent after a reconnect.
  try {
    const validator = Validator.all([
      Validator.contains('"type":"trade"'),
      Validator.contains('"status":"completed"'),
    ]);
    const handler = await api.createRawHandler(validator, '42["ping"]');
    const response = await handler.sendAndWait('42["trade/subscribe"]');
    console.log(`Raw order with keep-alive response: ${response}`);
  } catch (error) {
    console.log(`Order with keep-alive failed: ${error.message}`);
  }

  await api.shutdown();
}

const ssid = process.argv[2] || process.env.POCKET_OPTION_SSID;
main(ssid).catch(console.error);
