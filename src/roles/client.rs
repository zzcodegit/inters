use crate::addr::NodeAddr;
use crate::ant::{Ant, AntDedup, AntType};
use crate::build_info::BuildInfo;
use crate::config::ClientConfigCli as ClientArgs;
use crate::discovery;
use crate::discovery::DiscoveryMessage;
use crate::flow::FlowTable;
use crate::handshake::{
    build_handshake_init, build_handshake_init_with_cookie, derive_session_key_from_ack,
    encode_plaintext, HandshakeAckPayload, HandshakeChallengePayload,
};
use crate::node_config::NodeRole;
use crate::ops::drain;
use crate::packet::{parse_tcp_ports, FlowKey, Ipv4Header};
use crate::protocol::{decode, MsgType, StreamFrame, TunnelMessage, PROTOCOL_VERSION};
use crate::route::Route;
use crate::route_memory::RouteCache;
use crate::route_store::{RouteFailureKind, RouteStore};
use crate::session::SessionCrypto;
use crate::stage_trace;
use crate::stream_reliable::{AckFrame, ReliableStream};
use crate::transport::{Transport, UdpTransport};
use crate::tun::TunDevice;
use crate::wire::{build_encrypted_packet, parse_routing_header, RoutingInfo};
use anyhow::Result;
use serde_json::json;
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Arc, Mutex,
};
use std::time::Instant;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, UdpSocket};
use tokio::sync::{mpsc, Mutex as AsyncMutex};
use tokio::time::{timeout, Duration};
use tracing::{debug, error, info};

/// Minimal HTTP helper: find (header_end, content_length) if possible.
fn http_content_length(buf: &[u8]) -> Option<(usize, usize)> {
    let haystack = buf;
    let needle = b"\r\n\r\n";
    if haystack.len() < needle.len() {
        return None;
    }
    let mut hdr_end = None;
    // Inclusive upper bound: allow matching delimiter at the very end.
    let end = haystack.len().saturating_sub(needle.len());
    for i in 0..=end {
        if &haystack[i..i + needle.len()] == needle {
            hdr_end = Some(i + needle.len());
            break;
        }
    }
    let hdr_end = hdr_end?;
    let headers = &haystack[..hdr_end];
    let marker = b"Content-Length:";
    let pos = headers.windows(marker.len()).position(|w| w == marker)?;
    let rest = &headers[pos + marker.len()..];
    let rest = match rest.iter().position(|b| !b.is_ascii_whitespace()) {
        Some(idx) => &rest[idx..],
        None => return None,
    };
    let mut len: usize = 0;
    let mut any = false;
    for &b in rest {
        if b.is_ascii_digit() {
            any = true;
            len = len.saturating_mul(10).saturating_add((b - b'0') as usize);
        } else {
            break;
        }
    }
    if !any {
        return None;
    }
    Some((hdr_end, len))
}

fn http_header_end(buf: &[u8]) -> Option<usize> {
    let haystack = buf;
    let needle = b"\r\n\r\n";
    if haystack.len() < needle.len() {
        return None;
    }
    let end = haystack.len().saturating_sub(needle.len());
    for i in 0..=end {
        if &haystack[i..i + needle.len()] == needle {
            return Some(i + needle.len());
        }
    }
    None
}

fn http_status_code(buf: &[u8]) -> Option<u16> {
    // Minimal parse: find "HTTP/1.1 " then 3 digits.
    let marker = b"HTTP/1.1 ";
    let pos = buf.windows(marker.len()).position(|w| w == marker)?;
    let start = pos + marker.len();
    if buf.len() < start + 3 {
        return None;
    }
    let d = &buf[start..start + 3];
    if !d.iter().all(|b| b.is_ascii_digit()) {
        return None;
    }
    Some((d[0] - b'0') as u16 * 100 + (d[1] - b'0') as u16 * 10 + (d[2] - b'0') as u16)
}

fn strip_port_from_host_bytes(host_port: &[u8]) -> Option<&[u8]> {
    if host_port.is_empty() {
        return None;
    }
    if host_port.first() == Some(&b'[') {
        let close = host_port.iter().position(|&b| b == b']')?;
        if close <= 1 {
            return None;
        }
        return Some(&host_port[1..close]);
    }
    if let Some(idx) = host_port.iter().rposition(|&b| b == b':') {
        if idx + 1 < host_port.len() && host_port[idx + 1..].iter().all(|b| b.is_ascii_digit()) {
            return Some(&host_port[..idx]);
        }
    }
    Some(host_port)
}

fn extract_http_host_from_request<'a>(req: &'a [u8]) -> Option<&'a str> {
    let marker = b"Host:";
    let pos = req
        .windows(marker.len())
        .position(|window| window == marker)?;
    let mut idx = pos + marker.len();
    while idx < req.len() && req[idx].is_ascii_whitespace() {
        idx += 1;
    }
    if idx >= req.len() {
        return None;
    }
    let end = req[idx..]
        .iter()
        .position(|&b| b == b'\r' || b == b'\n')
        .map(|offset| idx + offset)
        .unwrap_or(req.len());
    let host_port = &req[idx..end];
    strip_port_from_host_bytes(host_port).and_then(|host| std::str::from_utf8(host).ok())
}

fn request_site_label(req: &[u8]) -> String {
    extract_http_host_from_request(req)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown")
        .to_string()
}

fn emit_client_stage(stage: &str, payload: serde_json::Value) {
    if !stage_trace::enabled() {
        return;
    }
    let mut object = stage_trace::event("client", stage);
    if let serde_json::Value::Object(fields) = payload {
        object.extend(fields);
    }
    stage_trace::emit(serde_json::Value::Object(object));
}

type ResponseTx = mpsc::UnboundedSender<Option<Vec<u8>>>;
type ResponseSenderTable = Arc<Mutex<HashMap<u32, ResponseTx>>>;
type ReliableStreamTable = Arc<Mutex<HashMap<u32, Arc<Mutex<ReliableStream>>>>>;
type StreamRouteTable = Arc<Mutex<HashMap<u32, Route>>>;
type ResponseStartedTable = Arc<Mutex<HashMap<u32, bool>>>;
type CompletedResponseStreamTable = Arc<Mutex<HashMap<u32, CompletedResponseStream>>>;

const COMPLETED_RESPONSE_STREAM_LINGER: Duration = Duration::from_secs(30);
const MAX_COMPLETED_RESPONSE_STREAMS: usize = 2048;

#[derive(Clone)]
struct CompletedResponseStream {
    reliable: Arc<Mutex<ReliableStream>>,
    route: Route,
    completed_at: Instant,
    completion_reason: &'static str,
    late_payloads: u64,
    late_close_signals: u64,
}

fn purge_completed_response_streams(map: &mut HashMap<u32, CompletedResponseStream>, now: Instant) {
    map.retain(|_, entry| {
        now.duration_since(entry.completed_at) <= COMPLETED_RESPONSE_STREAM_LINGER
    });
    if map.len() <= MAX_COMPLETED_RESPONSE_STREAMS {
        return;
    }
    let mut by_age = map
        .iter()
        .map(|(&stream_id, entry)| (stream_id, entry.completed_at))
        .collect::<Vec<_>>();
    by_age.sort_by_key(|(_, completed_at)| *completed_at);
    let remove_count = map.len().saturating_sub(MAX_COMPLETED_RESPONSE_STREAMS);
    for (stream_id, _) in by_age.into_iter().take(remove_count) {
        map.remove(&stream_id);
    }
}

fn remember_completed_response_stream(
    completed_streams: &CompletedResponseStreamTable,
    stream_id: u32,
    reliable: Arc<Mutex<ReliableStream>>,
    route: Route,
    completion_reason: &'static str,
) {
    let mut map = completed_streams.lock().unwrap();
    purge_completed_response_streams(&mut map, Instant::now());
    map.insert(
        stream_id,
        CompletedResponseStream {
            reliable,
            route,
            completed_at: Instant::now(),
            completion_reason,
            late_payloads: 0,
            late_close_signals: 0,
        },
    );
}

fn snapshot_completed_response_stream(
    completed_streams: &CompletedResponseStreamTable,
    stream_id: u32,
) -> Option<CompletedResponseStream> {
    let mut map = completed_streams.lock().unwrap();
    purge_completed_response_streams(&mut map, Instant::now());
    map.get(&stream_id).cloned()
}

fn note_completed_response_late_payload(
    completed_streams: &CompletedResponseStreamTable,
    stream_id: u32,
) -> Option<(u64, &'static str, u64)> {
    let mut map = completed_streams.lock().unwrap();
    purge_completed_response_streams(&mut map, Instant::now());
    let entry = map.get_mut(&stream_id)?;
    entry.late_payloads = entry.late_payloads.saturating_add(1);
    Some((
        entry.completed_at.elapsed().as_millis() as u64,
        entry.completion_reason,
        entry.late_payloads,
    ))
}

fn note_completed_response_close(
    completed_streams: &CompletedResponseStreamTable,
    stream_id: u32,
) -> Option<(u64, &'static str, u64)> {
    let mut map = completed_streams.lock().unwrap();
    purge_completed_response_streams(&mut map, Instant::now());
    let entry = map.get_mut(&stream_id)?;
    entry.late_close_signals = entry.late_close_signals.saturating_add(1);
    Some((
        entry.completed_at.elapsed().as_millis() as u64,
        entry.completion_reason,
        entry.late_close_signals,
    ))
}

fn build_probe_request() -> Vec<u8> {
    b"GET / HTTP/1.1\r\nHost: probe\r\nConnection: close\r\n\r\n".to_vec()
}

