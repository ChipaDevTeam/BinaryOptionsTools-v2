//! Stress test for multiple simultaneous subscriptions on one connection.
//!
//! Usage: `multi_sub_stress <max_pairs> <tf_secs,tf_secs,...> [stagger_secs]`
//! Ramps from 1 to `max_pairs` subscriptions per timeframe, waiting for every
//! active stream to deliver a candle after each addition.

use std::time::{Duration, Instant};

use binary_options_tools::pocketoption::{candle::SubscriptionType, PocketOption};
use futures_util::StreamExt;

const PAIRS: [&str; 5] = [
    "EURUSD_otc",
    "GBPUSD_otc",
    "USDJPY_otc",
    "AUDUSD_otc",
    "EURJPY_otc",
];

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::WARN)
        .init();
    let ssid = std::env::var("POCKET_OPTION_SSID")?;
    let args: Vec<String> = std::env::args().collect();
    let max_pairs: usize = args.get(1).and_then(|a| a.parse().ok()).unwrap_or(3);
    let tfs: Vec<u64> = args
        .get(2)
        .map(|a| a.split(',').filter_map(|s| s.parse().ok()).collect())
        .unwrap_or_else(|| vec![5, 15, 30, 60, 120, 180]);
    let stagger: u64 = args.get(3).and_then(|a| a.parse().ok()).unwrap_or(5);

    for tf in tfs {
        let client = PocketOption::new(&ssid).await?;
        tokio::time::sleep(Duration::from_secs(3)).await;
        let mut streams = Vec::new();
        for n in 1..=max_pairs {
            let asset = PAIRS[n - 1];
            let t0 = Instant::now();
            let sub_type = if tf == 0 {
                SubscriptionType::none()
            } else {
                SubscriptionType::time_aligned(Duration::from_secs(tf))?
            };
            match client.subscribe(asset, sub_type).await {
                Ok(s) => streams.push((asset, s.to_stream())),
                Err(e) => {
                    println!("tf={tf}s pairs={n}: SUBSCRIBE FAIL {asset}: {e}");
                    break;
                }
            }
            println!(
                "tf={tf}s pairs={n}: subscribed {asset} in {:?}",
                t0.elapsed()
            );
            tokio::time::sleep(Duration::from_secs(stagger)).await;

            // every active stream must yield a candle
            let limit = Duration::from_secs(tf * 2 + 20);
            let mut ok = true;
            for (name, st) in streams.iter_mut() {
                let t = Instant::now();
                match tokio::time::timeout(limit, st.next()).await {
                    Ok(Some(Ok(_))) => println!("  {name}: candle after {:?}", t.elapsed()),
                    other => {
                        println!(
                            "  {name}: FAIL after {:?}: {:?}",
                            t.elapsed(),
                            other.map(|o| o.map(|r| r.map(|_| ())))
                        );
                        ok = false;
                    }
                }
            }
            if !ok {
                println!("tf={tf}s pairs={n}: FAILED");
                break;
            }
            println!(
                "tf={tf}s pairs={n}: OK (connected={})",
                client.is_connected()
            );
        }
        drop(streams);
        client.shutdown().await.ok();
    }
    Ok(())
}
