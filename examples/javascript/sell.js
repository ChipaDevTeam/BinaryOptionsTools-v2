// Places a put trade and reports the balance before and after it settles.
//
//   node sell.js "<ssid>"

const { PocketOption } = require("../../nodejs");

async function main(ssid) {
  const api = await PocketOption.create(ssid);

  const balanceBefore = await api.balance();
  console.log(`Balance before trade: $${balanceBefore.toFixed(2)}`);

  const [dealId, deal] = await api.sell("EURUSD_otc", 1.0, 60);
  console.log(`\nTrade placed successfully!`);
  console.log(`Deal ID: ${dealId}`);
  console.log("Deal data:", deal);

  const settled = await api.result(dealId);
  console.log(`\nProfit: $${settled.profit}`);

  const balanceAfter = await api.balance();
  console.log(`Balance after trade: $${balanceAfter.toFixed(2)}`);

  await api.shutdown();
}

const ssid = process.argv[2] || process.env.POCKET_OPTION_SSID;
main(ssid).catch(console.error);
