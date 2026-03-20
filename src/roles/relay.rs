use anyhow::Result;
use tracing::{error, info};

use crate::addr::NodeAddr;
use crate::ops::drain;
use crate::transport::{Transport, UdpTransport};
use crate::wire::{build_routed_packet, parse_routing_header};

pub type RelayArgs = crate::config::RelayConfigCli;

pub async fn run_relay(args: RelayArgs) -> Result<()> {
    let self_addr = args.listen;
    let transport = UdpTransport::bind(self_addr).await?;
    info!(addr = %self_addr, "relay listening (hop-by-hop)");
    info!(role = "relay", addr = %self_addr, "node ready");

    // For the first relay in the path, previous hop toward the client is not in
    // Route.hops; we remember it here. This is per-process, single-session MVP.
    let mut client_addr: Option<NodeAddr> = None;

    loop {
        if drain::deadline_reached() {
            info!("[drain] shutdown complete");
            break;
        }
        let (from, data) = transport.recv().await?;

        let (mut routing, inner) = match parse_routing_header(&data) {
            Ok(v) => v,
            Err(e) => {
                error!(%e, "relay: failed to parse routing header, dropping packet");
                continue;
            }
        };

        let hops = &routing.route.hops;
        if hops.is_empty() {
            error!("relay: empty route in routing header, dropping");
            continue;
        }
        // Position of this relay in the route, if any.
        // When bound to 0.0.0.0, match by port only (the port is unique per relay).
        let idx_opt = hops.iter().position(|a| {
            a.as_socket_addr().map_or(false, |sa| {
                if self_addr.ip().is_unspecified() {
                    sa.port() == self_addr.port()
                } else {
                    sa == self_addr
                }
            })
        });
        let Some(idx) = idx_opt else {
            error!(self_addr = %self_addr, "relay: self address not present in route, dropping");
            continue;
        };

        // Downstream hop (toward exit) if any.
        let downstream = if idx + 1 < hops.len() {
            Some(hops[idx + 1].clone())
        } else {
            None
        };

        // Packet from downstream → reverse toward previous hop (or client).
        let from_downstream = downstream
            .as_ref()
            .and_then(|d| d.as_socket_addr())
            .map_or(false, |d| Some(d) == from.as_socket_addr());

        if from_downstream {
            if idx == 0 {
                // First relay: previous hop is the client, not in Route.hops.
                if let Some(ref caddr) = client_addr {
                    routing.hop_index = 0;
                    info!(
                        from = %from,
                        to = %caddr,
                        idx,
                        "relay reverse path (first hop) forwarding to client at idx={}",
                        idx
                    );
                    let out = match build_routed_packet(&routing, inner) {
                        Ok(o) => o,
                        Err(e) => {
                            error!(%e, "relay: failed to rebuild reverse packet to client");
                            continue;
                        }
                    };
                    if let Err(e) = transport.send(caddr, &out).await {
                        error!(%e, "relay failed to send reverse packet to client");
                    }
                } else {
                    info!("relay got reverse packet but has no recorded client addr yet");
                }
            } else {
                // Interior relay: previous hop is another relay in the route.
                let prev = hops[idx - 1].clone();
                routing.hop_index = (idx - 1) as u8;
                info!(
                    from = %from,
                    to = %prev,
                    idx,
                    "relay reverse path forwarding to previous hop at idx={}",
                    idx - 1
                );
                let out = match build_routed_packet(&routing, inner) {
                    Ok(o) => o,
                    Err(e) => {
                        error!(%e, "relay: failed to rebuild reverse packet to previous hop");
                        continue;
                    }
                };
                if let Err(e) = transport.send(&prev, &out).await {
                    error!(%e, "relay failed to send reverse packet to previous hop");
                }
            }
        } else {
            // Forward direction: from client or upstream relay toward exit.
            if idx == 0 {
                // Remember client addr for reverse path on first relay.
                if drain::is_draining() && client_addr.is_none() {
                    info!("[drain] rejecting new sessions");
                    continue;
                }
                if let Some(ref existing) = client_addr {
                    if drain::is_draining() && existing != &from {
                        info!("[drain] rejecting new sessions");
                        continue;
                    }
                }
                client_addr = Some(from.clone());
            }
            if let Some(next) = downstream {
                routing.hop_index = (idx + 1) as u8;
                info!(
                    from = %from,
                    to = %next,
                    idx,
                    "relay forward path forwarding from idx={} to downstream",
                    idx + 1
                );
                let out = match build_routed_packet(&routing, inner) {
                    Ok(o) => o,
                    Err(e) => {
                        error!(%e, "relay: failed to rebuild forward packet");
                        continue;
                    }
                };
                if let Err(e) = transport.send(&next, &out).await {
                    error!(%e, "relay failed to send forward packet");
                }
            } else {
                error!(
                    from = %from,
                    idx,
                    route_len = hops.len(),
                    "relay: forward packet at last hop (no downstream), dropping"
                );
            }
        }
    }
    Ok(())
}

