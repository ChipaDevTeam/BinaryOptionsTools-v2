// Iterates over every raw response matching a validator.
//
//   node create_raw_iterator.js "<ssid>"

const { PocketOption, Validator } = require("../../nodejs");

async function main(ssid) {
  const api = await PocketOption.create(ssid);

  const validator = Validator.contains('"signals"');

  // Sends the message, then yields matching responses for 10 seconds.
  const stream = await api.createRawIterator('42["signals/subscribe"]', validator, 10_000);

  for await (const message of stream) {
    console.log(`Received: ${message}`);
  }

  console.log("Iterator finished");
  await api.shutdown();
}

const ssid = process.argv[2] || process.env.POCKET_OPTION_SSID;
main(ssid).catch(console.error);
