// Streams real-time candles for a symbol.
//
//   node subscribe_symbol.js "<ssid>"

const { PocketOption } = require("../../nodejs");

async function main(ssid) {
  const api = await PocketOption.create(ssid);

  // Without the second argument every update is yielded as it arrives; with
  // it the updates are aggregated into candles of that many seconds.
  const stream = await api.subscribe("EURUSD_otc", 60);
  console.log("Listening for real-time candles...");

  let received = 0;
  for await (const candle of stream) {
    console.log(candle);
    if (++received >= 5) break;
  }

  await api.unsubscribe("EURUSD_otc");
  await api.shutdown();
}

const ssid = process.argv[2] || process.env.POCKET_OPTION_SSID;
main(ssid).catch(console.error);