fn classify_anyhow_failure(e: &anyhow::Error) -> RouteFailureKind {
    // Conservative mapping (Stage 6.2): only a few stable categories.
    let mut cur: &(dyn std::error::Error + 'static) = e.as_ref();
    loop {
        if let Some(ioe) = cur.downcast_ref::<std::io::Error>() {
            use std::io::ErrorKind;
            return match ioe.kind() {
                ErrorKind::TimedOut => RouteFailureKind::Timeout,
                ErrorKind::ConnectionRefused => RouteFailureKind::Refused,
                ErrorKind::ConnectionReset => RouteFailureKind::Unreachable,
                ErrorKind::NotConnected => RouteFailureKind::Unreachable,
                _ => RouteFailureKind::Unknown,
            };
        }
        match cur.source() {
            Some(next) => cur = next,
            None => break,
        }
    }
    RouteFailureKind::Unknown
}

/// Stage 5: Build route from CLI args. Resolves all addresses.
async fn build_route(args: &ClientArgs) -> Result<Route> {
    let len = args.route_length.clamp(1, 16);
    let mut hops = Vec::with_capacity(len as usize);
    match len {
        1 => {
            let mut addrs = tokio::net::lookup_host(args.exit_addr.clone()).await?;
            let a = addrs
                .next()
                .ok_or_else(|| anyhow::anyhow!("client: could not resolve exit address"))?;
            hops.push(NodeAddr::from(a));
        }
        2 => {
            let mut r = tokio::net::lookup_host(args.relay_addr.clone()).await?;
            hops.push(NodeAddr::from(r.next().ok_or_else(|| {
                anyhow::anyhow!("client: could not resolve relay address")
            })?));
            let mut e = tokio::net::lookup_host(args.exit_addr.clone()).await?;
            hops.push(NodeAddr::from(e.next().ok_or_else(|| {
                anyhow::anyhow!("client: could not resolve exit address")
            })?));
        }
        _ => {
            let mut r1 = tokio::net::lookup_host(args.relay_addr.clone()).await?;
            hops.push(NodeAddr::from(r1.next().ok_or_else(|| {
                anyhow::anyhow!("client: could not resolve relay address")
            })?));
            let r2_addr = args
                .relay2_addr
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("client: route-length 3 requires --relay2-addr"))?;
            let mut r2 = tokio::net::lookup_host(r2_addr).await?;
            hops.push(NodeAddr::from(r2.next().ok_or_else(|| {
                anyhow::anyhow!("client: could not resolve relay2 address")
            })?));
            let mut e = tokio::net::lookup_host(args.exit_addr.clone()).await?;
            hops.push(NodeAddr::from(e.next().ok_or_else(|| {
                anyhow::anyhow!("client: could not resolve exit address")
            })?));
        }
    }
    let route = Route { hops };
    let path_str = route
        .hops
        .iter()
        .map(|a| a.to_string())
        .collect::<Vec<_>>()
        .join(" → ");
    info!(hops = route.len(), path = %path_str, "client route built");
    Ok(route)
}

/// Stage 6: Generate a set of candidate routes derived from CLI args.
/// We keep the existing `--route-length` behavior as the primary route, but
/// also register shorter alternatives for adaptive selection/fallback.
async fn build_initial_routes(args: &ClientArgs) -> Result<(Route, Vec<Route>)> {
    let primary = build_route(args).await?;
    if args.exact_route_only {
        return Ok((primary.clone(), vec![primary]));
    }
    let max_len = primary.len().max(args.route_length as usize);

    // Resolve exit/relay/relay2 once to avoid repeated DNS work.
    let mut exit_addrs = tokio::net::lookup_host(args.exit_addr.clone()).await?;
    let exit = exit_addrs
        .next()
        .ok_or_else(|| anyhow::anyhow!("client: could not resolve exit address"))?;

    let mut candidates: Vec<Route> = Vec::new();

    // 1-hop: client -> exit
    candidates.push(Route {
        hops: vec![NodeAddr::from(exit)],
    });

    // 2-hop: client -> relay -> exit
    if max_len >= 2 {
        let mut relay_addrs = tokio::net::lookup_host(args.relay_addr.clone()).await?;
        if let Some(relay1) = relay_addrs.next() {
            candidates.push(Route {
                hops: vec![NodeAddr::from(relay1), NodeAddr::from(exit)],
            });

            // 3-hop: client -> relay1 -> relay2 -> exit (if provided)
            if max_len >= 3 {
                if let Some(r2s) = args.relay2_addr.as_ref() {
                    let mut relay2_addrs = tokio::net::lookup_host(r2s).await?;
                    if let Some(relay2) = relay2_addrs.next() {
                        candidates.push(Route {
                            hops: vec![
                                NodeAddr::from(relay1),
                                NodeAddr::from(relay2),
                                NodeAddr::from(exit),
                            ],
                        });
                    }
                }
            }
        }
    }

    // Ensure primary is present (and unique set).
    let mut out: Vec<Route> = Vec::new();
    for r in candidates
        .into_iter()
        .chain(std::iter::once(primary.clone()))
    {
        if !out.iter().any(|x| x.hops == r.hops) {
            out.push(r);
        }
    }

    Ok((primary, out))
}

