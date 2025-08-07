+++
title = "TypeScript Example"
description = "Example of how to use the PocketOption client in TypeScript."
weight = 3
+++

# TypeScript Example

Here is a basic example of how to use the `PocketOption` client in a TypeScript application.

**Note:** The JavaScript/TypeScript wrapper (`BinaryOptionsToolsJs`) is still under development and is not yet fully functional. This example is for illustrative purposes only.

For more information, please refer to the `BinaryOptionsToolsJs` directory.

```typescript
import { PocketOption } from "binary-options-tools";

async function main() {
    // Replace with your actual session ID
    const ssid = "YOUR_SSID_HERE";

    // Create a new PocketOption client
    const client = await PocketOption.new(ssid);

    // Get the current balance
    const balance = await client.balance();
    console.log(`Current balance: ${balance}`);

    // Get the list of available assets
    const assets = await client.assets();
    console.log(`Available assets: ${assets}`);

    // Shutdown the client
    await client.shutdown();
}

main().catch(console.error);
```
