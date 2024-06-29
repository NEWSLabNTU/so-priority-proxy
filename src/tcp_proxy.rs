use anyhow::Context;
use futures::stream::{FuturesUnordered, TryStreamExt};
use nix::sys::socket::{setsockopt, sockopt::Priority};
use std::{net::SocketAddr, os::fd::AsFd};
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info};

pub async fn tcp_proxy(
    bind_addr: SocketAddr,
    dst_addr: SocketAddr,
    priority: u8,
) -> anyhow::Result<()> {
    let (tx, rx) = flume::bounded(4);
    let listener = TcpListener::bind(bind_addr)
        .await
        .with_context(|| format!("unable to bind address {bind_addr}"))?;

    let accepter = async {
        loop {
            let (src_stream, src_addr) = listener
                .accept()
                .await
                .with_context(|| format!("accept() failed on address '{bind_addr}'"))?;
            info!("accepted a connection from {src_addr}");

            let forwarder = run_forwarding(src_addr, dst_addr, src_stream, priority);
            let Ok(()) = tx.send_async(forwarder).await else {
                break;
            };
        }

        anyhow::Ok(())
    };

    let waiter = async {
        let mut futures = FuturesUnordered::new();

        loop {
            if futures.is_empty() {
                let Ok(copier) = rx.recv_async().await else {
                    break;
                };
                futures.push(copier);
            } else {
                tokio::select! {
                    result = rx.recv_async() => {
                        let Ok(copier) = result else {
                            break;
                        };
                        futures.push(copier);
                    }
                    result = futures.try_next() => {
                        if let Err(err) = result {
                            error!("{err}");
                        }
                    }
                }
            }
        }

        loop {
            match futures.try_next().await {
                Ok(Some(())) => {}
                Ok(None) => break,
                Err(err) => {
                    error!("{err}");
                }
            }
        }

        anyhow::Ok(())
    };

    futures::try_join!(accepter, waiter)?;

    Ok(())
}

async fn run_forwarding(
    src_addr: SocketAddr,
    dst_addr: SocketAddr,
    mut src_stream: TcpStream,
    priority: u8,
) -> anyhow::Result<()> {
    let priority = priority as i32;

    let mut dst_stream = TcpStream::connect(dst_addr)
        .await
        .with_context(|| format!("unable to connect to {dst_addr}"))?;
    info!("established a TCP proxy {src_addr} <-> {dst_addr}");

    setsockopt(&dst_stream.as_fd(), Priority, &priority)
        .with_context(|| format!("setsockopt() failed on socket connecting to {dst_addr}"))?;

    let (mut src_rd, mut src_wr) = src_stream.split();
    let (mut dst_rd, mut dst_wr) = dst_stream.split();

    tokio::select! {
        result = tokio::io::copy(&mut src_rd, &mut dst_wr) => {
            result.with_context(|| format!("I/O error when forwarding {src_addr} to {dst_addr}"))?;
        }
        result = tokio::io::copy(&mut dst_rd, &mut src_wr) => {
            result.with_context(|| format!("I/O error when forwarding {dst_addr} to {src_addr}"))?;
        }
    }

    Ok(())
}
