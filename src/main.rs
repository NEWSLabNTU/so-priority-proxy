mod config;
mod proxy;

use clap::Parser;
use config::{Config, Map, Protocol};
use futures::{stream::FuturesUnordered, TryStreamExt};
use proxy::tcp_proxy;
use std::{path::PathBuf, time::Duration};
use tracing::error;

#[derive(Parser)]
struct Args {
    pub config: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args = Args::parse();
    let config = Config::open(args.config)?;

    let futures: FuturesUnordered<_> = config
        .maps
        .into_iter()
        .map(|map| {
            let Map {
                protocol,
                src,
                dst,
                priority,
            } = map;

            match protocol {
                Protocol::Tcp => tokio::spawn(async move {
                    loop {
                        match tcp_proxy(src, dst, priority).await {
                            Ok(()) => break,
                            Err(err) => {
                                error!("proxy {src} -> {dst} failed: {err}\nretry in 3 seconds");
                                tokio::time::sleep(Duration::from_secs(3)).await;
                            }
                        }
                    }
                }),
                Protocol::Udp => todo!(),
            }
        })
        .collect();

    futures.try_for_each(|()| futures::future::ok(())).await?;

    Ok(())
}
