// Reads the account balance.
//
//   node balance.js "<ssid>"

const { PocketOption } = require("../../nodejs");

async function main(ssid) {
  // The constructor returns immediately and connects in the background;
  // balance() waits for that connection before asking the server.
  const api = new PocketOption(ssid);

  const balance = await api.balance();
  console.log(`Balance: ${balance}`);

  await api.shutdown();
}

const ssid = process.argv[2] || process.env.POCKET_OPTION_SSID;
main(ssid).catch(console.error);
