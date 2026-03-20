use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Protocol {
    Udp,
    Quic,
    Ws,
    Tls,
    Vless,
    Wireguard,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeAddr {
    pub ip: IpAddr,
    pub port: u16,
    pub protocol: Protocol,
}

impl NodeAddr {
    pub fn as_socket_addr(&self) -> Option<SocketAddr> {
        match self.protocol {
            Protocol::Udp => Some(SocketAddr::new(self.ip, self.port)),
            _ => None,
        }
    }
}

impl fmt::Display for NodeAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}://{}:{}", self.protocol, self.ip, self.port)
    }
}

impl From<SocketAddr> for NodeAddr {
    fn from(value: SocketAddr) -> Self {
        Self {
            ip: value.ip(),
            port: value.port(),
            protocol: Protocol::Udp,
        }
    }
}

impl TryFrom<NodeAddr> for SocketAddr {
    type Error = anyhow::Error;

    fn try_from(value: NodeAddr) -> Result<Self, Self::Error> {
        value
            .as_socket_addr()
            .ok_or_else(|| anyhow::anyhow!("NodeAddr protocol is not UDP"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nodeaddr_from_socketaddr_defaults_udp() {
        let sa: SocketAddr = "127.0.0.1:1234".parse().unwrap();
        let na = NodeAddr::from(sa);
        assert_eq!(na.protocol, Protocol::Udp);
        assert_eq!(na.as_socket_addr(), Some(sa));
    }
}

