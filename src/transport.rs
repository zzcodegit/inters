use anyhow::Result;
use async_trait::async_trait;
use crate::addr::{NodeAddr, Protocol};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tracing::debug;

#[async_trait]
pub trait Transport: Send + Sync {
    async fn send(&self, to: &NodeAddr, data: &[u8]) -> Result<()>;
    async fn recv(&self) -> Result<(NodeAddr, Vec<u8>)>;
    fn local_addr(&self) -> Result<NodeAddr>;
}

#[derive(Clone)]
pub struct UdpTransport {
    socket: Arc<UdpSocket>,
}

impl UdpTransport {
    pub async fn bind(addr: SocketAddr) -> Result<Self> {
        let socket = UdpSocket::bind(addr).await?;
        Ok(Self {
            socket: Arc::new(socket),
        })
    }

    pub async fn connect(addr: SocketAddr) -> Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.connect(addr).await?;
        Ok(Self {
            socket: Arc::new(socket),
        })
    }

    pub fn from_socket(socket: UdpSocket) -> Self {
        Self {
            socket: Arc::new(socket),
        }
    }
}

#[async_trait]
impl Transport for UdpTransport {
    async fn send(&self, to: &NodeAddr, data: &[u8]) -> Result<()> {
        let sa = to
            .as_socket_addr()
            .ok_or_else(|| anyhow::anyhow!("UdpTransport: non-UDP NodeAddr"))?;
        debug!(protocol = ?to.protocol, to = %to, bytes = data.len(), "transport send");
        self.socket.send_to(data, sa).await?;
        Ok(())
    }

    async fn recv(&self) -> Result<(NodeAddr, Vec<u8>)> {
        let mut buf = vec![0u8; 65535];
        let (len, addr) = self.socket.recv_from(&mut buf).await?;
        buf.truncate(len);
        let from = NodeAddr::from(addr);
        debug!(protocol = ?from.protocol, from = %from, bytes = len, "transport recv");
        Ok((from, buf))
    }

    fn local_addr(&self) -> Result<NodeAddr> {
        let sa = self.socket.local_addr()?;
        Ok(NodeAddr {
            ip: sa.ip(),
            port: sa.port(),
            protocol: Protocol::Udp,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn udp_transport_roundtrip() {
        let addr1: SocketAddr = "127.0.0.1:40001".parse().unwrap();
        let addr2: SocketAddr = "127.0.0.1:40002".parse().unwrap();

        let t1 = UdpTransport::bind(addr1).await.unwrap();
        let t2 = UdpTransport::bind(addr2).await.unwrap();

        let msg = b"hello";

        let h1 = tokio::spawn(async move {
            let (_from, data) = t1.recv().await.unwrap();
            data
        });

        t2.send(&NodeAddr::from(addr1), msg).await.unwrap();
        let received = h1.await.unwrap();
        assert_eq!(&received, msg);
    }
}

