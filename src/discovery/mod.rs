pub mod types;
pub mod store;

use crate::addr::{NodeAddr, Protocol};
use crate::discovery::store::DiscoveryStore;
use crate::discovery::types::{NodeAdvertisement, NodeId};
use crate::node_config::NodeRole;
use crate::protocol::{MsgType, TunnelMessage};
use crate::route::Route;
use crate::wire::{build_handshake_packet, RoutingInfo};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub const MAX_DISCOVERY_RESPONSE_ADS: usize = 32;
pub const MAX_DISCOVERY_BYTES: usize = 4096;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryAdvertisePayload {
    pub advertisement: NodeAdvertisement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryQueryPayload {
    pub max_results: u8,
    #[serde(default)]
    pub role: Option<NodeRole>,
    #[serde(default)]
    pub protocol: Option<Protocol>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryResponsePayload {
    pub advertisements: Vec<NodeAdvertisement>,
}

#[derive(Debug, Clone)]
pub enum DiscoveryMessage {
    Advertise(NodeAdvertisement),
    Query(DiscoveryQueryPayload),
    Response(DiscoveryResponsePayload),
}

pub fn generate_node_id() -> NodeId {
    let mut id = [0u8; 32];
    OsRng.fill_bytes(&mut id);
    id
}

/// Deterministic node id for the lifetime of the bind address.
/// This keeps `node_id` stable across restarts for the same UDP endpoint.
pub fn node_id_from_addr(addr: &NodeAddr) -> NodeId {
    use sha2::Digest;
    let mut hasher = sha2::Sha256::new();
    hasher.update(format!("{:?}|{}|{}", addr.protocol, addr.ip, addr.port).as_bytes());
    let out = hasher.finalize();
    let mut id = [0u8; 32];
    id.copy_from_slice(&out[..]);
    id
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn make_self_advertisement(
    node_id: NodeId,
    role: NodeRole,
    addr: NodeAddr,
    version: String,
    ttl_ms: u64,
) -> NodeAdvertisement {
    NodeAdvertisement {
        node_id,
        role,
        addr,
        version,
        advertised_at_ms: now_ms(),
        ttl_ms,
    }
}

pub fn new_store(max_entries: usize) -> DiscoveryStore {
    DiscoveryStore::new(max_entries)
}

pub fn parse_discovery_message(msg: &TunnelMessage) -> Option<DiscoveryMessage> {
    if msg.payload.len() > MAX_DISCOVERY_BYTES {
        return None;
    }
    match msg.header.msg_type {
        MsgType::DiscoveryAdvertise => {
            let p: DiscoveryAdvertisePayload = bincode::deserialize(&msg.payload).ok()?;
            Some(DiscoveryMessage::Advertise(p.advertisement))
        }
        MsgType::DiscoveryQuery => {
            let p: DiscoveryQueryPayload = bincode::deserialize(&msg.payload).ok()?;
            Some(DiscoveryMessage::Query(p))
        }
        MsgType::DiscoveryResponse => {
            let p: DiscoveryResponsePayload = bincode::deserialize(&msg.payload).ok()?;
            if p.advertisements.len() > MAX_DISCOVERY_RESPONSE_ADS {
                return None;
            }
            Some(DiscoveryMessage::Response(p))
        }
        _ => None,
    }
}

pub fn clamp_query_max_results(max_results: u8) -> usize {
    if max_results == 0 {
        return 0;
    }
    (max_results as usize).min(MAX_DISCOVERY_RESPONSE_ADS)
}

pub fn build_discovery_response_payload(
    store: &DiscoveryStore,
    query: &DiscoveryQueryPayload,
    now_ms: u64,
) -> DiscoveryResponsePayload {
    let role = query.role;
    let protocol = query.protocol;
    let max = clamp_query_max_results(query.max_results);
    let ads = store.fresh_candidates(now_ms, protocol, role, max);
    DiscoveryResponsePayload { advertisements: ads }
}

/// Conservative helper for later integration: returns fresh local candidates
/// from the discovery cache, bounded and role/protocol filtered.
pub fn fresh_discovery_candidates(
    store: &DiscoveryStore,
    now_ms: u64,
    protocol: Option<Protocol>,
    role: Option<NodeRole>,
    max: usize,
) -> Vec<NodeAdvertisement> {
    store.fresh_candidates(now_ms, protocol, role, max)
}

/// Build a best-effort plaintext discovery packet that can be sent via UDP
/// directly to `to` (without establishing an AEAD session).
pub fn build_plaintext_packet(to: &NodeAddr, msg: &TunnelMessage) -> anyhow::Result<Vec<u8>> {
    let inner = crate::protocol::encode(msg)?;
    let routing = RoutingInfo {
        hop_index: 0,
        route: Route { hops: vec![to.clone()] },
    };
    build_handshake_packet(&routing, inner)
}


