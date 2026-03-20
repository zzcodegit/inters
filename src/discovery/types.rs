use crate::addr::NodeAddr;
use crate::node_config::NodeRole;
use serde::{Deserialize, Serialize};

pub type NodeId = [u8; 32];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeAdvertisement {
    pub node_id: NodeId,
    pub role: NodeRole,
    pub addr: NodeAddr,
    pub version: String,
    pub advertised_at_ms: u64,
    pub ttl_ms: u64,
}

impl NodeAdvertisement {
    pub fn expires_at_ms(&self) -> u64 {
        self.advertised_at_ms.saturating_add(self.ttl_ms)
    }

    pub fn is_fresh(&self, now_ms: u64) -> bool {
        now_ms <= self.expires_at_ms()
    }
}

