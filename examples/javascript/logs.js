// Writes the library logs to disk and to the terminal while trading.
//
//   node logs.js "<ssid>"

const { PocketOption, startLogs } = require("../../nodejs");

async function main(ssid) {
  // Creates logs.log and error.log in the given directory. Set terminal to
  // false to only write the files.
  startLogs({ path: ".", level: "DEBUG", terminal: true });

  const api = await PocketOption.create(ssid);

  const [buyId] = await api.buy("EURUSD_otc", 1.0, 300);
  const [sellId] = await api.sell("EURUSD_otc", 1.0, 300);
  console.log(buyId, sellId);

  const buyResult = await api.result(buyId);
  const sellResult = await api.result(sellId);

  console.log("Buy trade profit:", buyResult.profit);
  console.log("Sell trade profit:", sellResult.profit);

  await api.shutdown();
}

const ssid = process.argv[2] || process.env.POCKET_OPTION_SSID;
main(ssid).catch(console.error);
