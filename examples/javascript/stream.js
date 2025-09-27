const { PocketOption } = require('./binary-options-tools.node');


async function main(ssid) {
    // Initialize the API client
    const api = new PocketOption(ssid);
    
    // Wait for connection to establish
    await new Promise(resolve => setTimeout(resolve, 5000));
    
    // Subscribe to a symbol stream
    const stream = await api.subscribe("EURUSD_otc");
    
    console.log("Starting stream...");
    
    // Listen to the stream for 1 minute
    const endTime = Date.now() + 60000; // 60 seconds
    
    try {
        for await (const data of stream) {
            console.log("Received data:", data);
            
            if (Date.now() > endTime) {
                console.log("Stream time finished");
                break;
            }
        }
    } catch (error) {
        console.error("Stream error:", error);
    } finally {
        // Clean up
        await stream.close();
    }
}

// Check if ssid is provided as command line argument
const ssid = ''
main(ssid).catch(console.error);