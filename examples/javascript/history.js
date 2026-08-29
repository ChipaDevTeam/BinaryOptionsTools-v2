// Fetches candle history and formats the timestamps.
//
//   node history.js "<ssid>"

const { PocketOption } = require("../../nodejs");

async function main(ssid) {
  const api = await PocketOption.create(ssid);

  const candles = await api.history("EURUSD_otc", 3600);
  console.log(`Raw candles: ${candles.length}`);

  const formatted = candles.map((candle) => ({
    time: new Date(candle.time * 1000).toISOString(),
    open: candle.open,
    high: candle.high,
    low: candle.low,
    close: candle.close,
  }));

  console.log("Formatted candles:", formatted.slice(0, 5));

  await api.shutdown();
}

const ssid = process.argv[2] || process.env.POCKET_OPTION_SSID;
main(ssid).catch(console.error);
