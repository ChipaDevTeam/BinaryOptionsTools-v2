import asyncio
from BinaryOptionsToolsV2.pocketoption import PocketOptionAsync
import time

async def main(ssid: str):
    async with PocketOptionAsync(ssid) as api:
        asset = "EURUSD_otc"
        amount = 1.0
        timeframe = 60
        
        # 1. Get current server time and price to set up a pending order
        # Subscribe to ensure we get updateStream messages for server time sync
        print(f"Subscribing to {asset} for server time sync...")
        # We use subscribe_symbol which is the correct method name in the Python wrapper
        await api.subscribe_symbol(asset)
        
        # Wait a bit for server time to sync from websocket messages
        print("Waiting for server time to sync...")
        server_time = 0
        for i in range(10):
            await asyncio.sleep(1)
            server_time = await api.get_server_time()
            if server_time > 1000000:
                print(f"Server time synced after {i+1} seconds.")
                break
            else:
                print(f"Still waiting... current server_time: {server_time}")
        
        print(f"Current server time: {server_time}")
        
        # Get candles to estimate current price
        candles = await api.get_candles(asset, 60, 60) # Last 60 seconds
        if candles:
            # Prices are often returned as strings in JSON
            try:
                current_price = float(candles[-1]['close'])
                print(f"Current estimated price for {asset}: {current_price}")
            except (ValueError, KeyError, TypeError):
                print(f"Error parsing candle price. Using fallback.")
                current_price = 1.0850
        else:
            print(f"Could not get candles for {asset}. Using a dummy price for testing.")
            current_price = 1.0850
            
        # 2. Open a pending order by price (open_type 1)
        # Parameters: open_type, amount, asset, open_time, open_price, timeframe, min_payout, command
        pending_price = round(current_price - 0.0010, 5) # Far enough to not trigger immediately
        
        print(f"Opening pending order at price: {pending_price}")
        try:
            pending_order_price = await api.open_pending_order(
                open_type=1,         # 1: By Price
                amount=amount,
                asset=asset,
                open_time="0",          # String "0" because we are using price
                open_price=pending_price,
                timeframe=timeframe,
                min_payout=0,
                command=0            # 0: Buy/Call
            )
            print(f"Pending order by price created: {pending_order_price}")
            
            # 3. Open a pending order by time (open_type 0)
            # If server_time is too low (e.g. 1), use local time as a fallback for the test
            actual_time = server_time if server_time > 1000000 else int(time.time())
            pending_timestamp = actual_time + 300
            
            # Convert timestamp to string format: "YYYY-MM-DD HH:MM:SS"
            # PocketOption uses UTC for server time strings
            pending_time_str = time.strftime('%Y-%m-%d %H:%M:%S', time.gmtime(pending_timestamp))
            
            print(f"Opening pending order at time: {pending_time_str} (in 5 minutes)")
            pending_order_time = await api.open_pending_order(
                open_type=0,         # 0: By Time
                amount=amount,
                asset=asset,
                open_time=pending_time_str,
                open_price=0,          # 0 because we are using time
                timeframe=timeframe,
                min_payout=0,
                command=1            # 1: Sell/Put
            )
            print(f"Pending order by time created: {pending_order_time}")
            
            # 4. List pending orders and cancel them
            pending_deals = await api.get_pending_deals()
            print(f"Current pending deals: {pending_deals}")
            
            tickets = []
            if isinstance(pending_deals, list):
                for deal in pending_deals:
                    ticket = deal.get('ticket') or deal.get('id')
                    if ticket:
                        tickets.append(str(ticket))
            
            if tickets:
                print(f"Found {len(tickets)} pending orders. Canceling them in batch...")
                cancel_result = await api.cancel_pending_orders(tickets)
                print(f"Batch cancel result: {cancel_result}")
                
                # 5. Verify they are gone
                pending_deals_after = await api.get_pending_deals()
                print(f"Pending deals after cancellation: {pending_deals_after}")
            else:
                print("No pending orders found to cancel.")
                
        except Exception as e:
            print(f"Error during pending order test: {e}")
            import traceback
            traceback.print_exc()

if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    asyncio.run(main(ssid))
