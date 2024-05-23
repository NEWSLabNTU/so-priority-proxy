use anyhow::Context;
use futures::stream::{FuturesUnordered, TryStreamExt};
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tracing::{error, info};

pub async fn tcp_proxy(src: SocketAddr, dst: SocketAddr, priority: u8) -> anyhow::Result<()> {
    let (tx, rx) = flume::bounded(4);
    let listener = TcpListener::bind(src)
        .await
        .with_context(|| format!("unable to listen to address {src}"))?;

    let accepter = async {
        loop {
            let (mut src_stream, src_addr) = listener
                .accept()
                .await
                .with_context(|| format!("accept() failed on listening address '{src}'"))?;
            info!("accepted a connection from '{src_addr}', forwarding to '{dst}'");

            let copier = async move {
                let mut dst_stream = TcpStream::connect(dst)
                    .await
                    .with_context(|| format!("unable to connect to '{dst}'"))?;

                let (mut src_rd, mut src_wr) = src_stream.split();
                let (mut dst_rd, mut dst_wr) = dst_stream.split();

                futures::try_join!(
                    tokio::io::copy(&mut src_rd, &mut dst_wr),
                    tokio::io::copy(&mut dst_rd, &mut src_wr)
                )
                .with_context(|| format!("I/O error when forwarding {src_addr} to {dst}"))?;

                anyhow::Ok(())
            };
            let Ok(()) = tx.send_async(copier).await else {
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

// async fn udp_proxy(src: SocketAddr, dst: SocketAddr, priority: u8) -> anyhow::Result<()> {
//     let socket = UdpSocket::bind(src).await?;
//     let mut buf = vec![0u8; 4096];

//     loop {
//         socket.recv(buf)
//     }

//     Ok(())
// }
