from BinaryOptionsToolsV2.pocketoption import PocketOption


# Main part of the code
def main(ssid: str):
    # Use context manager for automatic connection and cleanup
    with PocketOption(ssid) as api:
        # Get all active assets
        active_assets = api.active_assets()
        print(f"Found {len(active_assets)} active assets:")
        print("-" * 60)

        # Group by asset type for better organization
        from collections import defaultdict

        by_type = defaultdict(list)
        for asset in active_assets:
            by_type[asset["asset_type"]].append(asset)

        for asset_type, assets in sorted(by_type.items()):
            print(f"\n{asset_type.upper()} ({len(assets)}):")
            for asset in sorted(assets, key=lambda x: x["symbol"]):
                otc_marker = " (OTC)" if asset["is_otc"] else ""
                print(
                    f"  {asset['symbol']}{otc_marker}: {asset['name']} - Payout: {asset['payout']}%"
                )


if __name__ == "__main__":
    ssid = input("Please enter your ssid: ")
    main(ssid)