pub async fn run_client(args: ClientArgs) -> Result<()> {
    info!(mode = %args.mode, local_listen = %args.local_listen, relay = %args.relay_addr, "client starting");
    let (primary_route, routes) = build_initial_routes(&args).await?;
    let first_hop = primary_route
        .first_hop()
        .ok_or_else(|| anyhow::anyhow!("client: empty route"))?;

    let route_cache_path = args.route_cache_path.clone();
    let mut route_cache = RouteCache::load(&route_cache_path)?;
    route_cache.record_success(
        &args.relay_addr,
        &args.exit_addr,
        "udp",
        std::time::Duration::from_millis(10),
        std::time::Duration::from_secs(60),
    );
    route_cache.save(&route_cache_path)?;

    // On non-Unix platforms we do not support TUN mode in Stage 2.
    if args.mode == "tun" {
        #[cfg(not(unix))]
        {
            return Err(anyhow::anyhow!(
                "client tun mode is only supported on Unix-like systems (e.g. Linux) in Stage 2. \
Please run with --mode tcp on this platform, or use a Unix host for experimental TUN lab setups. \
See README: Stage 2 support matrix."
            ));
        }
    }

    // Bind a single UDP socket for the lifetime of the client process.
    // On Linux (including Docker) используем обычный async-bind Tokio.
    // На Windows увеличиваем буферы через socket2 и помним выставить nonblocking.
    #[cfg(windows)]
    let udp_socket = {
        use socket2::Socket;
        let std_sock = std::net::UdpSocket::bind("0.0.0.0:0")?;
        let sock2 = Socket::from(std_sock);
        sock2.set_recv_buffer_size(1_000_000)?;
        sock2.set_send_buffer_size(1_000_000)?;
        sock2.set_nonblocking(true)?;
        UdpSocket::from_std(sock2.into())?
    };
    #[cfg(not(windows))]
    let udp_socket = UdpSocket::bind("0.0.0.0:0").await?;

    let transport = UdpTransport::from_socket(udp_socket);

    info!(
        local = %transport.local_addr()?,
        first_hop = %first_hop,
        route_len = primary_route.len(),
        candidates = routes.len(),
        "client transport bound"
    );
    info!(role = "client", udp = %transport.local_addr()?, tcp = %args.local_listen, "node ready");

    // Stage 9.1: minimal discovery store + best-effort self advertisement (control-plane only).
    let discovery_store = Arc::new(Mutex::new(discovery::new_store(args.discovery_max_entries)));
    let discovery_self_addr = transport.local_addr()?;
    let discovery_self_adv = {
        let bi = BuildInfo::current();
        let self_id = discovery::node_id_from_addr(&discovery_self_addr);
        let ttl_ms = args.discovery_advertise_ttl_sec.saturating_mul(1000);
        discovery::make_self_advertisement(
            self_id,
            NodeRole::Client,
            discovery_self_addr.clone(),
            bi.short(),
            ttl_ms,
        )
    };
    {
        let now = crate::ant::now_ms();
        let mut store = discovery_store.lock().unwrap();
        store.purge_expired(now);
        store.insert(discovery_self_adv.clone());
    }
    debug!(
        event = "discovery_self_advertise",
        role = "client",
        addr = %discovery_self_addr,
        enabled = args.discovery_enabled,
        "self advertisement cached"
    );

    // Stage 5: handshake once per circuit; packets go through relays to exit.
    let crypto = perform_handshake(&transport, &primary_route).await?;

    // Best-effort publish/query to bootstrap peers after AEAD session is established.
    if args.discovery_enabled {
        let bootstrap_peers: Vec<NodeAddr> = match args.discovery_bootstrap_peers.clone() {
            Some(v) if !v.is_empty() => v.into_iter().map(NodeAddr::from).collect(),
            _ => primary_route.hops.clone(),
        };
        let store_for_task = discovery_store.clone();
        let udp_for_task = transport.clone();
        let self_adv_for_task = discovery_self_adv.clone();
        let query_on_start = args.discovery_query_on_start;
        tokio::spawn(async move {
            for peer in bootstrap_peers {
                if peer.as_socket_addr() == discovery_self_addr.as_socket_addr() {
                    continue;
                }

                // DiscoveryAdvertise
                let adv_payload = discovery::DiscoveryAdvertisePayload {
                    advertisement: self_adv_for_task.clone(),
                };
                if let Ok(payload_bytes) = bincode::serialize(&adv_payload) {
                    let msg =
                        TunnelMessage::new(MsgType::DiscoveryAdvertise, 1, 0, 0, payload_bytes);
                    if let Ok(packet) = discovery::build_plaintext_packet(&peer, &msg) {
                        let _ = udp_for_task.send(&peer, &packet).await;
                    }
                    debug!(
                        event = "discovery_advertise_sent",
                        peer = %peer,
                        "sent self discovery advertisement"
                    );
                }

                // DiscoveryQuery
                if query_on_start {
                    let query = discovery::DiscoveryQueryPayload {
                        max_results: 16,
                        role: None,
                        protocol: Some(crate::addr::Protocol::Udp),
                    };
                    if let Ok(payload_bytes) = bincode::serialize(&query) {
                        let qmsg =
                            TunnelMessage::new(MsgType::DiscoveryQuery, 1, 0, 0, payload_bytes);
                        if let Ok(packet) = discovery::build_plaintext_packet(&peer, &qmsg) {
                            if udp_for_task.send(&peer, &packet).await.is_ok() {
                                let mut store = store_for_task.lock().unwrap();
                                store.on_query_sent();
                            }
                        }
                    }
                    debug!(
                        event = "discovery_query",
                        peer = %peer,
                        max_results = 16,
                        "sent discovery query"
                    );
                }
            }
        });
    }

    if args.mode == "tcp" {
        // Channel: Some(chunk) = response data, None = end-of-response.
        let response_senders: ResponseSenderTable = Arc::new(Mutex::new(HashMap::new()));
        // Per-stream reliable state.
        let reliable_streams: ReliableStreamTable = Arc::new(Mutex::new(HashMap::new()));
        // Track which streams have received at least one response chunk (optional).
        let response_started: ResponseStartedTable = Arc::new(Mutex::new(HashMap::new()));
        // Stage 6: track per-stream route for ACKs/retransmits.
        let stream_routes_for_io: StreamRouteTable = Arc::new(Mutex::new(HashMap::new()));
        let completed_response_streams: CompletedResponseStreamTable =
            Arc::new(Mutex::new(HashMap::new()));

        // Stage 7: ant dedup + optional measurement agents.
        let ants_enabled = std::env::var("VPNNODE_ANTS")
            .map(|v| v == "1")
            .unwrap_or(false);
        let ant_dedup: Arc<Mutex<AntDedup>> =
            Arc::new(Mutex::new(AntDedup::new(256, Duration::from_secs(30))));

        // Stage 6/7: adaptive route selection store (in-memory), also used by ants.
        let mut store = RouteStore::new();
        for r in routes {
            let initial = if r.hops == primary_route.hops {
                1.0
            } else {
                0.6
            };
            store.add_route(r, initial);
        }
        let route_store = Arc::new(AsyncMutex::new(store));

        // Task: receive from relay/exit over UDP and dispatch to TCP streams
        let udp_recv = transport.clone();
        let crypto_recv = crypto.clone();
        let response_senders_recv = response_senders.clone();
        let reliable_streams_recv = reliable_streams.clone();
        let response_started_recv = response_started.clone();
        let udp_send_for_ack = transport.clone();
        let udp_send_discovery = transport.clone();
        let discovery_store_recv = discovery_store.clone();
        let crypto_send_for_ack = crypto.clone();
        let stream_routes_for_ack = stream_routes_for_io.clone();
        let completed_response_streams_recv = completed_response_streams.clone();
        let default_route_for_ack = primary_route.clone();
        let route_store_for_ants = route_store.clone();
        let ant_dedup_recv = ant_dedup.clone();
        tokio::spawn(async move {
            loop {
                let Ok((from, data)) = udp_recv.recv().await else {
                    break;
                };
                info!(
                    protocol = ?from.protocol,
                    from = %from,
                    len = data.len(),
                    "client received via transport (tcp mode)"
                );

                let (routing, inner) = match parse_routing_header(&data) {
                    Ok(v) => v,
                    Err(e) => {
                        error!(%e, "client failed to parse routing header (tcp mode)");
                        continue;
                    }
                };
                let _hop_index = routing.hop_index;

                let msg = match crypto_recv.open_message(inner) {
                    Ok(m) => m,
                    Err(e) => {
                        // Stage 9.1: discovery is control-plane only and may arrive
                        // as a plaintext TunnelMessage (no AEAD session required).
                        if let Ok(plain_msg) = decode(inner) {
                            if args.discovery_enabled {
                                if let Some(event) = discovery::parse_discovery_message(&plain_msg)
                                {
                                    let now = crate::ant::now_ms();
                                    match event {
                                        DiscoveryMessage::Advertise(advertisement) => {
                                            let mut store = discovery_store_recv.lock().unwrap();
                                            store.purge_expired(now);
                                            store.insert(advertisement);
                                            debug!(
                                                event = "discovery_advertise_received",
                                                peer = %from,
                                                "received discovery advertise"
                                            );
                                            continue;
                                        }
                                        DiscoveryMessage::Query(query) => {
                                            let response_payload = {
                                                let mut store =
                                                    discovery_store_recv.lock().unwrap();
                                                store.on_query_received();
                                                store.purge_expired(now);
                                                discovery::build_discovery_response_payload(
                                                    &store, &query, now,
                                                )
                                            };
                                            let payload_bytes =
                                                match bincode::serialize(&response_payload) {
                                                    Ok(b) => b,
                                                    Err(_) => continue,
                                                };
                                            let resp_msg = TunnelMessage::new(
                                                MsgType::DiscoveryResponse,
                                                1,
                                                0,
                                                0,
                                                payload_bytes,
                                            );
                                            if let Ok(packet) =
                                                discovery::build_plaintext_packet(&from, &resp_msg)
                                            {
                                                let send_ok = udp_send_discovery
                                                    .send(&from, &packet)
                                                    .await
                                                    .is_ok();
                                                if send_ok {
                                                    let mut store =
                                                        discovery_store_recv.lock().unwrap();
                                                    store.on_response_sent();
                                                }
                                            }
                                            debug!(
                                                event = "discovery_response",
                                                peer = %from,
                                                sent_ads = response_payload.advertisements.len(),
                                                "answered discovery query"
                                            );
                                            continue;
                                        }
                                        DiscoveryMessage::Response(resp) => {
                                            let mut store = discovery_store_recv.lock().unwrap();
                                            store.on_response_received();
                                            store.purge_expired(now);
                                            for adv in resp.advertisements.into_iter() {
                                                store.insert(adv);
                                            }
                                            debug!(
                                                event = "discovery_response_received",
                                                peer = %from,
                                                "received discovery response"
                                            );
                                            continue;
                                        }
                                    }
                                }
                            }
                        }
                        error!(%e, "client failed to open message");
                        continue;
                    }
                };
                if msg.header.version != PROTOCOL_VERSION {
                    error!("client: protocol version mismatch");
                    continue;
                }
                let sid = msg.header.stream_id;
                let msg_type = msg.header.msg_type;
                let payload_len = msg.payload.len();
                debug!(
                    stream_id = sid,
                    msg_type = ?msg_type,
                    payload_len = payload_len,
                    "client received tunnel message (tcp mode)"
                );

                if msg_type == MsgType::Data {
                    let frame: StreamFrame = match bincode::deserialize(&msg.payload) {
                        Ok(f) => f,
                        Err(e) => {
                            error!(%e, "client: failed to decode StreamFrame");
                            continue;
                        }
                    };
                    debug!(
                        stream_id = sid,
                        frame_seq = frame.frame_seq,
                        payload_len = frame.payload.len(),
                        "client: received StreamFrame from tunnel (response path)"
                    );

                    let active_tx = {
                        let map = response_senders_recv.lock().unwrap();
                        map.get(&sid).cloned()
                    };
                    let active_rs = {
                        let map = reliable_streams_recv.lock().unwrap();
                        map.get(&sid).cloned()
                    };
                    let active_route = {
                        let map = stream_routes_for_ack.lock().unwrap();
                        map.get(&sid).cloned()
                    };
                    let completed_entry = if active_tx.is_none() && active_rs.is_none() {
                        snapshot_completed_response_stream(&completed_response_streams_recv, sid)
                    } else {
                        None
                    };
                    let (rs_arc, route_for_this_stream, maybe_tx, completion_reason) =
                        if let Some(rs_arc) = active_rs {
                            (
                                rs_arc,
                                active_route.unwrap_or_else(|| default_route_for_ack.clone()),
                                active_tx,
                                None,
                            )
                        } else if active_tx.is_some() {
                            let rs_arc = {
                                let mut map = reliable_streams_recv.lock().unwrap();
                                map.entry(sid)
                                    .or_insert_with(|| Arc::new(Mutex::new(ReliableStream::new())))
                                    .clone()
                            };
                            (
                                rs_arc,
                                active_route.unwrap_or_else(|| default_route_for_ack.clone()),
                                active_tx,
                                None,
                            )
                        } else if let Some(completed) = completed_entry {
                            (
                                completed.reliable.clone(),
                                completed.route.clone(),
                                None,
                                Some(completed.completion_reason),
                            )
                        } else {
                            error!(
                                stream_id = sid,
                                frame_seq = frame.frame_seq,
                                payload_len = frame.payload.len(),
                                "client: response payload for unknown stream"
                            );
                            continue;
                        };

                    // Feed into per-stream reliable receive path and build cumulative ACK.
                    let (deliver, ack_seq, end_of_stream) = {
                        let mut guard = rs_arc.lock().unwrap();
                        guard.process_incoming(&frame)
                    };

                    // Send ACK back to exit (as Ping control message).
                    let ack = AckFrame {
                        stream_id: sid,
                        ack_seq,
                    };
                    if let Ok(ack_bytes) = bincode::serialize(&ack) {
                        let ack_msg = TunnelMessage::new(MsgType::Ping, 1, sid, 0, ack_bytes);
                        let Some(first_hop_for_ack) = route_for_this_stream.first_hop() else {
                            continue;
                        };
                        let routing = RoutingInfo {
                            hop_index: 0,
                            route: route_for_this_stream.clone(),
                        };
                        if let Ok(ct) =
                            build_encrypted_packet(&crypto_send_for_ack, &routing, ack_msg)
                        {
                            let _ = udp_send_for_ack.send(&first_hop_for_ack, &ct).await;
                        }
                    }

                    if let Some(tx) = maybe_tx {
                        for chunk in deliver {
                            if !chunk.is_empty() {
                                info!(
                                    stream_id = sid,
                                    bytes = chunk.len(),
                                    "client delivering response chunk"
                                );
                                let first_response_payload = {
                                    let mut map = response_started_recv.lock().unwrap();
                                    map.insert(sid, true).is_none()
                                };
                                if first_response_payload {
                                    emit_client_stage(
                                        "response_payload_received",
                                        json!({
                                            "stream_id": sid,
                                            "route_len": route_for_this_stream.len(),
                                            "route_chain": route_for_this_stream
                                                .hops
                                                .iter()
                                                .map(|hop| hop.to_string())
                                                .collect::<Vec<_>>()
                                                .join(" -> "),
                                            "chunk_bytes": chunk.len(),
                                            "frame_seq": frame.frame_seq,
                                        }),
                                    );
                                }
                                let _ = tx.send(Some(chunk));
                            }
                        }
                        if end_of_stream {
                            debug!(stream_id = sid, "client: end-of-stream from reliable layer");
                            let _ = tx.send(None);
                        }
                    } else if let Some(completion_reason) = completion_reason {
                        let late_bytes: usize = deliver.iter().map(|chunk| chunk.len()).sum();
                        let (completed_age_ms, _, late_count) =
                            note_completed_response_late_payload(
                                &completed_response_streams_recv,
                                sid,
                            )
                            .unwrap_or((0, completion_reason, 0));
                        emit_client_stage(
                            "late_payload_after_completion",
                            json!({
                                "stream_id": sid,
                                "route_len": route_for_this_stream.len(),
                                "route_chain": route_for_this_stream
                                    .hops
                                    .iter()
                                    .map(|hop| hop.to_string())
                                    .collect::<Vec<_>>()
                                    .join(" -> "),
                                "frame_seq": frame.frame_seq,
                                "ack_seq": ack_seq,
                                "delivered_chunks": deliver.len(),
                                "delivered_bytes": late_bytes,
                                "end_of_stream": end_of_stream,
                                "completed_age_ms": completed_age_ms,
                                "completion_reason": completion_reason,
                                "late_payload_count": late_count,
                                "duplicate_like": late_bytes == 0 && !end_of_stream,
                            }),
                        );
                        debug!(
                            stream_id = sid,
                            frame_seq = frame.frame_seq,
                            ack_seq,
                            delivered_bytes = late_bytes,
                            end_of_stream,
                            completion_reason,
                            completed_age_ms,
                            late_payload_count = late_count,
                            "client: late response payload after completion"
                        );
                    } else {
                        error!(
                            stream_id = sid,
                            "client: MISSING response channel for response payloads"
                        );
                    }
                } else if msg_type == MsgType::Ping {
                    // ACK-only control message from exit for client->exit request stream.
                    let ack: AckFrame = match bincode::deserialize(&msg.payload) {
                        Ok(a) => a,
                        Err(e) => {
                            error!(%e, "client: failed to decode AckFrame");
                            continue;
                        }
                    };
                    let rs = {
                        let mut map = reliable_streams_recv.lock().unwrap();
                        map.entry(ack.stream_id)
                            .or_insert_with(|| Arc::new(Mutex::new(ReliableStream::new())))
                            .clone()
                    };
                    let mut guard = rs.lock().unwrap();
                    let (acked, avg_latency_ms) = guard.apply_ack_with_latency(ack.ack_seq);
                    if acked > 0 {
                        debug!(
                            stream_id = ack.stream_id,
                            ack_seq = ack.ack_seq,
                            acked_frames = acked,
                            ack_latency_ms_avg = ?avg_latency_ms,
                            "client: cumulative ACK applied"
                        );
                    }
                } else if msg_type == MsgType::Error {
                    let maybe_tx = {
                        let map = response_senders_recv.lock().unwrap();
                        map.get(&sid).cloned()
                    };
                    if let Some(tx) = maybe_tx {
                        let body = if msg.payload.is_empty() {
                            b"upstream error".to_vec()
                        } else {
                            msg.payload.clone()
                        };
                        debug!(
                            stream_id = sid,
                            body_len = body.len(),
                            "client received Error, mapping to HTTP 502"
                        );
                        let resp = format!(
                            "HTTP/1.1 502 Bad Gateway\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            body.len()
                        );
                        let mut full = resp.into_bytes();
                        full.extend_from_slice(&body);
                        let _ = tx.send(Some(full));
                        let _ = tx.send(None);
                    }
                } else if msg_type == MsgType::CloseStream {
                    // Treat CloseStream as end-of-response as well. This makes the TCP-mode
                    // client robust even if the empty DATA end marker is lost.
                    let maybe_tx = {
                        let map = response_senders_recv.lock().unwrap();
                        map.get(&sid).cloned()
                    };
                    if let Some(tx) = maybe_tx {
                        debug!(
                            stream_id = sid,
                            "client: received CloseStream, ending response"
                        );
                        let _ = tx.send(None);
                    } else if let Some((completed_age_ms, completion_reason, late_close_count)) =
                        note_completed_response_close(&completed_response_streams_recv, sid)
                    {
                        emit_client_stage(
                            "late_close_after_completion",
                            json!({
                                "stream_id": sid,
                                "completed_age_ms": completed_age_ms,
                                "completion_reason": completion_reason,
                                "late_close_count": late_close_count,
                            }),
                        );
                        debug!(
                            stream_id = sid,
                            completed_age_ms,
                            completion_reason,
                            late_close_count,
                            "client: late CloseStream after completion"
                        );
                    }
                } else if msg_type == MsgType::Ant {
                    // Stage 7: measurement-only ants. Optional and bounded.
                    let ant: Ant = match bincode::deserialize(&msg.payload) {
                        Ok(a) => a,
                        Err(e) => {
                            debug!(%e, "client: failed to decode ant");
                            continue;
                        }
                    };
                    if !ant.validate() || !ant.size_ok() {
                        debug!("client: dropping invalid/oversize ant");
                        continue;
                    }
                    {
                        let mut d = ant_dedup_recv.lock().unwrap();
                        if !d.check_and_mark(ant.id) {
                            debug!("client: duplicate ant ignored");
                            continue;
                        }
                    }
                    if ant.ant_type == AntType::Echo {
                        let rtt = crate::ant::now_ms().saturating_sub(ant.created_at_ms);
                        let mut store = route_store_for_ants.lock().await;
                        for hop in &ant.path {
                            let key = crate::route_store::TransportKey {
                                protocol: hop.protocol,
                                port: hop.port,
                            };
                            store.record_transport_success(key, Some(rtt));
                            // Also reinforce pheromone memory for this transport edge.
                            store.reinforce_transport_pheromone(
                                hop.protocol,
                                hop.port,
                                Some(rtt),
                                true,
                            );
                        }
                        debug!(
                            rtt_ms = rtt,
                            obs = ant.observations.len(),
                            "client: applied ant observations"
                        );
                    }
                } else {
                    debug!(
                        stream_id = sid,
                        msg_type = ?msg_type,
                        "client ignoring non-data/error message type"
                    );
                }
            }
        });

        // Background retransmit task (Stage 4/5 reliable stream).
        let udp_retx = transport.clone();
        let crypto_retx = crypto.clone();
        let reliable_streams_retx = reliable_streams.clone();
        let stream_routes_retx = stream_routes_for_io.clone();
        let default_route_retx = primary_route.clone();
        let retx_interval = Duration::from_millis(args.retransmit_interval.max(50));
        let chunk_size = args.chunk_size.max(200);
        tokio::spawn(async move {
            let tick = Duration::from_millis(50);
            loop {
                tokio::time::sleep(tick).await;
                let now = std::time::Instant::now();
                let frames: Vec<(u32, StreamFrame)> = {
                    let map = reliable_streams_retx.lock().unwrap();
                    let mut out = Vec::new();
                    for (&sid, rs_arc) in map.iter() {
                        let mut rs = rs_arc.lock().unwrap();
                        for f in rs.frames_for_retransmit(sid, now, retx_interval, 16) {
                            out.push((sid, f));
                        }
                    }
                    out
                };
                for (_sid, f) in frames {
                    let Ok(frame_bytes) = bincode::serialize(&f) else {
                        continue;
                    };
                    let msg = TunnelMessage::new(MsgType::Data, 1, f.stream_id, 0, frame_bytes);
                    let route_for_this_stream = {
                        let map = stream_routes_retx.lock().unwrap();
                        map.get(&f.stream_id)
                            .cloned()
                            .unwrap_or_else(|| default_route_retx.clone())
                    };
                    let Some(first_hop_for_this_stream) = route_for_this_stream.first_hop() else {
                        continue;
                    };
                    let routing = RoutingInfo {
                        hop_index: 0,
                        route: route_for_this_stream,
                    };
                    if let Ok(ct) = build_encrypted_packet(&crypto_retx, &routing, msg) {
                        let _ = udp_retx.send(&first_hop_for_this_stream, &ct).await;
                    }
                }
                let _ = chunk_size; // keep for future tuning without warnings
            }
        });

        if ants_enabled {
            // spawn a low-rate echo ant sender that uses best route as a template
            let udp_ant = transport.clone();
            let crypto_ant = crypto.clone();
            let route_store_ant = route_store.clone();
            let ant_dedup_send = ant_dedup.clone();
            tokio::spawn(async move {
                use rand_core::{OsRng, RngCore};
                let tick = Duration::from_secs(5);
                loop {
                    tokio::time::sleep(tick).await;
                    let now = Instant::now();
                    let pick = {
                        let mut s = route_store_ant.lock().await;
                        s.get_best_route(now, &[])
                    };
                    let Some((idx, _score)) = pick else { continue };
                    let route = {
                        let s = route_store_ant.lock().await;
                        s.routes().get(idx).map(|c| c.route.clone())
                    };
                    let Some(route) = route else { continue };
                    let mut id = [0u8; 16];
                    OsRng.fill_bytes(&mut id);
                    {
                        let mut d = ant_dedup_send.lock().unwrap();
                        let _ = d.check_and_mark(id);
                    }
                    let ant = Ant {
                        id,
                        ant_type: AntType::Echo,
                        ttl: 2,
                        path: route.hops.clone(),
                        observations: vec![],
                        created_at_ms: crate::ant::now_ms(),
                    };
                    if !ant.size_ok() {
                        continue;
                    }
                    let payload = match bincode::serialize(&ant) {
                        Ok(p) => p,
                        Err(_) => continue,
                    };
                    let msg = TunnelMessage::new(MsgType::Ant, 1, 0, 0, payload);
                    let routing = RoutingInfo {
                        hop_index: 0,
                        route: route.clone(),
                    };
                    if let Ok(ct) = build_encrypted_packet(&crypto_ant, &routing, msg) {
                        let Some(first_hop) = route.first_hop() else {
                            continue;
                        };
                        let _ = udp_ant.send(&first_hop, &ct).await;
                        debug!(hops = route.len(), "client: ant created/sent");
                    }
                }
            });
        }

        run_client_tcp_mode(
            args,
            transport,
            crypto,
            route_store,
            stream_routes_for_io,
            response_senders,
            reliable_streams,
            response_started,
            completed_response_streams,
        )
        .await
    } else {
        run_client_tun_mode(
            args,
            transport,
            crypto,
            first_hop,
            primary_route,
            discovery_store,
        )
        .await
    }
}

