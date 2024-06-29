use anyhow::Context;
use nix::sys::socket::{setsockopt, sockopt::Priority};
use std::{net::SocketAddr, os::fd::AsFd};
use tokio::net::UdpSocket;

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

    let src_addr = loop {
        let (sz, from_addr) = socket.recv_from(&mut buf).await?;

        if sz == 0 {
            return Ok(());
        }

        if from_addr != dst_addr {
            socket.send_to(&buf[0..sz], dst_addr).await?;
            break from_addr;
        }
    };

    loop {
        let (sz, from_addr) = socket.recv_from(&mut buf).await?;
        if sz == 0 {
            break;
        }

        let buf = &buf[0..sz];

        if from_addr == dst_addr {
            socket.send_to(buf, src_addr).await?;
        } else if from_addr == src_addr {
            socket.send_to(buf, dst_addr).await?;
        }
    }

    Ok(())
}
