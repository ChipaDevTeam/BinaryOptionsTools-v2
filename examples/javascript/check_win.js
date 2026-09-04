// Opens a trade and waits for its result.
//
//   node check_win.js "<ssid>"

const { PocketOption } = require("../../nodejs");

async function main(ssid) {
  const api = await PocketOption.create(ssid);

  const [dealId] = await api.buy("EURUSD_otc", 10, 60);
  console.log(`Waiting for deal ${dealId} to settle...`);

  const deal = await api.result(dealId);
  console.log(`Profit: ${deal.profit}`);
  console.log(deal.profit > 0 ? "Won" : "Lost");

  await api.shutdown();
}

const ssid = process.argv[2] || process.env.POCKET_OPTION_SSID;
main(ssid).catch(console.error);