async fn run_client_tcp_mode(
    args: ClientArgs,
    udp: UdpTransport,
    crypto: SessionCrypto,
    route_store: Arc<AsyncMutex<RouteStore>>,
    stream_routes: StreamRouteTable,
    response_senders: ResponseSenderTable,
    reliable_streams: ReliableStreamTable,
    response_started: ResponseStartedTable,
    completed_response_streams: CompletedResponseStreamTable,
) -> Result<()> {
    let listener = TcpListener::bind(args.local_listen).await?;
    info!(addr = %args.local_listen, "client listening for local tcp");

    let next_stream_id = Arc::new(AtomicU32::new(1));
    let next_probe_stream_id = Arc::new(AtomicU32::new(1_000_000_000));

    // Stage 6: initial RTT probes for all candidate routes.
    // Runs in background; if it fails, normal requests still work.
    {
        let udp_probe = udp.clone();
        let crypto_probe = crypto.clone();
        let route_store_probe = route_store.clone();
        let response_senders_probe = response_senders.clone();
        let reliable_streams_probe = reliable_streams.clone();
        let response_started_probe = response_started.clone();
        let stream_routes_probe = stream_routes.clone();
        let completed_response_streams_probe = completed_response_streams.clone();
        let next_sid_probe = next_probe_stream_id.clone();
        let chunk_size = args.chunk_size.max(256).min(1200);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(100)).await;
            let routes_snapshot = {
                let store = route_store_probe.lock().await;
                store
                    .routes()
                    .iter()
                    .map(|c| c.route.clone())
                    .collect::<Vec<_>>()
            };
            for (idx, r) in routes_snapshot.into_iter().enumerate() {
                let sid = next_sid_probe.fetch_add(1, Ordering::Relaxed);
                let start = Instant::now();
                let res = tunnel_http_roundtrip(
                    &udp_probe,
                    &crypto_probe,
                    &r,
                    sid,
                    &build_probe_request(),
                    chunk_size,
                    args.max_inflight_frames,
                    &response_senders_probe,
                    &reliable_streams_probe,
                    &response_started_probe,
                    &stream_routes_probe,
                    &completed_response_streams_probe,
                    None,
                    false,
                )
                .await;
                let rtt_ms = start.elapsed().as_millis() as u64;
                let ok = res
                    .as_ref()
                    .ok()
                    .and_then(|b| http_status_code(b))
                    .map(|c| c >= 200 && c < 500)
                    .unwrap_or(false);
                let mut store = route_store_probe.lock().await;
                store.update_metrics(idx, Some(rtt_ms), ok);
            }
        });
    }

    loop {
        if drain::is_draining() {
            info!("[drain] rejecting new sessions");
            break;
        }
        let (mut tcp, addr) = listener.accept().await?;
        let sid0 = next_stream_id.fetch_add(1, Ordering::Relaxed);
        info!(%addr, stream_id = sid0, "client accepted local connection");
        let udp = udp.clone();
        let crypto = crypto.clone();
        let route_store = route_store.clone();
        let stream_routes = stream_routes.clone();
        let next_stream_id = next_stream_id.clone();
        let response_senders_for_task = response_senders.clone();
        let reliable_streams_for_task = reliable_streams.clone();
        let response_started_for_task = response_started.clone();
        let completed_response_streams_for_task = completed_response_streams.clone();

        tokio::spawn(async move {
            // Read request from local TCP.  For plain HTTP we buffer until the
            // full request is assembled (Content-Length / header-end heuristics).
            // For TLS/binary protocols (first byte == 0x16) we break out of the
            // buffering loop immediately and switch to a bidirectional streaming
            // mode so the TLS handshake round-trips can complete.
            let mut buf = vec![0u8; 4096];
            let mut request_buf = Vec::new();
            let first_byte_timeout = Duration::from_secs(10);
            let read_idle_timeout = Duration::from_secs(5);
            let accept_start = std::time::Instant::now();
            let mut is_tls = false;

            loop {
                // TLS fast-path: first byte 0x16 = TLS handshake record.
                // Send immediately without waiting for idle timeout.
                if !request_buf.is_empty() && request_buf[0] == 0x16 {
                    // Read the rest of the TLS ClientHello quickly (≤200 ms).
                    let _ = timeout(Duration::from_millis(200), async {
                        loop {
                            match tcp.read(&mut buf).await {
                                Ok(n) if n > 0 => request_buf.extend_from_slice(&buf[..n]),
                                _ => break,
                            }
                        }
                    })
                    .await;
                    is_tls = true;
                    break;
                }

                // Determine if we already know how much to read.
                let target_len =
                    if let Some((hdr_end, body_len)) = http_content_length(&request_buf) {
                        Some(hdr_end + body_len)
                    } else {
                        None
                    };
                if let Some(need) = target_len {
                    if request_buf.len() >= need {
                        debug!(
                            stream_id = sid0,
                            total = request_buf.len(),
                            "client: full HTTP request buffered (headers + body)"
                        );
                        break;
                    }
                } else if let Some(hdr_end) = http_header_end(&request_buf) {
                    // No Content-Length: treat as headers-only request (e.g. GET).
                    if request_buf.len() >= hdr_end {
                        debug!(
                            stream_id = sid0,
                            total = request_buf.len(),
                            "client: HTTP headers buffered (no content-length), treating as complete request"
                        );
                        break;
                    }
                }

                let elapsed = accept_start.elapsed();
                let tout = if request_buf.is_empty() {
                    first_byte_timeout.saturating_sub(elapsed)
                } else {
                    read_idle_timeout
                };
                let read_result = timeout(tout, tcp.read(&mut buf)).await;

                let n = match read_result {
                    Err(_) => {
                        if request_buf.is_empty() {
                            error!(
                                stream_id = sid0,
                                "client: timed out waiting for first byte from local tcp"
                            );
                            return;
                        }
                        if let Some(need) = target_len {
                            if request_buf.len() < need {
                                error!(
                                    stream_id = sid0,
                                    have = request_buf.len(),
                                    need,
                                    "client: idle timeout before full HTTP body received"
                                );
                                return;
                            }
                        }
                        debug!(
                            stream_id = sid0,
                            "client: read idle timeout with buffered request, treating as complete"
                        );
                        break;
                    }
                    Ok(Ok(0)) => {
                        if request_buf.is_empty() {
                            // Expected during readiness/probe connections that only test
                            // listener availability and close immediately.
                            debug!("client: local tcp closed before sending request");
                            return;
                        }
                        debug!(
                            stream_id = sid0,
                            "client: local tcp closed, finishing request buffering"
                        );
                        break;
                    }
                    Ok(Ok(n)) => n,
                    Ok(Err(e)) => {
                        error!(%e, "client failed to read from local tcp");
                        return;
                    }
                };

                request_buf.extend_from_slice(&buf[..n]);
                debug!(
                    stream_id = sid0,
                    read_bytes = n,
                    total = request_buf.len(),
                    "client: buffered request bytes from local tcp"
                );
            }

            if request_buf.is_empty() {
                error!(stream_id = sid0, "client: empty buffered request, aborting");
                return;
            }
            info!(
                stream_id = sid0,
                bytes = request_buf.len(),
                "client: buffered full local request"
            );
            let request_site = request_site_label(&request_buf);
            emit_client_stage(
                "request_buffered",
                json!({
                    "stream_id": sid0,
                    "site": request_site.as_str(),
                    "request_bytes": request_buf.len(),
                    "buffer_ms": accept_start.elapsed().as_millis() as u64,
                    "local_peer": addr.to_string(),
                    "tls_mode": is_tls,
                }),
            );

            let chunk_size = args.chunk_size.max(256).min(1200);
            let mut excluded: Vec<Route> = Vec::new();
            let mut final_resp: Option<Vec<u8>> = None;
            let mut used_idx: Option<usize> = None;
            let mut used_score: f32 = 0.0;
            let mut used_hops: usize = 0;
            let mut rtt_ms: Option<u64> = None;

            for attempt in 0..3 {
                let now = Instant::now();
                let pick = {
                    let mut store = route_store.lock().await;
                    store.get_best_route(now, &excluded)
                };
                let Some((idx, score)) = pick else {
                    break;
                };
                let route = {
                    let store = route_store.lock().await;
                    store.routes().get(idx).map(|c| c.route.clone())
                };
                let Some(route) = route else { break };
                let hops = route.len();
                let details = {
                    let store = route_store.lock().await;
                    store.score_details(idx, now)
                };
                if let Some(d) = details {
                    info!(
                        stream_id = sid0,
                        attempt,
                        hops,
                        score = d.final_score,
                        base = d.base,
                        transport_agg = d.transport_agg,
                        "route selected: hops=N score=X"
                    );
                } else {
                    info!(
                        stream_id = sid0,
                        attempt, hops, score, "route selected: hops=N score=X"
                    );
                }
                let sid = if attempt == 0 {
                    sid0
                } else {
                    next_stream_id.fetch_add(1, Ordering::Relaxed)
                };
                emit_client_stage(
                    "route_selected",
                    json!({
                    "stream_id": sid,
                    "site": request_site.as_str(),
                    "attempt": attempt,
                    "route_len": hops,
                        "route_chain": route
                            .hops
                            .iter()
                            .map(|hop| hop.to_string())
                            .collect::<Vec<_>>()
                            .join(" -> "),
                        "score": score,
                        "since_request_buffered_ms": accept_start.elapsed().as_millis() as u64,
                    }),
                );
                let start_rtt = Instant::now();
                match tunnel_http_roundtrip(
                    &udp,
                    &crypto,
                    &route,
                    sid,
                    &request_buf,
                    chunk_size,
                    args.max_inflight_frames,
                    &response_senders_for_task,
                    &reliable_streams_for_task,
                    &response_started_for_task,
                    &stream_routes,
                    &completed_response_streams_for_task,
                    Some(&mut tcp),
                    is_tls,
                )
                .await
                {
                    Ok(resp) => {
                        let elapsed = start_rtt.elapsed().as_millis() as u64;
                        let code = http_status_code(&resp);
                        // Stage 6 failure rules: treat route-level timeouts (504) as route failure.
                        // Do NOT treat upstream target errors (502) as route failure.
                        let is_fail = matches!(code, Some(504));
                        if is_fail {
                            excluded.push(route.clone());
                            {
                                let mut store = route_store.lock().await;
                                store.record_failure(idx, RouteFailureKind::Timeout);
                            }
                            info!(stream_id = sid0, attempt, hops, score, code = ?code, "route failed, switching");
                            continue;
                        }
                        final_resp = Some(resp);
                        used_idx = Some(idx);
                        used_score = score;
                        used_hops = hops;
                        rtt_ms = Some(elapsed);
                        break;
                    }
                    Err(e) => {
                        excluded.push(route.clone());
                        {
                            let mut store = route_store.lock().await;
                            store.record_failure(idx, classify_anyhow_failure(&e));
                        }
                        info!(stream_id = sid0, attempt, hops, score, %e, "route failed, switching");
                    }
                }
            }

            if let (Some(idx), Some(resp)) = (used_idx, final_resp.as_ref()) {
                info!(
                    stream_id = sid0,
                    hops = used_hops,
                    score = used_score,
                    rtt_ms = rtt_ms.unwrap_or(0),
                    "route success, updating metrics"
                );
                let mut store = route_store.lock().await;
                store.record_success(idx, rtt_ms);
                if let Some(code) = http_status_code(resp) {
                    if code == 504 {
                        // Defensive: if we got a synthesized 504, treat as failure.
                        store.record_failure(idx, RouteFailureKind::Timeout);
                    }
                }
            }

            if let Some(resp) = final_resp {
                // For TLS/streaming mode, `tunnel_http_roundtrip` already writes
                // chunks to the local TCP socket as they arrive.
                if !is_tls {
                    if let Err(e) = tcp.write_all(&resp).await {
                        error!(%e, "client failed to write response to local tcp");
                    }
                }
            } else {
                error!(stream_id = sid0, "client: all routes failed");
                // For TLS mode we must not write plaintext HTTP fallback into an
                // active TLS tunnel.
                if !is_tls {
                    let fallback = b"HTTP/1.1 504 Gateway Timeout\r\nConnection: close\r\n\r\n";
                    let _ = tcp.write_all(fallback).await;
                }
            }

            // For TLS/streaming sessions: after delivering the first response
            // (TLS ServerHello+Certificate), the remote TLS stack expects more
            // round-trips (TLS Finished, then the actual HTTP request, etc.).
            // We run a bidirectional continuation loop: read more data from the
            // local TCP socket and do additional tunnel round-trips until the
            // connection closes on either side.
            if is_tls {
                let chunk_size = args.chunk_size.max(256).min(1200);
                let mut cont_buf = vec![0u8; 4096];
                loop {
                    // Read next chunk from curl (TLS Finished, HTTP request, …)
                    let n = match timeout(Duration::from_secs(30), tcp.read(&mut cont_buf)).await {
                        Ok(Ok(0)) | Err(_) => break, // TCP closed or idle timeout
                        Ok(Ok(n)) => n,
                        Ok(Err(_)) => break,
                    };
                    let more_data = cont_buf[..n].to_vec();
                    // Reuse the original stream_id so the exit looks up the
                    // same TCP socket it kept open for this streaming session.
                    let cont_sid = sid0;
                    let pick = {
                        let mut store = route_store.lock().await;
                        store.get_best_route(Instant::now(), &[])
                    };
                    let Some((idx, _score)) = pick else { break };
                    let route = {
                        let store = route_store.lock().await;
                        store.routes().get(idx).map(|c| c.route.clone())
                    };
                    let Some(route) = route else { break };
                    match tunnel_http_roundtrip(
                        &udp,
                        &crypto,
                        &route,
                        cont_sid,
                        &more_data,
                        chunk_size,
                        args.max_inflight_frames,
                        &response_senders_for_task,
                        &reliable_streams_for_task,
                        &response_started_for_task,
                        &stream_routes,
                        &completed_response_streams_for_task,
                        Some(&mut tcp),
                        true,
                    )
                    .await
                    {
                        Ok(resp) => {
                            if resp.is_empty() {
                                continue; // empty response is OK for TLS ACKs
                            }
                            if tcp.write_all(&resp).await.is_err() {
                                break;
                            }
                        }
                        Err(_) => break,
                    }
                }
            }

            let _ = tcp.shutdown().await;
        });
    }
    // Wait for drain deadline (if any) so existing streams can complete.
    while drain::is_draining() && !drain::deadline_reached() {
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    if drain::is_draining() {
        info!("[drain] shutdown complete");
    }
    Ok(())
}

