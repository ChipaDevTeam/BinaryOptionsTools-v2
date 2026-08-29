// Fetches candles for several offsets and timeframes.
//
//   node get_candles.js "<ssid>"

const { PocketOption } = require("../../nodejs");

async function main(ssid) {
  const api = await PocketOption.create(ssid);

  // How far back to look, in seconds, and the duration of each candle.
  const offsets = Array.from({ length: 10 }, (_, i) => 3600 * (i + 1));
  const timeFrames = [1, 5, 15, 30, 60, 300];

  for (const offset of offsets) {
    for (const frame of timeFrames) {
      const candles = await api.getCandles("EURUSD_otc", frame, offset);
      console.log(`Offset ${offset}s, timeframe ${frame}s: ${candles.length} candles`);
    }
  }

  await api.shutdown();
}

const ssid = process.argv[2] || process.env.POCKET_OPTION_SSID;
main(ssid).catch(console.error);
