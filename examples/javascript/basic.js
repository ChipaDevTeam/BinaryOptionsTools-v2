// Smallest possible session: connect, read the balance, disconnect.
//
//   node basic.js "<ssid>"

const { PocketOption } = require("../../nodejs");

async function main(ssid) {
  // create() rejects if the connection cannot be established, unlike `new`
  // which connects in the background.
  const api = await PocketOption.create(ssid);

  console.log(`Demo account: ${await api.isDemo()}`);
  console.log(`Balance: ${await api.balance()}`);

  await api.shutdown();
}

const ssid = process.argv[2] || process.env.POCKET_OPTION_SSID;
main(ssid).catch(console.error);
