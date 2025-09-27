const { PocketOption } = require('./binary-options-tools.node');

async function main(ssid) {
    // Initialize the API client
    const api = new PocketOption(ssid);
    
    // Wait for connection to establish
    await new Promise(resolve => setTimeout(resolve, 5000));
    
    const [orderId, details] = await api.buy("EURUSD_otc", 10, 60);
    const results = await api.result(orderId);
    // Get balance
    console.log(`Balance: ${results.profit}`);
}

// Check if ssid is provided as command line argument
const ssid = ''

main(ssid).catch(console.error);