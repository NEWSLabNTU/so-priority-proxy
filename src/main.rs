mod config;
mod tcp_proxy;
mod udp_proxy;

use clap::Parser;
use config::{Config, Map, Protocol};
use futures::{stream::FuturesUnordered, TryStreamExt};
use std::{path::PathBuf, time::Duration};
use tcp_proxy::tcp_proxy;
use tracing::error;
use udp_proxy::udp_proxy;

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
                src: bind_addr,
                dst: dst_addr,
                priority,
            } = map;

            match protocol {
                Protocol::Tcp => tokio::spawn(async move {
                    loop {
                        match tcp_proxy(bind_addr, dst_addr, priority).await {
                            Ok(()) => break,
                            Err(err) => {
                                error!("proxy {bind_addr} -> {dst_addr} failed: {err}\nretry in 3 seconds");
                                tokio::time::sleep(Duration::from_secs(3)).await;
                            }
                        }
                    }
                }),
                Protocol::Udp => tokio::spawn(async move {
                    loop {
                        match udp_proxy(bind_addr, dst_addr, priority).await {
                            Ok(()) => break,
                            Err(err) => {
                                error!("proxy {bind_addr} -> {dst_addr} failed: {err}\nretry in 3 seconds");
                                tokio::time::sleep(Duration::from_secs(3)).await;
                            }
                        }
                    }
                }),
            }
        })
        .collect();

    futures.try_for_each(|()| futures::future::ok(())).await?;

    Ok(())
}
