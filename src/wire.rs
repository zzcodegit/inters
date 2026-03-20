use crate::protocol::{PROTOCOL_VERSION, TunnelMessage};
use crate::route::Route;
use crate::session::SessionCrypto;
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

/// Routing metadata carried in the outer header for hop-by-hop forwarding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingInfo {
    pub hop_index: u8,
    pub route: Route,
}

/// Build a wire packet for an encrypted tunnel message (DATA / OpenStream / etc.).
///
/// The caller provides full routing info; relays can read/mutate only this
/// metadata and never decrypt or parse the inner payload.
pub fn build_encrypted_packet(
    crypto: &SessionCrypto,
    routing: &RoutingInfo,
    msg: TunnelMessage,
) -> Result<Vec<u8>> {
    let inner = crypto.seal_message(msg)?;
    build_routed_packet(routing, &inner)
}

/// Build a wire packet for a cleartext handshake message.
///
/// Used only before SessionCrypto is established (HandshakeInit /
/// HandshakeChallenge / HandshakeAck path).
pub fn build_handshake_packet(routing: &RoutingInfo, plaintext: Vec<u8>) -> Result<Vec<u8>> {
    build_routed_packet(routing, &plaintext)
}

/// Build a wire packet from routing metadata and inner payload bytes.
pub fn build_routed_packet(routing: &RoutingInfo, inner: &[u8]) -> Result<Vec<u8>> {
    let routing_bytes = bincode::serialize(routing)?;
    if routing_bytes.len() > u16::MAX as usize {
        bail!("routing header too large");
    }
    let len = routing_bytes.len() as u16;
    let mut out = Vec::with_capacity(1 + 2 + routing_bytes.len() + inner.len());
    out.push(PROTOCOL_VERSION);
    out.extend_from_slice(&len.to_be_bytes());
    out.extend_from_slice(&routing_bytes);
    out.extend_from_slice(inner);
    Ok(out)
}

/// Parse outer routing header and return (RoutingInfo, inner_payload_slice).
pub fn parse_routing_header(data: &[u8]) -> Result<(RoutingInfo, &[u8])> {
    if data.len() < 3 {
        bail!("wire packet too short for routing header");
    }
    let version = data[0];
    if version != PROTOCOL_VERSION {
        bail!(
            "protocol version mismatch in wire header: got {}, expected {}",
            version,
            PROTOCOL_VERSION
        );
    }
    let len = u16::from_be_bytes([data[1], data[2]]) as usize;
    if data.len() < 3 + len {
        bail!("wire packet too short for routing payload");
    }
    let routing: RoutingInfo = bincode::deserialize(&data[3..3 + len])?;
    Ok((routing, &data[3 + len..]))
}