async fn tunnel_http_roundtrip(
    udp: &UdpTransport,
    crypto: &SessionCrypto,
    route: &Route,
    stream_id: u32,
    request: &[u8],
    chunk_size: usize,
    max_inflight_frames: usize,
    response_senders: &ResponseSenderTable,
    reliable_streams: &ReliableStreamTable,
    response_started: &ResponseStartedTable,
    stream_routes: &StreamRouteTable,
    completed_response_streams: &CompletedResponseStreamTable,
    mut local_tcp: Option<&mut tokio::net::TcpStream>,
    write_response_to_tcp: bool,
) -> Result<Vec<u8>> {
    let stream_start = Instant::now();
    let first_hop = route
        .first_hop()
        .ok_or_else(|| anyhow::anyhow!("client: empty route for stream"))?;
    let request_site = request_site_label(request);
    let route_chain = route
        .hops
        .iter()
        .map(|hop| hop.to_string())
        .collect::<Vec<_>>()
        .join(" -> ");
    debug!(
        stream_id = stream_id,
        bytes = request.len(),
        hops = route.len(),
        "client: tunnel_http_roundtrip start"
    );
    emit_client_stage(
        "roundtrip_started",
        json!({
            "stream_id": stream_id,
            "site": request_site.as_str(),
            "route_len": route.len(),
            "route_chain": route_chain.as_str(),
            "request_bytes": request.len(),
            "write_response_to_tcp": write_response_to_tcp,
        }),
    );

    {
        let mut map = stream_routes.lock().unwrap();
        map.insert(stream_id, route.clone());
    }

    let (tx_from_udp, mut rx_from_udp_stream) = mpsc::unbounded_channel::<Option<Vec<u8>>>();
    {
        let mut map = response_senders.lock().unwrap();
        map.insert(stream_id, tx_from_udp);
    }
    emit_client_stage(
        "response_channel_created",
        json!({
            "stream_id": stream_id,
            "site": request_site.as_str(),
            "route_len": route.len(),
            "route_chain": route_chain.as_str(),
            "write_response_to_tcp": write_response_to_tcp,
        }),
    );

    let rs_arc = {
        let mut map = reliable_streams.lock().unwrap();
        map.entry(stream_id)
            .or_insert_with(|| Arc::new(Mutex::new(ReliableStream::new())))
            .clone()
    };

    let routing = RoutingInfo {
        hop_index: 0,
        route: route.clone(),
    };

    // Send request frames.
    let chunk_size = chunk_size.max(256).min(1200);
    let mut offset = 0;
    let mut sent_chunks = 0usize;
    let mut sent_bytes = 0usize;
    let mut first_hop_sent_logged = false;
    while offset < request.len() {
        // Backpressure: keep inflight bounded to avoid packet storms and
        // excessive out-of-order that could trip session-level replay windows.
        loop {
            let inflight = { rs_arc.lock().unwrap().inflight() };
            if inflight < max_inflight_frames.max(8) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }

        let end = (offset + chunk_size).min(request.len());
        let chunk = request[offset..end].to_vec();
        offset = end;
        sent_chunks += 1;
        sent_bytes += chunk.len();

        let frame = {
            let mut rs = rs_arc.lock().unwrap();
            rs.build_outgoing_frame(stream_id, chunk)
        };
        let frame_bytes = bincode::serialize(&frame)?;
        let msg = TunnelMessage::new(MsgType::Data, 1, stream_id, 0, frame_bytes);
        let ct = build_encrypted_packet(crypto, &routing, msg)?;
        udp.send(&first_hop, &ct).await?;
        if !first_hop_sent_logged {
            emit_client_stage(
                "first_hop_send",
                json!({
                    "stream_id": stream_id,
                    "site": request_site.as_str(),
                    "route_len": route.len(),
                    "route_chain": route_chain.as_str(),
                    "since_roundtrip_start_ms": stream_start.elapsed().as_millis() as u64,
                    "first_chunk_bytes": frame.payload.len(),
                }),
            );
            first_hop_sent_logged = true;
        }
        {
            let mut rs = rs_arc.lock().unwrap();
            rs.mark_sent(frame.frame_seq);
        }
    }
    debug!(
        stream_id = stream_id,
        sent_chunks, sent_bytes, "client: sent request chunks over tunnel"
    );

    // End-of-request marker.
    loop {
        let inflight = { rs_arc.lock().unwrap().inflight() };
        if inflight < max_inflight_frames.max(8) {
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let end_frame = {
        let mut rs = rs_arc.lock().unwrap();
        rs.build_outgoing_frame(stream_id, vec![])
    };
    let end_bytes = bincode::serialize(&end_frame)?;
    let msg = TunnelMessage::new(MsgType::Data, 1, stream_id, 0, end_bytes);
    let ct = build_encrypted_packet(crypto, &routing, msg)?;
    udp.send(&first_hop, &ct).await?;
    {
        let mut rs = rs_arc.lock().unwrap();
        rs.mark_sent(end_frame.frame_seq);
    }

    // Wait briefly for request delivery ACKs so the exit doesn't flush a partial request.
    // This keeps Stage 4 reliability deterministic for large transfers.
    let wait_deadline = Instant::now() + Duration::from_secs(2);
    loop {
        let inflight = { rs_arc.lock().unwrap().inflight() };
        if inflight == 0 {
            break;
        }
        if Instant::now() >= wait_deadline {
            debug!(
                stream_id = stream_id,
                inflight, "client: request still inflight after wait, proceeding to read response"
            );
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Receive response.
    // In TLS/streaming mode we must write chunks to the local TCP socket
    // immediately, otherwise curl's TLS stack blocks until the full body is
    // buffered.
    let mut full = Vec::new();
    let mut resp_bytes_written: usize = 0;
    let mut resp_frames: usize = 0;
    let mut first_client_byte_ms: Option<u64> = None;
    let idle_timeout = if write_response_to_tcp {
        Duration::from_secs(120)
    } else {
        Duration::from_secs(5)
    };
    let completion_reason: &'static str = loop {
        let msg = match timeout(idle_timeout, rx_from_udp_stream.recv()).await {
            Ok(v) => v,
            Err(_) => {
                if write_response_to_tcp {
                    continue; // keep waiting for stream end marker
                }
                emit_client_stage(
                    "response_timeout",
                    json!({
                        "stream_id": stream_id,
                        "site": request_site.as_str(),
                        "route_len": route.len(),
                        "route_chain": route_chain.as_str(),
                        "idle_timeout_ms": idle_timeout.as_millis() as u64,
                    }),
                );
                break "response_idle_timeout";
            }
        };
        let Some(msg) = msg else {
            break "response_channel_closed";
        };
        match msg {
            Some(chunk) => {
                if first_client_byte_ms.is_none() && !chunk.is_empty() {
                    let since_start = stream_start.elapsed().as_millis() as u64;
                    first_client_byte_ms = Some(since_start);
                    emit_client_stage(
                        "first_byte_delivered",
                        json!({
                            "stream_id": stream_id,
                            "site": request_site.as_str(),
                            "route_len": route.len(),
                            "route_chain": route_chain.as_str(),
                            "since_roundtrip_start_ms": since_start,
                            "chunk_bytes": chunk.len(),
                        }),
                    );
                }
                if write_response_to_tcp {
                    let tcp = local_tcp
                        .as_mut()
                        .expect("write_response_to_tcp=true requires local_tcp");
                    if let Err(e) = tcp.write_all(&chunk).await {
                        error!(stream_id = stream_id, %e, "client failed writing streamed response");
                        break "local_tcp_write_failed";
                    }
                    resp_bytes_written += chunk.len();
                    resp_frames += 1;
                } else {
                    full.extend(chunk);
                    // Safety cap: in TCP-mode (write_response_to_tcp=false) we buffer
                    // the entire HTTP response in memory until we detect its
                    // Content-Length or hit an upper bound.
                    //
                    // Previous hard limit (66 KiB) made large transfers impossible.
                    // We cap based on the request size with a small buffer.
                    let max_full = request.len().saturating_add(256 * 1024);
                    if full.len() >= max_full {
                        break "buffer_cap_reached";
                    }
                    if let Some((hdr_end, body_len)) = http_content_length(&full) {
                        let need = hdr_end + body_len;
                        if full.len() >= need {
                            break "content_length_satisfied";
                        }
                    }
                }
            }
            None => break "end_of_stream",
        }
    };

    // Cleanup.
    remember_completed_response_stream(
        completed_response_streams,
        stream_id,
        rs_arc.clone(),
        route.clone(),
        completion_reason,
    );
    {
        let mut map = reliable_streams.lock().unwrap();
        map.remove(&stream_id);
    }
    {
        let mut map = response_senders.lock().unwrap();
        map.remove(&stream_id);
    }
    {
        let mut map = response_started.lock().unwrap();
        map.remove(&stream_id);
    }
    {
        let mut map = stream_routes.lock().unwrap();
        map.remove(&stream_id);
    }
    emit_client_stage(
        "response_channel_removed",
        json!({
            "stream_id": stream_id,
            "site": request_site.as_str(),
            "route_len": route.len(),
            "route_chain": route_chain.as_str(),
            "completion_reason": completion_reason,
            "write_response_to_tcp": write_response_to_tcp,
        }),
    );

    if !write_response_to_tcp && full.is_empty() {
        anyhow::bail!("no response (timeout or channel closed)");
    }
    emit_client_stage(
        "stream_complete",
        json!({
            "stream_id": stream_id,
            "site": request_site.as_str(),
            "route_len": route.len(),
            "route_chain": route_chain.as_str(),
            "since_roundtrip_start_ms": stream_start.elapsed().as_millis() as u64,
            "first_client_byte_ms": first_client_byte_ms,
            "response_bytes": if write_response_to_tcp { resp_bytes_written } else { full.len() },
            "response_frames": resp_frames,
            "completion_reason": completion_reason,
            "write_response_to_tcp": write_response_to_tcp,
        }),
    );
    if write_response_to_tcp {
        info!(
            stream_id = stream_id,
            bytes_recv = resp_bytes_written,
            frames_recv = resp_frames,
            stream_duration_ms = stream_start.elapsed().as_millis(),
            "client: finished streamed response"
        );
        Ok(Vec::new())
    } else {
        Ok(full)
    }
}

async fn run_client_tun_mode(
    args: ClientArgs,
    udp: UdpTransport,
    crypto: SessionCrypto,
    first_hop: NodeAddr,
    route: Route,
    discovery_store: Arc<Mutex<discovery::store::DiscoveryStore>>,
) -> Result<()> {
    info!("client starting in tun mode");

    let tun: TunDevice = TunDevice::create(
        &args.tun_name,
        &args.tun_address,
        &args.tun_netmask,
        args.tun_mtu,
    )
    .await?;

    let flow_table = Arc::new(AsyncMutex::new(FlowTable::default()));
    let next_stream_id = Arc::new(AsyncMutex::new(1u32));

    // UDP receive path: tunnel -> client -> TUN.
    let udp_recv = udp.clone();
    let crypto_recv = crypto.clone();
    let flow_table_recv = flow_table.clone();
    let udp_send_discovery = udp.clone();
    let discovery_store_recv = discovery_store.clone();
    let discovery_enabled = args.discovery_enabled;
    let tun_handle = Arc::new(AsyncMutex::new(tun));
    let tun_writer = tun_handle.clone();

    tokio::spawn(async move {
        loop {
            let Ok((_from, data)) = udp_recv.recv().await else {
                break;
            };
            debug!(len = data.len(), "client received via transport (tun mode)");

            let (_routing, inner) = match parse_routing_header(&data) {
                Ok(v) => v,
                Err(e) => {
                    error!(%e, "client tun: failed to parse routing header");
                    continue;
                }
            };

            let msg = match crypto_recv.open_message(inner) {
                Ok(m) => m,
                Err(e) => {
                    // Stage 9.1: discovery is control-plane only and may arrive
                    // as a plaintext TunnelMessage (no AEAD session required).
                    if discovery_enabled {
                        if let Ok(plain_msg) = decode(inner) {
                            if let Some(event) = discovery::parse_discovery_message(&plain_msg) {
                                let now = crate::ant::now_ms();
                                match event {
                                    DiscoveryMessage::Advertise(advertisement) => {
                                        let mut store = discovery_store_recv.lock().unwrap();
                                        store.purge_expired(now);
                                        store.insert(advertisement);
                                        debug!(
                                            event = "discovery_advertise_received",
                                            peer = %_from,
                                            "received discovery advertise"
                                        );
                                        continue;
                                    }
                                    DiscoveryMessage::Query(query) => {
                                        let response_payload = {
                                            let mut store = discovery_store_recv.lock().unwrap();
                                            store.on_query_received();
                                            store.purge_expired(now);
                                            discovery::build_discovery_response_payload(
                                                &store, &query, now,
                                            )
                                        };
                                        let payload_bytes =
                                            match bincode::serialize(&response_payload) {
                                                Ok(b) => b,
                                                Err(_) => continue,
                                            };
                                        let resp_msg = TunnelMessage::new(
                                            MsgType::DiscoveryResponse,
                                            1,
                                            0,
                                            0,
                                            payload_bytes,
                                        );
                                        if let Ok(packet) =
                                            discovery::build_plaintext_packet(&_from, &resp_msg)
                                        {
                                            if udp_send_discovery
                                                .send(&_from, &packet)
                                                .await
                                                .is_ok()
                                            {
                                                let mut store =
                                                    discovery_store_recv.lock().unwrap();
                                                store.on_response_sent();
                                            }
                                        }
                                        debug!(
                                            event = "discovery_response",
                                            peer = %_from,
                                            sent_ads = response_payload.advertisements.len(),
                                            "answered discovery query"
                                        );
                                        continue;
                                    }
                                    DiscoveryMessage::Response(resp) => {
                                        let mut store = discovery_store_recv.lock().unwrap();
                                        store.on_response_received();
                                        store.purge_expired(now);
                                        for adv in resp.advertisements.into_iter() {
                                            store.insert(adv);
                                        }
                                        debug!(
                                            event = "discovery_response_received",
                                            peer = %_from,
                                            "received discovery response"
                                        );
                                        continue;
                                    }
                                }
                            }
                        }
                    }
                    error!(%e, "client tun: failed to open message");
                    continue;
                }
            };

            if msg.header.version != PROTOCOL_VERSION {
                error!("client tun: protocol version mismatch");
                continue;
            }

            let sid = msg.header.stream_id;
            let msg_type = msg.header.msg_type;
            let payload_len = msg.payload.len();

            debug!(
                stream_id = sid,
                msg_type = ?msg_type,
                payload_len = payload_len,
                "client tun: received tunnel message"
            );

            match msg_type {
                MsgType::Data => {
                    // For Stage 2 MVP, assume the payload is a complete IPv4/TCP
                    // packet that can be written back to the TUN device.
                    {
                        let mut table = flow_table_recv.lock().await;
                        let now = std::time::Instant::now();
                        // Keep flow active for this stream (reverse packet)
                        table.touch_by_stream(sid, now);
                        if let Some(flow) = table.get_flow_for_stream(sid) {
                            debug!(
                                stream_id = sid,
                                src = ?flow.src,
                                dst = ?flow.dst,
                                src_port = flow.src_port,
                                dst_port = flow.dst_port,
                                "client tun: reusing existing flow for reverse packet"
                            );
                        } else {
                            debug!(
                                stream_id = sid,
                                "client tun: no flow mapping for stream_id on reverse path"
                            );
                        }
                    }

                    let mut tun = tun_writer.lock().await;
                    if let Err(e) = tun.write_packet(&msg.payload).await {
                        error!(%e, "client tun: failed to write packet to tun");
                    } else {
                        debug!(
                            stream_id = sid,
                            bytes = payload_len,
                            "client tun: wrote packet to tun"
                        );
                    }
                }
                MsgType::Error => {
                    // Cleanup any flow associated with this stream.
                    let mut table = flow_table_recv.lock().await;
                    table.remove_by_stream(sid);
                    debug!(
                        stream_id = sid,
                        "client tun: removed flow mapping due to error message"
                    );
                }
                MsgType::CloseStream => {
                    // Explicit close from exit/relay: drop flow mapping.
                    let mut table = flow_table_recv.lock().await;
                    table.remove_by_stream(sid);
                    debug!(
                        stream_id = sid,
                        "client tun: removed flow mapping due to close-stream message"
                    );
                }
                _ => {
                    debug!(
                        stream_id = sid,
                        msg_type = ?msg_type,
                        "client tun: ignoring non-data/error message type"
                    );
                }
            }
        }
    });

    // Forward path: TUN -> client -> tunnel.
    let mut buf = vec![0u8; 65535];
    loop {
        let n = tun_handle.lock().await.read_packet(&mut buf).await?;
        let pkt = &buf[..n];

        debug!(bytes = n, "client tun: read packet from tun");

        let (ip_hdr, hdr_len) = match Ipv4Header::parse(pkt) {
            Ok(v) => v,
            Err(e) => {
                error!(%e, "client tun: failed to parse ipv4 header");
                continue;
            }
        };

        // Only handle TCP for now.
        if ip_hdr.protocol != 6 {
            debug!(
                protocol = ip_hdr.protocol,
                "client tun: skipping non-TCP packet"
            );
            continue;
        }

        let tcp_offset = hdr_len;
        if pkt.len() <= tcp_offset {
            debug!("client tun: packet too short for tcp header");
            continue;
        }
        let tcp = &pkt[tcp_offset..];
        let (src_port, dst_port) = match parse_tcp_ports(tcp) {
            Ok(v) => v,
            Err(e) => {
                error!(%e, "client tun: failed to parse tcp ports");
                continue;
            }
        };

        let flow_key = FlowKey::new(ip_hdr.src, ip_hdr.dst, src_port, dst_port, ip_hdr.protocol);

        let stream_id = {
            let mut table = flow_table.lock().await;
            let mut next = next_stream_id.lock().await;
            let now = std::time::Instant::now();
            // Light-weight idle-timeout cleanup: remove flows that have been
            // idle for longer than the configured window before creating or
            // reusing a flow for this packet.
            let removed = table.prune_idle(std::time::Duration::from_secs(60), now);
            if removed > 0 {
                debug!(
                    removed,
                    "client tun: pruned idle flows while handling outbound packet"
                );
            }
            let sid = table.get_or_create(flow_key, &mut *next, now);
            debug!(
                stream_id = sid,
                src = ?flow_key.src,
                dst = ?flow_key.dst,
                src_port = flow_key.src_port,
                dst_port = flow_key.dst_port,
                "client tun: flow created or reused for outbound packet"
            );
            sid
        };

        // For Stage 2 MVP, treat entire packet payload (from IP header onwards)
        // as tunnel data for this stream.
        let msg = TunnelMessage::new(MsgType::Data, 1, stream_id, 0, pkt.to_vec());
        let routing = RoutingInfo {
            hop_index: 0,
            route: route.clone(),
        };
        match build_encrypted_packet(&crypto, &routing, msg) {
            Ok(ct) => {
                debug!(
                    stream_id = stream_id,
                    bytes = pkt.len(),
                    "client tun: sending DATA over tunnel"
                );
                if let Err(e) = udp.send(&first_hop, &ct).await {
                    error!(%e, "client tun: failed to send data to relay");
                    continue;
                }
            }
            Err(e) => {
                error!(%e, "client tun: failed to seal message");
                continue;
            }
        }
    }
}

/// Client-side runtime handshake with the exit over UDP via relay(s).
/// Stage 5: handshake messages follow the same routed path as DATA:
/// client -> relay(ы) -> exit and обратно.
async fn perform_handshake(udp: &UdpTransport, route: &Route) -> Result<SessionCrypto> {
    use tokio::time::{sleep, timeout};

    let session_id = 1;
    let handshake_start = Instant::now();
    let route_chain = route
        .hops
        .iter()
        .map(|hop| hop.to_string())
        .collect::<Vec<_>>()
        .join(" -> ");
    emit_client_stage(
        "handshake_started",
        json!({
            "session_id": session_id,
            "route_len": route.len(),
            "route_chain": route_chain.as_str(),
        }),
    );

    let max_attempts = 10u32;
    let first_hop = route
        .first_hop()
        .ok_or_else(|| anyhow::anyhow!("client handshake: empty route"))?;
    for attempt in 1..=max_attempts {
        info!(
            attempt,
            max_attempts, "client handshake: sending HandshakeInit via routed path (hop 0 -> exit)"
        );
        let (init_msg, local_secret, _local_pub) = build_handshake_init(session_id);
        let init_bytes = encode_plaintext(&init_msg)?;
        let routing = RoutingInfo {
            hop_index: 0,
            route: route.clone(),
        };
        let outer = crate::wire::build_handshake_packet(&routing, init_bytes)?;
        udp.send(&first_hop, &outer).await?;

        // Step 1: expect HandshakeChallenge (Stage 3.1)
        let data = match timeout(Duration::from_secs(3), udp.recv()).await {
            Ok(Ok((_from, v))) => v,
            Ok(Err(e)) => {
                error!(%e, "client handshake: transport receive error");
                sleep(Duration::from_millis(500)).await;
                continue;
            }
            Err(_) => {
                debug!("client handshake: timeout waiting for HandshakeChallenge");
                sleep(Duration::from_millis(500)).await;
                continue;
            }
        };
        let (_routing, inner) = crate::wire::parse_routing_header(&data)?;
        let msg = decode(inner)?;
        if msg.header.version != PROTOCOL_VERSION {
            error!(
                got = msg.header.version,
                expected = PROTOCOL_VERSION,
                "client handshake: protocol version mismatch"
            );
            sleep(Duration::from_millis(500)).await;
            continue;
        }
        if msg.header.msg_type != MsgType::HandshakeChallenge {
            debug!(
                msg_type = ?msg.header.msg_type,
                "client handshake: expected HandshakeChallenge, retrying"
            );
            sleep(Duration::from_millis(500)).await;
            continue;
        }
        let challenge: HandshakeChallengePayload = bincode::deserialize(&msg.payload)
            .map_err(|e| anyhow::anyhow!("challenge decode: {}", e))?;

        // Preserve client_nonce from our first init (same payload we sent).
        let first_payload: crate::handshake::HandshakeInitPayload =
            bincode::deserialize(&init_msg.payload)
                .map_err(|e| anyhow::anyhow!("init payload decode: {}", e))?;

        // Step 2: send HandshakeInit with cookie
        let init_with_cookie = build_handshake_init_with_cookie(
            session_id,
            first_payload.client_pubkey,
            first_payload.client_nonce,
            challenge.cookie,
        );
        let init_cookie_bytes = encode_plaintext(&init_with_cookie)?;
        let routing2 = RoutingInfo {
            hop_index: 0,
            route: route.clone(),
        };
        let outer2 = crate::wire::build_handshake_packet(&routing2, init_cookie_bytes)?;
        udp.send(&first_hop, &outer2).await?;

        // Step 3: expect HandshakeAck
        let data2 = match timeout(Duration::from_secs(3), udp.recv()).await {
            Ok(Ok((_from2, v))) => v,
            Ok(Err(e)) => {
                error!(%e, "client handshake: transport receive error waiting for ack");
                sleep(Duration::from_millis(500)).await;
                continue;
            }
            Err(_) => {
                debug!("client handshake: timeout waiting for HandshakeAck");
                sleep(Duration::from_millis(500)).await;
                continue;
            }
        };
        let (_routing2, inner2) = crate::wire::parse_routing_header(&data2)?;
        let msg2 = decode(inner2)?;
        if msg2.header.version != PROTOCOL_VERSION {
            error!("client handshake: protocol version mismatch on ack");
            sleep(Duration::from_millis(500)).await;
            continue;
        }
        if msg2.header.msg_type != MsgType::HandshakeAck {
            debug!(msg_type = ?msg2.header.msg_type, "client handshake: expected HandshakeAck, retrying");
            sleep(Duration::from_millis(500)).await;
            continue;
        }
        let ack: HandshakeAckPayload = bincode::deserialize(&msg2.payload)?;
        let aead_key = derive_session_key_from_ack(&local_secret, &ack);
        info!("client handshake: session established with exit (Stage 3.1 challenge flow)");
        emit_client_stage(
            "session_established",
            json!({
                "session_id": session_id,
                "route_len": route.len(),
                "route_chain": route_chain.as_str(),
                "handshake_ms": handshake_start.elapsed().as_millis() as u64,
                "attempt": attempt,
            }),
        );
        return Ok(SessionCrypto::new(aead_key));
    }

    Err(anyhow::anyhow!(
        "client handshake: failed to establish session after {max_attempts} attempts"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;

    fn test_route() -> Route {
        Route {
            hops: vec![NodeAddr::from(
                "127.0.0.1:30000".parse::<SocketAddr>().unwrap(),
            )],
        }
    }

    #[test]
    fn completed_response_stream_retains_ack_state_for_late_duplicates() {
        let stream_id = 42u32;
        let reliable = Arc::new(Mutex::new(ReliableStream::new()));
        {
            let mut guard = reliable.lock().unwrap();
            let data = StreamFrame {
                stream_id,
                frame_seq: 0,
                ack_seq: 0,
                payload: b"hello".to_vec(),
            };
            let end = StreamFrame {
                stream_id,
                frame_seq: 1,
                ack_seq: 0,
                payload: Vec::new(),
            };

            let (deliver_data, ack_after_data, eos_after_data) = guard.process_incoming(&data);
            assert_eq!(deliver_data, vec![b"hello".to_vec()]);
            assert_eq!(ack_after_data, 1);
            assert!(!eos_after_data);

            let (deliver_end, ack_after_end, eos_after_end) = guard.process_incoming(&end);
            assert!(deliver_end.is_empty());
            assert_eq!(ack_after_end, 2);
            assert!(eos_after_end);
        }

        let completed: CompletedResponseStreamTable = Arc::new(Mutex::new(HashMap::new()));
        remember_completed_response_stream(
            &completed,
            stream_id,
            reliable.clone(),
            test_route(),
            "content_length_satisfied",
        );

        let snapshot =
            snapshot_completed_response_stream(&completed, stream_id).expect("completed stream");
        let late_duplicate = StreamFrame {
            stream_id,
            frame_seq: 0,
            ack_seq: 0,
            payload: b"hello".to_vec(),
        };
        let (deliver_late, ack_after_late, eos_after_late) = {
            let mut guard = snapshot.reliable.lock().unwrap();
            guard.process_incoming(&late_duplicate)
        };

        assert!(
            deliver_late.is_empty(),
            "late duplicate must not be redelivered"
        );
        assert_eq!(
            ack_after_late, 2,
            "completed stream must retain cumulative ACK state for late duplicates"
        );
        assert!(!eos_after_late);

        let (completed_age_ms, completion_reason, late_payload_count) =
            note_completed_response_late_payload(&completed, stream_id)
                .expect("late payload should update completed stream counters");
        assert_eq!(completion_reason, "content_length_satisfied");
        assert_eq!(late_payload_count, 1);
        assert!(completed_age_ms <= COMPLETED_RESPONSE_STREAM_LINGER.as_millis() as u64);
    }
}
