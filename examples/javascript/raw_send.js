const { PocketOption } = require('./binary-options-tools.node');


async function main(ssid) {
    // Initialize the API client
    const api = new PocketOption(ssid);
    
    // Wait for connection to establish
    await new Promise(resolve => setTimeout(resolve, 5000));
    
    try {
        // Get raw handle
        const rawHandle = await api.raw_handle();
        
        // Subscribe to signals
        await rawHandle.send('42["signals/subscribe"]');
        console.log("Sent signals subscription message");
        
        // Subscribe to price updates
        await rawHandle.send('42["price/subscribe"]');
        console.log("Sent price subscription message");
        
        // Custom message example
        const customMessage = '42["custom/event",{"param":"value"}]';
        await rawHandle.send(customMessage);
        console.log(`Sent custom message: ${customMessage}`);
        
        // Multiple messages in sequence
        const messages = [
            '42["chart/subscribe",{"asset":"EURUSD"}]',
            '42["trades/subscribe"]',
            '42["notifications/subscribe"]'
        ];
        
        for (const msg of messages) {
            await rawHandle.send(msg);
            console.log(`Sent message: ${msg}`);
            await new Promise(resolve => setTimeout(resolve, 1000)); // 1 second delay
        }
        
    } catch (error) {
        console.log(`Error sending message: ${error}`);
    }
}

// Check if ssid is provided as command line argument
const ssid = ''

main(ssid).catch(console.error);