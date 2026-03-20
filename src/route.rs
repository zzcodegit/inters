//! Stage 5: Variable-length circuit routing.
//! Route = ordered list of UDP hop addresses. Client sends to first hop.
//! Last hop is exit. Relays forward without parsing.

use crate::addr::NodeAddr;
use std::net::SocketAddr;
use serde::{Serialize, Deserialize};

/// Ordered path of hops. Client sends to hops[0]. Last hop is exit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub hops: Vec<NodeAddr>,
}

impl Route {
    /// First hop address (where client sends)
    pub fn first_hop(&self) -> Option<NodeAddr> {
        self.hops.first().cloned()
    }

    /// Last hop (exit)
    pub fn last_hop(&self) -> Option<NodeAddr> {
        self.hops.last().cloned()
    }

    pub fn len(&self) -> usize {
        self.hops.len()
    }

    pub fn is_empty(&self) -> bool {
        self.hops.is_empty()
    }

    /// Next hop after given index (for forwarding). Returns None if at last hop.
    pub fn next_hop(&self, hop_index: usize) -> Option<NodeAddr> {
        self.hops.get(hop_index + 1).cloned()
    }

    /// UDP helper: first hop SocketAddr (only if protocol=UDP).
    pub fn first_hop_udp(&self) -> Option<SocketAddr> {
        self.hops.first().and_then(|h| h.as_socket_addr())
    }

    /// UDP helper: last hop SocketAddr (only if protocol=UDP).
    pub fn last_hop_udp(&self) -> Option<SocketAddr> {
        self.hops.last().and_then(|h| h.as_socket_addr())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::addr::NodeAddr;

    #[test]
    fn route_first_last_hop() {
        let r = Route {
            hops: vec![
                NodeAddr::from("127.0.0.1:100".parse::<SocketAddr>().unwrap()),
                NodeAddr::from("127.0.0.1:101".parse::<SocketAddr>().unwrap()),
                NodeAddr::from("127.0.0.1:102".parse::<SocketAddr>().unwrap()),
            ],
        };
        assert_eq!(r.first_hop_udp(), Some("127.0.0.1:100".parse().unwrap()));
        assert_eq!(r.last_hop_udp(), Some("127.0.0.1:102".parse().unwrap()));
        assert_eq!(r.len(), 3);
    }

    #[test]
    fn route_next_hop_selection() {
        let r = Route {
            hops: vec![
                NodeAddr::from("127.0.0.1:100".parse::<SocketAddr>().unwrap()),
                NodeAddr::from("127.0.0.1:101".parse::<SocketAddr>().unwrap()),
                NodeAddr::from("127.0.0.1:102".parse::<SocketAddr>().unwrap()),
            ],
        };
        assert_eq!(r.next_hop(0).unwrap().as_socket_addr(), Some("127.0.0.1:101".parse().unwrap()));
        assert_eq!(r.next_hop(1).unwrap().as_socket_addr(), Some("127.0.0.1:102".parse().unwrap()));
        assert_eq!(r.next_hop(2), None); // last hop
    }
}
