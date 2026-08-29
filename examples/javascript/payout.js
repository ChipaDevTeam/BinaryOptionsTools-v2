// Lists the payout percentage of every active asset.
//
//   node payout.js "<ssid>"

const { PocketOption } = require("../../nodejs");

async function main(ssid) {
  const api = await PocketOption.create(ssid);

  // The asset list arrives shortly after the connection is established.
  await api.waitForAssets(10);

  const payouts = await api.payout();
  for (const [asset, payout] of Object.entries(payouts)) {
    console.log(`${asset}: ${payout}%`);
  }

  await api.shutdown();
}

const ssid = process.argv[2] || process.env.POCKET_OPTION_SSID;
main(ssid).catch(console.error);
