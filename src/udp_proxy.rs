use anyhow::Context;
use nix::sys::socket::{setsockopt, sockopt::Priority};
use std::{net::SocketAddr, os::fd::AsFd};
use tokio::net::UdpSocket;
use tracing::info;

const BUF_SIZE: usize = 65535;

pub async fn udp_proxy(
    bind_addr: SocketAddr,
    dst_addr: SocketAddr,
    priority: u8,
) -> anyhow::Result<()> {
    let priority = priority as i32;

    let socket = UdpSocket::bind(bind_addr).await?;
    setsockopt(&socket.as_fd(), Priority, &priority)
        .with_context(|| format!("setsockopt() failed on socket connecting to '{dst_addr}'"))?;

    let mut buf = [0; BUF_SIZE];

    let send = |addr: SocketAddr, sz: usize| {
        let socket = &socket;
        async move {
            socket
                .send_to(&buf[0..sz], addr)
                .await
                .with_context(|| format!("failed to send to {addr}"))
        }
    };
    let recv = || {
        let socket = &socket;
        async move {
            let (sz, addr) = socket
                .recv_from(&mut buf)
                .await
                .with_context(|| format!("failed to receive data on {bind_addr}"))?;

            anyhow::Ok((sz, addr))
        }
    };

    // Wait for the first connection
    let src_addr = loop {
        let (sz, from_addr) = recv().await?;

        if sz == 0 {
            return Ok(());
        }

        if from_addr != dst_addr {
            send(dst_addr, sz).await?;
            break from_addr;
        }
    };
    info!("established a UDP proxy {src_addr} <-> {dst_addr}");

    // Start forwarding
    loop {
        let (sz, from_addr) = recv().await?;
        if sz == 0 {
            break;
        }

        if from_addr == dst_addr {
            send(src_addr, sz).await?;
        } else if from_addr == src_addr {
            send(dst_addr, sz).await?;
        }
    }

    Ok(())
}
