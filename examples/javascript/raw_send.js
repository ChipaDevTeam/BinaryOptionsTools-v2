// Sends raw WebSocket messages without waiting for a response.
//
//   node raw_send.js "<ssid>"

const { PocketOption } = require("../../nodejs");

async function main(ssid) {
  const api = await PocketOption.create(ssid);

  await api.sendRaw('42["signals/subscribe"]');
  console.log("Sent signals subscription message");

  await api.sendRaw('42["price/subscribe"]');
  console.log("Sent price subscription message");

  const messages = [
    '42["chart/subscribe",{"asset":"EURUSD"}]',
    '42["trades/subscribe"]',
    '42["notifications/subscribe"]',
  ];

  for (const message of messages) {
    await api.sendRaw(message);
    console.log(`Sent message: ${message}`);
    await new Promise((resolve) => setTimeout(resolve, 1000));
  }

  await api.shutdown();
}

const ssid = process.argv[2] || process.env.POCKET_OPTION_SSID;
main(ssid).catch(console.error);
