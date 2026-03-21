use crate::addr::NodeAddr;
use crate::ant::{Ant, AntDedup, AntType, Observation};
use crate::build_info::BuildInfo;
use crate::config::ExitConfigCli as ExitArgs;
use crate::discovery;
use crate::discovery::{DiscoveryAdvertisePayload, DiscoveryMessage, DiscoveryQueryPayload};
use crate::handshake::{
    encode_plaintext, handle_handshake_init, HandshakeChallengePayload, HandshakeInitPayload,
};
use crate::handshake_cookie::{generate_cookie, verify_cookie, HandshakeRateLimiter};
use crate::node_config::NodeRole;
use crate::ops::drain;
use crate::protocol::{
    decode, MsgType, ResponseQualityFeedback, StreamFrame, TunnelMessage, PROTOCOL_VERSION,
};
use crate::session::SessionCrypto;
use crate::stage_trace;
use crate::stream_reliable::{AckFrame, ReliableStream};
use crate::transport::{Transport, UdpTransport};
use crate::wire::{build_encrypted_packet, parse_routing_header, RoutingInfo};
use anyhow::Result;
use rand_core::{OsRng, RngCore};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::net::SocketAddr;
use std::net::UdpSocket as StdUdpSocket;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering as AtomicOrdering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;
use tracing::{debug, error, info, warn};

const MIN_EXIT_RESPONSE_WINDOW_FRAMES: usize = 8;
const MAX_EXIT_RESPONSE_WINDOW_FRAMES: usize = 256;
const EXIT_RESPONSE_RETRANSMIT_BASE_MS: u64 = 350;
const EXIT_RESPONSE_RETRANSMIT_SAFETY_MARGIN_MS: u64 = 75;
const EXIT_RESPONSE_RETRANSMIT_MAX_MS: u64 = 1500;

struct ExitStream {
    socket: TcpStream,
}

fn clamp_response_window_frames(frames: usize) -> usize {
    frames.clamp(
        MIN_EXIT_RESPONSE_WINDOW_FRAMES,
        MAX_EXIT_RESPONSE_WINDOW_FRAMES,
    )
}

fn adaptive_response_retransmit_interval(
    summary: crate::stream_reliable::AckLatencySummary,
) -> Duration {
    let mut interval_ms = EXIT_RESPONSE_RETRANSMIT_BASE_MS;
    if let Some(p95_ms) = summary.p95_ms {
        interval_ms =
            interval_ms.max(p95_ms.saturating_add(EXIT_RESPONSE_RETRANSMIT_SAFETY_MARGIN_MS));
    } else if let Some(avg_ms) = summary.avg_ms {
        interval_ms =
            interval_ms.max(avg_ms.saturating_add(EXIT_RESPONSE_RETRANSMIT_SAFETY_MARGIN_MS));
    } else if let Some(p50_ms) = summary.p50_ms {
        interval_ms =
            interval_ms.max(p50_ms.saturating_add(EXIT_RESPONSE_RETRANSMIT_SAFETY_MARGIN_MS));
    }

    Duration::from_millis(interval_ms.min(EXIT_RESPONSE_RETRANSMIT_MAX_MS))
}

/// Minimal HTTP parser helper: try to find Content-Length and header/body split.
fn http_content_length(buf: &[u8]) -> Option<(usize, usize)> {
    // Look for CRLFCRLF as end of headers.
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
    // Search for "Content-Length:" (case-sensitive) in headers.
    let headers = &haystack[..hdr_end];
    let marker = b"Content-Length:";
    let pos = headers.windows(marker.len()).position(|w| w == marker)?;
    let rest = &headers[pos + marker.len()..];
    // Skip spaces.
    let rest = match rest.iter().position(|b| !b.is_ascii_whitespace()) {
        Some(idx) => &rest[idx..],
        None => return None,
    };
    // Read decimal digits.
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

/// Extract the `Host:` value from an HTTP request (best-effort, lightweight).
/// - Returns host without port (e.g. `example.com` from `example.com:443`).
/// - Only supports ASCII hostnames (typical for curl/http libraries).
fn extract_http_host_from_request<'a>(req: &'a [u8]) -> Option<&'a str> {
    let marker = b"Host:";
    let pos = req.windows(marker.len()).position(|w| w == marker)?;
    let mut i = pos + marker.len();
    while i < req.len() && req[i].is_ascii_whitespace() {
        i += 1;
    }
    if i >= req.len() {
        return None;
    }

    // Stop at header line end.
    let end = req[i..]
        .iter()
        .position(|&b| b == b'\r' || b == b'\n')
        .map(|p| i + p)
        .unwrap_or(req.len());
    let host_port = &req[i..end];
    strip_port_from_host_bytes(host_port)
        .and_then(|host_bytes| std::str::from_utf8(host_bytes).ok())
}

/// Extract CONNECT target host (best-effort).
/// e.g. `CONNECT wikipedia.org:443 HTTP/1.1` => `wikipedia.org`
fn extract_connect_host_from_request<'a>(req: &'a [u8]) -> Option<&'a str> {
    if !req.starts_with(b"CONNECT ") {
        return None;
    }
    let start = "CONNECT ".len();
    let end = req[start..]
        .iter()
        .position(|&b| b == b' ')
        .map(|p| start + p)?;
    let host_port = &req[start..end];
    strip_port_from_host_bytes(host_port)
        .and_then(|host_bytes| std::str::from_utf8(host_bytes).ok())
}

/// Minimal TLS ClientHello SNI extractor (best-effort).
/// - Looks for Server Name Indication in an unencrypted ClientHello.
/// - If parsing fails or the buffer is truncated, returns `None`.
fn extract_tls_sni_from_request<'a>(req: &'a [u8]) -> Option<&'a str> {
    // TLS record (ClientHello typically starts with 0x16).
    if req.first().copied()? != 0x16 {
        return None;
    }
    // Need at least: record header (5) + handshake header (4).
    if req.len() < 9 {
        return None;
    }

    // TLS record header: type(1), version(2), length(2).
    let record_len = u16::from_be_bytes([req[3], req[4]]) as usize;
    if req.len() < 5 + record_len {
        return None; // truncated
    }

    // Handshake starts after record header.
    let mut i = 5;
    // Handshake header: type(1) + length(3)
    if req[i] != 0x01 {
        return None; // not ClientHello
    }
    i += 4;
    if req.len() < i + 2 + 32 {
        return None;
    }

    // ClientHello: legacy_version(2) + random(32)
    i += 2 + 32;
    if i >= req.len() {
        return None;
    }
    // session_id
    let sid_len = req[i] as usize;
    i += 1 + sid_len;
    if i + 2 > req.len() {
        return None;
    }
    // cipher_suites
    let cs_len = u16::from_be_bytes([req[i], req[i + 1]]) as usize;
    i += 2 + cs_len;
    if i + 1 > req.len() {
        return None;
    }
    // compression_methods
    let comp_len = req[i] as usize;
    i += 1 + comp_len;
    if i + 2 > req.len() {
        return None;
    }

    // extensions
    let ext_total_len = u16::from_be_bytes([req[i], req[i + 1]]) as usize;
    i += 2;
    let ext_end = i + ext_total_len;
    if ext_end > req.len() {
        return None;
    }

    let mut e = i;
    while e + 4 <= ext_end {
        let ext_type = u16::from_be_bytes([req[e], req[e + 1]]);
        let ext_size = u16::from_be_bytes([req[e + 2], req[e + 3]]) as usize;
        e += 4;
        if e + ext_size > ext_end {
            return None;
        }

        // server_name extension: 0x0000
        if ext_type == 0x0000 {
            if ext_size < 2 {
                return None;
            }
            let mut n = e;
            let list_len = u16::from_be_bytes([req[n], req[n + 1]]) as usize;
            n += 2;
            let list_end = n + list_len;
            if list_end > e + ext_size {
                return None;
            }
            while n + 3 <= list_end {
                let name_type = req[n];
                let name_len = u16::from_be_bytes([req[n + 1], req[n + 2]]) as usize;
                n += 3;
                if n + name_len > list_end {
                    return None;
                }
                // name_type == 0 => host_name
                if name_type == 0 {
                    return std::str::from_utf8(&req[n..n + name_len]).ok();
                }
                n += name_len;
            }
            return None;
        }

        e += ext_size;
    }

    None
}

fn strip_port_from_host_bytes<'a>(host_port: &'a [u8]) -> Option<&'a [u8]> {
    if host_port.is_empty() {
        return None;
    }
    // IPv6 bracket form: [::1]:443 or [::1]
    if host_port.first() == Some(&b'[') {
        let close = host_port.iter().position(|&b| b == b']')?;
        if close <= 1 {
            return None;
        }
        return Some(&host_port[1..close]);
    }

    // If we see a trailing :<digits> segment, strip it.
    if let Some(idx) = host_port.iter().rposition(|&b| b == b':') {
        if idx + 1 < host_port.len() && host_port[idx + 1..].iter().all(|b| b.is_ascii_digit()) {
            return Some(&host_port[..idx]);
        }
    }
    Some(host_port)
}

fn extract_site_from_request<'a>(req: &'a [u8]) -> Option<&'a str> {
    extract_connect_host_from_request(req)
        .or_else(|| extract_http_host_from_request(req))
        .or_else(|| extract_tls_sni_from_request(req))
}

fn fixed_site_from_target_addr(target_addr: &str) -> Option<String> {
    let (host, port_str) = target_addr.rsplit_once(':')?;
    if host.starts_with('[') {
        return None;
    }
    if port_str.parse::<u16>().is_err() {
        return None;
    }
    // If it looks like an IP, we don't have a stable "site".
    if host.parse::<std::net::IpAddr>().is_ok() {
        return None;
    }
    if host.is_empty() {
        return None;
    }
    Some(host.to_string())
}

fn extract_http_status_code(buf: &[u8]) -> Option<u16> {
    // Look for the first status line substring: `HTTP/<x.y> <code> ...`
    let marker = b"HTTP/";
    let pos = buf.windows(marker.len()).position(|w| w == marker)?;
    let mut i = pos + marker.len();

    // Parse version: digits and dots until space.
    while i < buf.len() && buf[i] != b' ' {
        i += 1;
    }
    if i >= buf.len() {
        return None;
    }
    while i < buf.len() && buf[i].is_ascii_whitespace() {
        i += 1;
    }
    if i + 3 > buf.len() {
        return None;
    }

    let d0 = buf[i];
    let d1 = buf[i + 1];
    let d2 = buf[i + 2];
    if !d0.is_ascii_digit() || !d1.is_ascii_digit() || !d2.is_ascii_digit() {
        return None;
    }
    let code = (d0 - b'0') as u16 * 100 + (d1 - b'0') as u16 * 10 + (d2 - b'0') as u16;
    Some(code)
}

fn emit_exit_stage(stage: &str, payload: serde_json::Value) {
    if !stage_trace::enabled() {
        return;
    }
    let mut object = stage_trace::event("exit", stage);
    if let serde_json::Value::Object(fields) = payload {
        object.extend(fields);
    }
    stage_trace::emit(serde_json::Value::Object(object));
}

/// Best-effort detection of the "outbound" IP (useful when bind_ip is `0.0.0.0`).
/// This is only used for logging/validation identity, not transport behavior.
fn best_effort_outbound_ip() -> Option<IpAddr> {
    // UDP connect doesn't send data; it just lets the OS select a local route.
    // If networking is restricted, we fall back to `None`.
    let sock = StdUdpSocket::bind("0.0.0.0:0").ok()?;
    let _ = sock.connect("1.1.1.1:80").ok()?;
    let la = sock.local_addr().ok()?;
    Some(la.ip())
}

pub async fn run_exit(args: ExitArgs) -> Result<()> {
    let udp = Arc::new(UdpTransport::bind(args.listen).await?);
    let response_window_frames = clamp_response_window_frames(args.response_window_frames);
    info!(
        listen_addr = %args.listen,
        target_addr = %args.target_addr,
        response_window_frames,
        "exit starting, binding UDP and resolving target"
    );
    info!(role = "exit", addr = %args.listen, "node ready");

    let fixed_site =
        fixed_site_from_target_addr(&args.target_addr).unwrap_or_else(|| "unknown".to_string());

    let mut target_addrs = tokio::net::lookup_host(&args.target_addr).await?;
    let target_addr: SocketAddr = target_addrs
        .next()
        .ok_or_else(|| anyhow::anyhow!("exit: could not resolve target address"))?;
    info!(%target_addr, "exit resolved target address");

    // Stage 3.1: cookie secret for stateless anti-amplification (no session until cookie verified).
    let mut cookie_secret = [0u8; 32];
    OsRng.fill_bytes(&mut cookie_secret);

    // Stage 3.1: rate limit handshake attempts per IP (max 50/sec).
    let rate_limiter = Arc::new(Mutex::new(HandshakeRateLimiter::new(50)));

    // Session state: we establish a single end-to-end session with the client
    // via cookie challenge then x25519 handshake, then AEAD-protected traffic.
    let mut session_crypto: Option<SessionCrypto> = None;
    let mut session_route: Option<crate::route::Route> = None;
    // UDP address of the relay (and thus path back to the client) for this session.
    let mut session_peer: Option<NodeAddr> = None;

    // Stage 7: ant dedup (exit only bounces Echo ants).
    let ant_dedup: Arc<Mutex<AntDedup>> =
        Arc::new(Mutex::new(AntDedup::new(256, Duration::from_secs(30))));
    let exit_ip = if args.listen.ip().is_unspecified() {
        best_effort_outbound_ip().unwrap_or(args.listen.ip())
    } else {
        args.listen.ip()
    };
    // Keep prefix consistent with existing NodeAddr Display ("Udp://ip:port"),
    // because validation tooling/ops checks expect it.
    let exit_identity = {
        let node_addr = NodeAddr {
            ip: exit_ip,
            port: args.listen.port(),
            protocol: crate::addr::Protocol::Udp,
        };
        node_addr.to_string()
    };
    let self_node_for_tcp = NodeAddr {
        ip: exit_ip,
        port: args.listen.port(),
        protocol: crate::addr::Protocol::Udp,
    };
    // Keep compatibility with existing code paths that expect `self_node`.
    let self_node = self_node_for_tcp.clone();

    // Stage 9.1: minimal discovery store + best-effort self advertisement (control-plane only).
    let discovery_store = Arc::new(Mutex::new(discovery::new_store(args.discovery_max_entries)));
    let self_adv = {
        let bi = BuildInfo::current();
        let self_id = discovery::node_id_from_addr(&self_node_for_tcp);
        let ttl_ms = args.discovery_advertise_ttl_sec.saturating_mul(1000);
        discovery::make_self_advertisement(
            self_id,
            NodeRole::Exit,
            self_node_for_tcp.clone(),
            bi.short(),
            ttl_ms,
        )
    };
    {
        let now = crate::ant::now_ms();
        let mut store = discovery_store.lock().unwrap();
        store.purge_expired(now);
        store.insert(self_adv.clone());
    }
    debug!(
        event = "discovery_self_advertise",
        role = "exit",
        addr = %self_node,
        enabled = args.discovery_enabled,
        "self advertisement cached"
    );

    // Best-effort publish/query to bootstrap peers. Must never affect dataplane startup.
    if args.discovery_enabled {
        let bootstrap_peers: Vec<NodeAddr> = args
            .discovery_bootstrap_peers
            .clone()
            .unwrap_or_default()
            .into_iter()
            .map(NodeAddr::from)
            .collect();

        for peer in bootstrap_peers {
            if peer.as_socket_addr() == self_node.as_socket_addr() {
                continue;
            }

            let adv_payload = DiscoveryAdvertisePayload {
                advertisement: self_adv.clone(),
            };

            if let Ok(payload_bytes) = bincode::serialize(&adv_payload) {
                let msg = TunnelMessage::new(MsgType::DiscoveryAdvertise, 1, 0, 0, payload_bytes);
                if let Ok(packet) = discovery::build_plaintext_packet(&peer, &msg) {
                    if udp.send(&peer, &packet).await.is_ok() {
                        debug!(
                            event = "discovery_advertise_sent",
                            peer = %peer,
                            "sent self discovery advertisement"
                        );
                    }
                }
            }

            if args.discovery_query_on_start {
                let query = DiscoveryQueryPayload {
                    max_results: 16,
                    role: None,
                    protocol: Some(crate::addr::Protocol::Udp),
                };
                if let Ok(payload_bytes) = bincode::serialize(&query) {
                    let qmsg = TunnelMessage::new(MsgType::DiscoveryQuery, 1, 0, 0, payload_bytes);
                    if let Ok(packet) = discovery::build_plaintext_packet(&peer, &qmsg) {
                        if udp.send(&peer, &packet).await.is_ok() {
                            let mut store = discovery_store.lock().unwrap();
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
    }

    // For simplicity keep a map stream_id -> TCP socket handle via channels
    let (tx_map, mut rx_ctrl) = mpsc::unbounded_channel::<(u32, Vec<u8>, NodeAddr)>(); // (stream_id, data, peer_addr)

    // Shared session crypto for the TCP worker, populated after first data packet.
    let shared_crypto: Arc<Mutex<Option<SessionCrypto>>> = Arc::new(Mutex::new(None));
    let shared_route: Arc<Mutex<Option<crate::route::Route>>> = Arc::new(Mutex::new(None));
    // Shared peer address for reverse traffic (exit -> relay -> client).
    let shared_peer: Arc<Mutex<Option<NodeAddr>>> = Arc::new(Mutex::new(None));

    // Stage 4: accumulate request chunks per stream until client-initiated
    // end-of-request marker, then flush exactly once to the TCP worker.
    let request_buffer: Arc<Mutex<HashMap<u32, Vec<u8>>>> = Arc::new(Mutex::new(HashMap::new()));
    let completed_requests: Arc<Mutex<HashSet<u32>>> = Arc::new(Mutex::new(HashSet::new()));
    // NOTE: request-side reliability is handled by `ReliableStream::process_incoming`
    // (reordering/dup handling) + ACKs, so we do not keep a separate seen set.

    // Task: read from TCP sockets and send back over UDP
    let udp_for_task = udp.clone();
    let shared_crypto_for_task = shared_crypto.clone();
    let shared_route_for_task = shared_route.clone();
    let shared_peer_for_task = shared_peer.clone();
    // Per-stream reliable state (both directions).
    let reliable_streams: Arc<Mutex<HashMap<u32, ReliableStream>>> =
        Arc::new(Mutex::new(HashMap::new()));

    let reliable_streams_for_task = reliable_streams.clone();
    let request_buffer_for_task = request_buffer.clone();
    let _completed_requests_for_task = completed_requests.clone();
    let response_window_frames_for_task = response_window_frames;

    // Task: handle TCP request/response per stream.
    tokio::spawn(async move {
        let mut streams: HashMap<u32, ExitStream> = HashMap::new();
        // Accumulate payload metrics across multiple downlink bursts that can share the
        // same tunnel stream_id (e.g. TLS continuation when we keep the TCP socket alive).
        //
        // Why: VALIDATION_ARTIFACT is emitted at the end of each downlink burst today,
        // but ReliableStream state can be reset while the TCP connection stays open.
        // Without an external accumulator, resp_bytes/frames_sent only reflect the last burst.
        struct ResponseTotals {
            first_response_start: Instant,
            total_payload_bytes: u64,
            total_payload_frames: u64,
            bursts_count: u64,
        }
        let mut response_totals: HashMap<u32, ResponseTotals> = HashMap::new();
        let mut final_emitted: HashSet<u32> = HashSet::new();
        info!("exit tcp worker task started");
        while let Some((stream_id, data, _peer_from_main)) = rx_ctrl.recv().await {
            // Determine where to send reverse traffic for this session.
            let peer = match shared_peer_for_task.lock().unwrap().clone() {
                Some(p) => p,
                None => {
                    error!(
                        stream_id,
                        "exit tcp worker has no session peer; dropping message"
                    );
                    continue;
                }
            };
            // data here is a command:
            // - empty => end-of-request marker (flush buffered request if any; otherwise treat as CloseStream)
            // - non-empty => legacy path (write provided bytes as request)
            let full_request = if data.is_empty() {
                let maybe_buf = {
                    let mut buf_map = request_buffer_for_task.lock().unwrap();
                    buf_map.remove(&stream_id)
                };
                match maybe_buf {
                    Some(buf) if !buf.is_empty() => buf,
                    _ => {
                        if let Some(mut st) = streams.remove(&stream_id) {
                            if let Err(e) = st.socket.shutdown().await {
                                error!(stream_id, %e, "exit tcp shutdown error");
                            }
                        }
                        // Stream is closed without a full request/response cycle.
                        // Deterministic final artifact for validation: CloseStream (no continuation expected).
                        if !final_emitted.contains(&stream_id) {
                            if let Some(entry) = response_totals.get(&stream_id) {
                                let dur_ms =
                                    entry.first_response_start.elapsed().as_millis() as u64;
                                let thr_bps_total = if dur_ms > 0 {
                                    (entry.total_payload_bytes as u128 * 1000u128) / dur_ms as u128
                                } else {
                                    0
                                };

                                let route_hops = shared_route_for_task
                                    .lock()
                                    .unwrap()
                                    .as_ref()
                                    .map(|r| r.len())
                                    .unwrap_or(0);

                                // Best-effort placeholders for fields that are only known
                                // in the downlink read loop (kept deterministic for extractors).
                                info!(
                                    "VALIDATION_ARTIFACT_FINAL,stream_id={},site={},exit={},route={},resp_bytes={},frames_sent={},bursts_count={},avg_inflight={:.3},max_inflight={},time_at_inflight_1_ms={},retransmit_rate={:.6},stream_duration_ms={},throughput_bps={},ack_latency_ms_avg={},http_code={},is_final=1,mode=overlay",
                                    stream_id,
                                    fixed_site.clone(),
                                    exit_identity,
                                    route_hops,
                                    entry.total_payload_bytes,
                                    entry.total_payload_frames,
                                    entry.bursts_count,
                                    0.0f64,
                                    0usize,
                                    0u64,
                                    0.0f64,
                                    dur_ms,
                                    thr_bps_total as u64,
                                    0u64,
                                    0u16
                                );
                                debug!(
                                    stream_id,
                                    resp_bytes_total = entry.total_payload_bytes,
                                    frames_sent_total = entry.total_payload_frames,
                                    bursts_count = entry.bursts_count,
                                    "FINAL artifact emitted for stream_id (CloseStream)"
                                );
                                final_emitted.insert(stream_id);
                            }
                        }
                        response_totals.remove(&stream_id);
                        debug!(
                            stream_id,
                            "exit received CloseStream (no buffered request), tcp socket (if any) closed"
                        );
                        continue;
                    }
                }
            } else {
                let maybe_buf = {
                    let mut buf_map = request_buffer_for_task.lock().unwrap();
                    buf_map.remove(&stream_id)
                };
                maybe_buf.unwrap_or_else(|| data.clone())
            };

            let site = extract_site_from_request(&full_request)
                .map(|s| s.to_string())
                .unwrap_or_else(|| fixed_site.clone());

            debug!(
                stream_id,
                bytes = full_request.len(),
                peer = %peer,
                "exit forwarding to target (connect or reuse)"
            );
            let mut st = if let Some(st) = streams.remove(&stream_id) {
                debug!(stream_id, "exit reusing existing tcp stream to target");
                st
            } else {
                let connect_start = Instant::now();
                emit_exit_stage(
                    "target_connect_started",
                    json!({
                        "stream_id": stream_id,
                        "site": site.as_str(),
                        "route_len": shared_route_for_task
                            .lock()
                            .unwrap()
                            .as_ref()
                            .map(|route| route.len())
                            .unwrap_or(0),
                        "peer": peer.to_string(),
                        "target_addr": target_addr.to_string(),
                    }),
                );
                // Bounded-time connect to target
                let connect_res =
                    timeout(Duration::from_secs(1), TcpStream::connect(target_addr)).await;

                match connect_res {
                    Ok(Ok(s)) => {
                        debug!(stream_id, %target_addr, "exit connected to target");
                        emit_exit_stage(
                            "target_connect_completed",
                            json!({
                                "stream_id": stream_id,
                                "site": site.as_str(),
                                "route_len": shared_route_for_task
                                    .lock()
                                    .unwrap()
                                    .as_ref()
                                    .map(|route| route.len())
                                    .unwrap_or(0),
                                "peer": peer.to_string(),
                                "target_addr": target_addr.to_string(),
                                "target_connect_ms": connect_start.elapsed().as_millis() as u64,
                            }),
                        );
                        ExitStream { socket: s }
                    }
                    Ok(Err(e)) => {
                        error!(stream_id, %e, %target_addr, "exit failed to connect to target");
                        let err_msg = TunnelMessage::new(
                            MsgType::Error,
                            1,
                            stream_id,
                            0,
                            b"target connect failed".to_vec(),
                        );
                        let maybe_crypto = shared_crypto_for_task.lock().unwrap().clone();
                        let maybe_route = shared_route_for_task.lock().unwrap().clone();
                        if let (Some(crypto_for_task), Some(route)) = (maybe_crypto, maybe_route) {
                            let routing = RoutingInfo {
                                hop_index: (route.len().saturating_sub(1)) as u8,
                                route,
                            };
                            if let Ok(ct) =
                                build_encrypted_packet(&crypto_for_task, &routing, err_msg)
                            {
                                debug!(
                                    stream_id,
                                    peer = %peer,
                                    "exit sending Error (target connect failed) to client"
                                );
                                let _ = udp_for_task.send(&peer, &ct).await;
                            }
                        } else {
                            error!(
                                    stream_id,
                                    "exit has no established session crypto/route for target connect error"
                                );
                        }
                        continue;
                    }
                    Err(_) => {
                        // timeout
                        error!(stream_id, %target_addr, "exit connect to target timed out");
                        let err_msg = TunnelMessage::new(
                            MsgType::Error,
                            1,
                            stream_id,
                            0,
                            b"target connect timeout".to_vec(),
                        );
                        let maybe_crypto = shared_crypto_for_task.lock().unwrap().clone();
                        let maybe_route = shared_route_for_task.lock().unwrap().clone();
                        if let (Some(crypto_for_task), Some(route)) = (maybe_crypto, maybe_route) {
                            let routing = RoutingInfo {
                                hop_index: (route.len().saturating_sub(1)) as u8,
                                route,
                            };
                            if let Ok(ct) =
                                build_encrypted_packet(&crypto_for_task, &routing, err_msg)
                            {
                                debug!(
                                    stream_id,
                                    peer = %peer,
                                    "exit sending Error (target connect timeout) to client"
                                );
                                let _ = udp_for_task.send(&peer, &ct).await;
                            }
                        } else {
                            error!(
                                    stream_id,
                                    "exit has no established session crypto/route for target connect timeout"
                                );
                        }
                        continue;
                    }
                }
            };
            debug!(
                stream_id,
                bytes = full_request.len(),
                "exit writing request bytes to target"
            );
            if let Err(e) = st.socket.write_all(&full_request).await {
                error!(stream_id, %e, "exit failed to write to target");
                let err_msg = TunnelMessage::new(
                    MsgType::Error,
                    1,
                    stream_id,
                    0,
                    b"target write failed".to_vec(),
                );
                let maybe_crypto = shared_crypto_for_task.lock().unwrap().clone();
                let maybe_route = shared_route_for_task.lock().unwrap().clone();
                if let (Some(crypto_for_task), Some(route)) = (maybe_crypto, maybe_route) {
                    let routing = RoutingInfo {
                        hop_index: (route.len().saturating_sub(1)) as u8,
                        route,
                    };
                    if let Ok(ct) = build_encrypted_packet(&crypto_for_task, &routing, err_msg) {
                        debug!(
                            stream_id,
                            peer = %peer,
                            "exit sending Error (target write failed) to client"
                        );
                        let _ = udp_for_task.send(&peer, &ct).await;
                    }
                } else {
                    error!(
                        stream_id,
                        "exit has no established session crypto/route for target write error"
                    );
                }
                continue;
            }
            // For plain HTTP-style request/response targets, shut down the write
            // half so the target can observe EOF and start producing a response.
            // For TLS (client request typically starts with 0x16), we must keep the
            // connection full-duplex, so we don't shutdown the write side.
            let keep_full_duplex = full_request.first().copied() == Some(0x16);
            if !keep_full_duplex {
                if let Err(e) = st.socket.shutdown().await {
                    error!(stream_id, %e, "exit: target write shutdown failed");
                }
            }

            // After write handling, read from the target with an idle-timeout so
            // we can support long-lived connections (TLS, keep-alive) without
            // buffering the entire response body.
            let mut tmp_buf = vec![0u8; 8192];
            // Stage 9.1b: continuous read->frame send (no full response buffering).
            const CHUNK: usize = 1000;
            let window_frames = response_window_frames_for_task;

            // Read timing controls: we stop when the target goes quiet for a while.
            // This preserves long-lived TLS/HTTP semantics without needing full-body buffering.
            let max_wait = Duration::from_secs(120);
            let idle_after_data = Duration::from_millis(5000);

            // Stream staging: bytes read from the TCP socket that haven't been sent as full frames.
            let mut pending: Vec<u8> = Vec::new();
            let mut pending_cursor: usize = 0;
            let mut seen_any = false;
            let mut connection_alive = true;
            let mut first_target_byte_ms: Option<u64> = None;

            // Metrics for this downlink burst.
            let response_start = Instant::now();
            let mut sent_payload_bytes: usize = 0;
            let mut sent_frames: usize = 0;
            let mut max_inflight: usize = 0;
            let mut http_code: Option<u16> = None;
            let mut first_overlay_send_ms: Option<u64> = None;
            let mut window_wait_events: u64 = 0;
            let mut window_wait_total_ms: u64 = 0;
            let mut window_wait_max_ms: u64 = 0;

            // Stage 9.1b observability:
            // - sample inflight every ~150ms while this stream is active
            // - keep rolling stats to detect stop-and-wait regressions
            // - compute moving throughput (bytes over last ~1s)
            #[derive(Clone, Copy, Debug, Default)]
            struct InflightSampleStats {
                sample_count: u64,
                inflight_sum: u64,
                max_inflight: u64,
                inflight1_samples: u64,
            }

            const SAMPLE_INTERVAL_MS: u64 = 150;
            let bytes_sent_atomic = Arc::new(AtomicU64::new(0));
            let pending_bytes_atomic = Arc::new(AtomicU64::new(0));
            let done_sampling = Arc::new(AtomicBool::new(false));
            let (stats_tx, stats_rx) = oneshot::channel::<InflightSampleStats>();

            {
                let reliable_streams_for_sampling = reliable_streams_for_task.clone();
                let done_sampling = done_sampling.clone();
                let bytes_sent_atomic = bytes_sent_atomic.clone();
                let pending_bytes_atomic = pending_bytes_atomic.clone();
                let stream_id = stream_id;

                tokio::spawn(async move {
                    let mut stats = InflightSampleStats::default();
                    let mut last_tp_log = Instant::now();
                    let mut tp_window_start = Instant::now();
                    let mut last_tp_bytes = bytes_sent_atomic.load(AtomicOrdering::Relaxed);

                    loop {
                        if done_sampling.load(AtomicOrdering::Relaxed) {
                            break;
                        }
                        tokio::time::sleep(Duration::from_millis(SAMPLE_INTERVAL_MS)).await;

                        let (inflight, reorder_depth, ack_latency_avg_ms) = {
                            let map = reliable_streams_for_sampling.lock().unwrap();
                            if let Some(rs) = map.get(&stream_id) {
                                (
                                    rs.inflight() as u64,
                                    rs.recv_buffer_len() as u64,
                                    rs.ack_latency_avg_ms(),
                                )
                            } else {
                                (0u64, 0u64, None)
                            }
                        };

                        stats.sample_count = stats.sample_count.saturating_add(1);
                        stats.inflight_sum = stats.inflight_sum.saturating_add(inflight);
                        if inflight > stats.max_inflight {
                            stats.max_inflight = inflight;
                        }
                        if inflight == 1 {
                            stats.inflight1_samples = stats.inflight1_samples.saturating_add(1);
                        }

                        // Once per ~1s, log a rolling snapshot (cheap + useful).
                        if last_tp_log.elapsed() >= Duration::from_secs(1) {
                            let now = Instant::now();
                            let bytes_now = bytes_sent_atomic.load(AtomicOrdering::Relaxed);
                            let pending_bytes = pending_bytes_atomic.load(AtomicOrdering::Relaxed);

                            let dt_ms = now.duration_since(tp_window_start).as_millis();
                            let delta_bytes = bytes_now.saturating_sub(last_tp_bytes);
                            let throughput_bps = if dt_ms > 0 {
                                (delta_bytes as u128 * 1000u128 / dt_ms as u128) as u64
                            } else {
                                0
                            };

                            let avg_inflight = if stats.sample_count > 0 {
                                stats.inflight_sum as f64 / stats.sample_count as f64
                            } else {
                                0.0
                            };

                            debug!(
                                stream_id,
                                frames_in_flight = inflight,
                                avg_inflight = avg_inflight,
                                max_inflight = stats.max_inflight,
                                time_at_inflight_1_ms =
                                    stats.inflight1_samples * SAMPLE_INTERVAL_MS,
                                pending_bytes = pending_bytes,
                                reorder_buffer_depth = reorder_depth,
                                moving_throughput_bps = throughput_bps,
                                ack_latency_ms_avg = ?ack_latency_avg_ms,
                                "exit stream inflight sampling"
                            );

                            last_tp_log = now;
                            tp_window_start = now;
                            last_tp_bytes = bytes_now;
                        }
                    }

                    let _ = stats_tx.send(stats);
                });
            }

            let maybe_crypto = shared_crypto_for_task.lock().unwrap().clone();
            let maybe_route = shared_route_for_task.lock().unwrap().clone();
            let (Some(crypto_for_task), Some(route)) = (maybe_crypto, maybe_route) else {
                error!(
                    stream_id,
                    "exit has no established session crypto/route for response"
                );
                continue;
            };
            let routing = RoutingInfo {
                hop_index: (route.len().saturating_sub(1)) as u8,
                route,
            };

            // Read loop: keep reading from the target socket and send tunnel frames while we
            // still make progress (and while inflight stays below the configured response window).
            let _read_timeout_res = timeout(max_wait, async {
                loop {
                    let idle = if !seen_any { max_wait } else { idle_after_data };
                    match timeout(idle, st.socket.read(&mut tmp_buf)).await {
                        Ok(Ok(0)) => {
                            connection_alive = false; // target closed (EOF)
                            break;
                        }
                        Ok(Ok(n)) => {
                            if n == 0 {
                                connection_alive = false;
                                break;
                            }
                            seen_any = true;
                            if first_target_byte_ms.is_none() {
                                let since_response_start = response_start.elapsed().as_millis() as u64;
                                first_target_byte_ms = Some(since_response_start);
                                emit_exit_stage(
                                    "first_target_byte",
                                    json!({
                                        "stream_id": stream_id,
                                        "site": site.as_str(),
                                        "route_len": routing.route.len(),
                                        "peer": peer.to_string(),
                                        "target_addr": target_addr.to_string(),
                                        "since_response_start_ms": since_response_start,
                                        "bytes_read": n,
                                    }),
                                );
                            }
                            pending.extend_from_slice(&tmp_buf[..n]);
                            if http_code.is_none() {
                                // Scan only a small prefix; avoid repeated work after we found it.
                                let scan_end = pending.len().min(1024);
                                http_code =
                                    extract_http_status_code(&pending[..scan_end]);
                            }
                            pending_bytes_atomic.store(
                                pending.len().saturating_sub(pending_cursor) as u64,
                                AtomicOrdering::Relaxed,
                            );

                            // Send frames while we have enough bytes to keep
                            // the client from timing out on small/segmented bodies.
                            // For large responses this quickly reaches CHUNK and stays efficient.
                            const MIN_FLUSH: usize = 1;
                            while pending.len().saturating_sub(pending_cursor) >= MIN_FLUSH {
                                let available =
                                    pending.len().saturating_sub(pending_cursor);
                                let take = available.min(CHUNK);
                                let chunk = pending[pending_cursor..pending_cursor + take].to_vec();
                                pending_cursor += take;
                                if pending_cursor >= 64 * 1024 {
                                    pending.drain(0..pending_cursor);
                                    pending_cursor = 0;
                                    pending_bytes_atomic.store(
                                        pending.len() as u64,
                                        AtomicOrdering::Relaxed,
                                    );
                                }

                                let chunk_len = chunk.len();

                                // Sliding window cap: don't let in-flight response frames
                                // grow beyond the configured response window.
                                let window_wait_started = Instant::now();
                                let mut waited_for_window = false;
                                loop {
                                    let inflight = {
                                        let map = reliable_streams_for_task.lock().unwrap();
                                        map.get(&stream_id)
                                            .map(|rs| rs.inflight())
                                            .unwrap_or(0)
                                    };
                                    if inflight > max_inflight {
                                        max_inflight = inflight;
                                    }
                                    if inflight < window_frames {
                                        break;
                                    }
                                    waited_for_window = true;
                                    tokio::time::sleep(Duration::from_millis(2)).await;
                                }
                                if waited_for_window {
                                    let waited_ms =
                                        window_wait_started.elapsed().as_millis() as u64;
                                    window_wait_events = window_wait_events.saturating_add(1);
                                    window_wait_total_ms =
                                        window_wait_total_ms.saturating_add(waited_ms);
                                    window_wait_max_ms = window_wait_max_ms.max(waited_ms);
                                }

                                let frame = {
                                    let mut map = reliable_streams_for_task.lock().unwrap();
                                    let rs = map
                                        .entry(stream_id)
                                        .or_insert_with(ReliableStream::new);
                                    rs.build_outgoing_frame(stream_id, chunk)
                                };

                                let frame_bytes = match bincode::serialize(&frame) {
                                    Ok(b) => b,
                                    Err(e) => {
                                        error!(
                                            stream_id,
                                            %e,
                                            "exit failed to serialize StreamFrame"
                                        );
                                        continue;
                                    }
                                };

                                // IMPORTANT: do NOT resend the same ciphertext — crypto anti-replay will reject it.
                                // Reseal the same StreamFrame bytes to get a new crypto seq/nonce.
                                let mut ok = false;
                                let msg2 = TunnelMessage::new(
                                    MsgType::Data,
                                    1,
                                    stream_id,
                                    0,
                                    frame_bytes.clone(),
                                );
                                let ct2 = match build_encrypted_packet(
                                    &crypto_for_task,
                                    &routing,
                                    msg2,
                                ) {
                                    Ok(ct) => ct,
                                    Err(e) => {
                                        error!(
                                            stream_id,
                                            %e,
                                            "exit failed to seal response chunk"
                                        );
                                        continue;
                                    }
                                };

                                match udp_for_task.send(&peer, &ct2).await {
                                    Ok(_) => {
                                        ok = true;
                                        if first_overlay_send_ms.is_none() && chunk_len > 0 {
                                            let since_response_start =
                                                response_start.elapsed().as_millis() as u64;
                                            let since_first_target_byte = first_target_byte_ms
                                                .map(|target_ms| {
                                                    since_response_start.saturating_sub(target_ms)
                                                })
                                                .unwrap_or(since_response_start);
                                            first_overlay_send_ms = Some(since_response_start);
                                            emit_exit_stage(
                                                "first_overlay_send",
                                                json!({
                                                    "stream_id": stream_id,
                                                    "site": site.as_str(),
                                                    "route_len": routing.route.len(),
                                                    "peer": peer.to_string(),
                                                    "target_addr": target_addr.to_string(),
                                                    "since_response_start_ms": since_response_start,
                                                    "since_first_target_byte_ms": since_first_target_byte,
                                                    "chunk_bytes": chunk_len,
                                                    "frame_seq": frame.frame_seq,
                                                    "window_frames": window_frames,
                                                }),
                                            );
                                        }
                                        debug!(
                                            stream_id,
                                            ct_len = ct2.len(),
                                            peer = %peer,
                                            "exit sent response DATA chunk"
                                        );
                                    }
                                    Err(e) => {
                                        error!(
                                            stream_id,
                                            %e,
                                            "exit failed to send response chunk"
                                        );
                                    }
                                }
                                if ok {
                                    sent_payload_bytes += chunk_len;
                                    bytes_sent_atomic
                                        .fetch_add(chunk_len as u64, AtomicOrdering::Relaxed);
                                    sent_frames += 1;
                                }
                            }
                        }
                        Ok(Err(_)) => {
                            connection_alive = false;
                            break;
                        }
                        Err(_) => {
                            // Idle timeout: target went quiet, end-of-response marker.
                            connection_alive = true;
                            break;
                        }
                    }
                }
            })
            .await;

            // Flush any remaining bytes (less than CHUNK).
            while pending.len().saturating_sub(pending_cursor) > 0 {
                let remaining = pending.len().saturating_sub(pending_cursor);
                let take = remaining.min(CHUNK);
                let chunk = pending[pending_cursor..pending_cursor + take].to_vec();
                let chunk_len = chunk.len();
                pending_cursor += take;

                if pending_cursor >= 64 * 1024 {
                    pending.drain(0..pending_cursor);
                    pending_cursor = 0;
                    pending_bytes_atomic.store(pending.len() as u64, AtomicOrdering::Relaxed);
                }

                let window_wait_started = Instant::now();
                let mut waited_for_window = false;
                loop {
                    let inflight = {
                        let map = reliable_streams_for_task.lock().unwrap();
                        map.get(&stream_id).map(|rs| rs.inflight()).unwrap_or(0)
                    };
                    if inflight > max_inflight {
                        max_inflight = inflight;
                    }
                    if inflight < window_frames {
                        break;
                    }
                    waited_for_window = true;
                    tokio::time::sleep(Duration::from_millis(2)).await;
                }
                if waited_for_window {
                    let waited_ms = window_wait_started.elapsed().as_millis() as u64;
                    window_wait_events = window_wait_events.saturating_add(1);
                    window_wait_total_ms = window_wait_total_ms.saturating_add(waited_ms);
                    window_wait_max_ms = window_wait_max_ms.max(waited_ms);
                }

                let frame = {
                    let mut map = reliable_streams_for_task.lock().unwrap();
                    let rs = map.entry(stream_id).or_insert_with(ReliableStream::new);
                    rs.build_outgoing_frame(stream_id, chunk)
                };

                let frame_bytes = match bincode::serialize(&frame) {
                    Ok(b) => b,
                    Err(e) => {
                        error!(stream_id, %e, "exit failed to serialize StreamFrame (tail)");
                        continue;
                    }
                };

                let mut ok = false;
                let msg2 = TunnelMessage::new(MsgType::Data, 1, stream_id, 0, frame_bytes.clone());
                let ct2 = match build_encrypted_packet(&crypto_for_task, &routing, msg2) {
                    Ok(ct) => ct,
                    Err(e) => {
                        error!(stream_id, %e, "exit failed to seal response tail chunk");
                        continue;
                    }
                };
                match udp_for_task.send(&peer, &ct2).await {
                    Ok(_) => {
                        ok = true;
                        if first_overlay_send_ms.is_none() && chunk_len > 0 {
                            let since_response_start = response_start.elapsed().as_millis() as u64;
                            let since_first_target_byte = first_target_byte_ms
                                .map(|target_ms| since_response_start.saturating_sub(target_ms))
                                .unwrap_or(since_response_start);
                            first_overlay_send_ms = Some(since_response_start);
                            emit_exit_stage(
                                "first_overlay_send",
                                json!({
                                    "stream_id": stream_id,
                                    "site": site.as_str(),
                                    "route_len": routing.route.len(),
                                    "peer": peer.to_string(),
                                    "target_addr": target_addr.to_string(),
                                    "since_response_start_ms": since_response_start,
                                    "since_first_target_byte_ms": since_first_target_byte,
                                    "chunk_bytes": chunk_len,
                                    "frame_seq": frame.frame_seq,
                                    "window_frames": window_frames,
                                }),
                            );
                        }
                        debug!(
                            stream_id,
                            ct_len = ct2.len(),
                            peer = %peer,
                            "exit sent response tail DATA chunk"
                        );
                    }
                    Err(e) => {
                        error!(stream_id, %e, "exit failed to send response tail chunk");
                    }
                }
                if ok {
                    sent_payload_bytes += chunk_len;
                    bytes_sent_atomic.fetch_add(chunk_len as u64, AtomicOrdering::Relaxed);
                    sent_frames += 1;
                }
            }

            // End-of-response marker so the client stops waiting for more chunks.
            let end_frame = {
                let mut map = reliable_streams_for_task.lock().unwrap();
                let rs = map.entry(stream_id).or_insert_with(ReliableStream::new);
                rs.build_outgoing_frame(stream_id, vec![])
            };
            if let Ok(frame_bytes) = bincode::serialize(&end_frame) {
                for _attempt in 1..=3u8 {
                    let msg2 =
                        TunnelMessage::new(MsgType::Data, 1, stream_id, 0, frame_bytes.clone());
                    if let Ok(ct2) = build_encrypted_packet(&crypto_for_task, &routing, msg2) {
                        let _ = udp_for_task.send(&peer, &ct2).await;
                    }
                }
            }

            // Stop inflight sampling and compute rolling stats.
            done_sampling.store(true, AtomicOrdering::Relaxed);
            let sample_stats = stats_rx.await.unwrap_or_default();
            let avg_inflight = if sample_stats.sample_count > 0 {
                sample_stats.inflight_sum as f64 / sample_stats.sample_count as f64
            } else {
                0.0
            };
            let time_at_inflight_1_ms = sample_stats
                .inflight1_samples
                .saturating_mul(SAMPLE_INTERVAL_MS);
            max_inflight = sample_stats.max_inflight as usize;

            let stream_duration_ms = response_start.elapsed().as_millis() as u64;
            let effective_throughput = if stream_duration_ms > 0 {
                (sent_payload_bytes as u128 * 1000u128) / stream_duration_ms as u128
            } else {
                0
            };

            let (
                frames_sent_total,
                frames_acked_total,
                total_retransmits,
                ack_latency_avg_ms,
                ack_latency_min_ms,
                ack_latency_p50_ms,
                ack_latency_p95_ms,
                ack_latency_max_ms,
            ) = {
                let map = reliable_streams_for_task.lock().unwrap();
                if let Some(rs) = map.get(&stream_id) {
                    let (fs, fa, tr) = rs.counters_snapshot();
                    let ack_summary = rs.ack_latency_summary();
                    (
                        fs,
                        fa,
                        tr,
                        ack_summary.avg_ms,
                        ack_summary.min_ms,
                        ack_summary.p50_ms,
                        ack_summary.p95_ms,
                        ack_summary.max_ms,
                    )
                } else {
                    (0u64, 0u64, 0u64, None, None, None, None, None)
                }
            };

            let retransmit_rate = if frames_sent_total > 0 {
                total_retransmits as f64 / frames_sent_total as f64
            } else {
                0.0
            };

            // Update cumulative payload metrics for this tunnel stream_id.
            // resp_bytes/frames_sent in VALIDATION_ARTIFACT should refer to DATA payload
            // only (excluding the empty end-of-response marker frame).
            let (
                sent_payload_bytes_total_u64,
                sent_frames_payload_total_u64,
                stream_duration_ms_total,
                effective_throughput_total,
            ) = {
                let entry = response_totals
                    .entry(stream_id)
                    .or_insert_with(|| ResponseTotals {
                        first_response_start: response_start,
                        total_payload_bytes: 0,
                        total_payload_frames: 0,
                        bursts_count: 0,
                    });
                entry.total_payload_bytes = entry
                    .total_payload_bytes
                    .saturating_add(sent_payload_bytes as u64);
                entry.total_payload_frames = entry
                    .total_payload_frames
                    .saturating_add(sent_frames as u64);
                entry.bursts_count = entry.bursts_count.saturating_add(1);

                let dur_ms = entry.first_response_start.elapsed().as_millis() as u64;
                let thr_bps_total = if dur_ms > 0 {
                    (entry.total_payload_bytes as u128 * 1000u128) / dur_ms as u128
                } else {
                    0
                };

                (
                    entry.total_payload_bytes,
                    entry.total_payload_frames,
                    dur_ms,
                    thr_bps_total,
                )
            };

            // Light sanity checks for validation runs.
            // Triggered only at the end of the current downlink burst (where we emit VALIDATION_ARTIFACT).
            let is_trivial_site = matches!(
                site.as_str(),
                "example.com" | "example.org" | "example.net" | "example"
            );
            if (!is_trivial_site && sent_payload_bytes_total_u64 < 1000)
                || sent_frames_payload_total_u64 < 5
            {
                warn!(
                    stream_id,
                    site = %site,
                    resp_bytes_total = sent_payload_bytes_total_u64,
                    frames_sent_total = sent_frames_payload_total_u64,
                    "SUSPICIOUSLY SMALL RESPONSE: possible accounting or streaming issue"
                );
            }

            let min_thr_bps = std::env::var("VPNNODE_MIN_THROUGHPUT_BPS")
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(50_000);
            let bytes_stop_wait_threshold = 200 * 1024;

            if sent_payload_bytes > bytes_stop_wait_threshold && avg_inflight < 1.5 {
                warn!(
                    stream_id,
                    resp_bytes = sent_payload_bytes,
                    avg_inflight,
                    "LOW WINDOW UTILIZATION: possible stop-and-wait behavior"
                );
            }

            if frames_sent_total > 0 && retransmit_rate > 0.05 {
                warn!(
                    stream_id,
                    frames_sent_total, total_retransmits, retransmit_rate, "HIGH RETRANSMIT RATE"
                );
            }

            if sent_payload_bytes > bytes_stop_wait_threshold
                && effective_throughput < min_thr_bps as u128
            {
                warn!(
                    stream_id,
                    throughput_bps = effective_throughput,
                    min_thr_bps,
                    "LOW THROUGHPUT"
                );
            }

            if let Some(ack_lat) = ack_latency_avg_ms {
                if ack_lat > 3000 {
                    warn!(
                        stream_id,
                        ack_latency_ms_avg = ack_lat,
                        "ACK latency unexpectedly high"
                    );
                }
            }

            info!(
                stream_id,
                resp_bytes = sent_payload_bytes,
                sent_payload_bytes,
                sent_frames,
                avg_inflight = avg_inflight,
                max_inflight,
                time_spent_at_inflight_1_ms = time_at_inflight_1_ms,
                response_window_frames = window_frames,
                first_overlay_send_ms = ?first_overlay_send_ms,
                window_wait_events,
                window_wait_total_ms,
                window_wait_max_ms,
                frames_sent_total,
                frames_acked_total,
                total_retransmits,
                retransmit_rate,
                stream_duration_ms,
                effective_throughput_bytes_per_s = effective_throughput,
                ack_latency_ms_avg = ?ack_latency_avg_ms,
                ack_latency_ms_p50 = ?ack_latency_p50_ms,
                ack_latency_ms_p95 = ?ack_latency_p95_ms,
                ack_latency_ms_max = ?ack_latency_max_ms,
                peer = %peer,
                "exit streamed response chunks and end-of-response to client"
            );
            let route_hops = routing.route.len();
            let http_code_val = http_code.unwrap_or(0);
            let ack_latency_ms_avg_val = ack_latency_avg_ms.unwrap_or(0);
            emit_exit_stage(
                "stream_complete",
                json!({
                    "stream_id": stream_id,
                    "site": site.as_str(),
                    "route_len": route_hops,
                    "peer": peer.to_string(),
                    "target_addr": target_addr.to_string(),
                    "first_target_byte_ms": first_target_byte_ms,
                    "first_overlay_send_ms": first_overlay_send_ms,
                    "stream_duration_ms": stream_duration_ms_total,
                    "resp_bytes": sent_payload_bytes_total_u64,
                    "frames_sent": sent_frames_payload_total_u64,
                    "avg_inflight": avg_inflight,
                    "max_inflight": max_inflight,
                    "window_frames": window_frames,
                    "window_wait_events": window_wait_events,
                    "window_wait_total_ms": window_wait_total_ms,
                    "window_wait_max_ms": window_wait_max_ms,
                    "time_at_inflight_1_ms": time_at_inflight_1_ms,
                    "total_retransmits": total_retransmits,
                    "retransmit_rate": retransmit_rate,
                    "ack_latency_ms_avg": ack_latency_ms_avg_val,
                    "ack_latency_ms_min": ack_latency_min_ms,
                    "ack_latency_ms_p50": ack_latency_p50_ms,
                    "ack_latency_ms_p95": ack_latency_p95_ms,
                    "ack_latency_ms_max": ack_latency_max_ms,
                    "http_code": http_code_val,
                    "connection_alive": connection_alive,
                }),
            );
            debug!(
                stream_id,
                site = %site,
                resp_bytes_total = sent_payload_bytes_total_u64,
                frames_sent_total = sent_frames_payload_total_u64,
                total_retransmits,
                stream_duration_ms_total,
                bursts_count = response_totals
                    .get(&stream_id)
                    .map(|e| e.bursts_count)
                    .unwrap_or(0),
                "exit stream totals snapshot"
            );
            info!(
                "VALIDATION_ARTIFACT,stream_id={},site={},exit={},route={},resp_bytes={},frames_sent={},avg_inflight={:.3},max_inflight={},time_at_inflight_1_ms={},retransmit_rate={:.6},stream_duration_ms={},throughput_bps={},ack_latency_ms_avg={},http_code={},is_final=0,mode=overlay",
                stream_id,
                site,
                exit_identity,
                route_hops,
                sent_payload_bytes_total_u64,
                sent_frames_payload_total_u64,
                avg_inflight,
                max_inflight,
                time_at_inflight_1_ms,
                retransmit_rate,
                stream_duration_ms_total,
                effective_throughput_total,
                ack_latency_ms_avg_val,
                http_code_val
            );

            let response_quality_feedback = ResponseQualityFeedback {
                stream_id,
                route_len: route_hops.min(u8::MAX as usize) as u8,
                resp_bytes: sent_payload_bytes_total_u64,
                frames_sent: sent_frames_payload_total_u64,
                stream_duration_ms: stream_duration_ms_total,
                first_target_byte_ms,
                first_overlay_send_ms,
                ack_latency_ms_avg: ack_latency_avg_ms,
                ack_latency_ms_p50: ack_latency_p50_ms,
                ack_latency_ms_p95: ack_latency_p95_ms,
                ack_latency_ms_max: ack_latency_max_ms,
                total_retransmits,
                retransmit_rate_ppm: (retransmit_rate.clamp(0.0, 1.0) * 1_000_000.0).round() as u32,
                window_wait_events,
                window_wait_total_ms,
                window_wait_max_ms,
                http_code,
            };

            // Only send CloseStream if the target TCP connection closed.
            // If the connection is still alive (streaming/TLS), keep it open
            // for follow-up data (TLS Finished, subsequent HTTP requests).
            if connection_alive {
                streams.insert(stream_id, st);
                // Reset reliable-stream state so the next continuation
                // round-trip (which starts sequence numbers from 0) is
                // accepted without triggering replay-window rejection.
                {
                    // Stage 9.1b correctness: don't drop response reliable state
                    // before the client has ACKed the in-flight response frames.
                    // Otherwise we lose retransmit capability and can surface
                    // "no response" timeouts under packet loss.
                    // Give the client more time to ACK the in-flight end-of-response marker.
                    // Under some loss patterns (especially with windowed DATA frames),
                    // ACK can arrive later than the previous 8s budget, causing end-marker
                    // delivery failures and "no response" timeouts on overlay curl.
                    let wait_deadline = Instant::now() + Duration::from_secs(20);
                    loop {
                        let inflight = {
                            let map = reliable_streams_for_task.lock().unwrap();
                            map.get(&stream_id).map(|rs| rs.inflight()).unwrap_or(0)
                        };
                        if inflight == 0 {
                            break;
                        }
                        if Instant::now() >= wait_deadline {
                            break;
                        }
                        tokio::time::sleep(Duration::from_millis(25)).await;
                    }

                    let mut rs_map = reliable_streams_for_task.lock().unwrap();
                    rs_map.remove(&stream_id);
                }
                debug!(stream_id, "exit keeping tcp connection open for streaming");
            } else {
                // Target closed: signal client that stream is done.
                let close_payload =
                    bincode::serialize(&response_quality_feedback).unwrap_or_default();
                for attempt in 1..=3u8 {
                    let msg2 = TunnelMessage::new(
                        MsgType::CloseStream,
                        1,
                        stream_id,
                        0,
                        close_payload.clone(),
                    );
                    if let Ok(ct2) = build_encrypted_packet(&crypto_for_task, &routing, msg2) {
                        let _ = udp_for_task.send(&peer, &ct2).await;
                        debug!(stream_id, attempt, peer = %peer, "exit sent CloseStream (target closed)");
                    }
                }
                // Deterministic validation: final artifact only on TCP EOF (no continuation expected),
                // and emitted exactly once per stream_id.
                if !final_emitted.contains(&stream_id) {
                    let final_bursts_count = response_totals
                        .get(&stream_id)
                        .map(|e| e.bursts_count)
                        .unwrap_or(0);
                    info!(
                        "VALIDATION_ARTIFACT_FINAL,stream_id={},site={},exit={},route={},resp_bytes={},frames_sent={},bursts_count={},avg_inflight={:.3},max_inflight={},time_at_inflight_1_ms={},retransmit_rate={:.6},stream_duration_ms={},throughput_bps={},ack_latency_ms_avg={},http_code={},is_final=1,mode=overlay",
                        stream_id,
                        site,
                        exit_identity,
                        route_hops,
                        sent_payload_bytes_total_u64,
                        sent_frames_payload_total_u64,
                        final_bursts_count,
                        avg_inflight,
                        max_inflight,
                        time_at_inflight_1_ms,
                        retransmit_rate,
                        stream_duration_ms_total,
                        effective_throughput_total,
                        ack_latency_ms_avg_val,
                        http_code_val
                    );
                    debug!(
                        stream_id,
                        bursts_count = final_bursts_count,
                        resp_bytes = sent_payload_bytes_total_u64,
                        frames_sent = sent_frames_payload_total_u64,
                        "FINAL artifact emitted for stream_id"
                    );
                    final_emitted.insert(stream_id);
                }
                // TCP stream ended => drop cumulative counters.
                response_totals.remove(&stream_id);
            }
            // close socket after single request/response
        }
    });

    // Task: retransmit unacked response frames (exit -> client) based on ACKs from client.
    {
        let udp_retx = udp.clone();
        let shared_crypto_retx = shared_crypto.clone();
        let shared_route_retx = shared_route.clone();
        let shared_peer_retx = shared_peer.clone();
        let reliable_streams_retx = reliable_streams.clone();
        tokio::spawn(async move {
            let tick = Duration::from_millis(50);
            loop {
                tokio::time::sleep(tick).await;
                let now = std::time::Instant::now();
                let (crypto_opt, route_opt, peer_opt) = {
                    (
                        shared_crypto_retx.lock().unwrap().clone(),
                        shared_route_retx.lock().unwrap().clone(),
                        shared_peer_retx.lock().unwrap().clone(),
                    )
                };
                let (Some(crypto), Some(route), Some(peer)) = (crypto_opt, route_opt, peer_opt)
                else {
                    continue;
                };
                let routing = RoutingInfo {
                    hop_index: (route.hops.len().saturating_sub(1)) as u8,
                    route,
                };
                let frames: Vec<StreamFrame> = {
                    let mut map = reliable_streams_retx.lock().unwrap();
                    let mut out = Vec::new();
                    for (&sid, rs) in map.iter_mut() {
                        let interval =
                            adaptive_response_retransmit_interval(rs.ack_latency_summary());
                        out.extend(rs.frames_for_retransmit(sid, now, interval, 16));
                    }
                    out
                };
                let mut per_stream_retx: HashMap<u32, usize> = HashMap::new();
                for f in frames {
                    *per_stream_retx.entry(f.stream_id).or_insert(0) += 1;
                    let Ok(frame_bytes) = bincode::serialize(&f) else {
                        continue;
                    };
                    let msg = TunnelMessage::new(MsgType::Data, 1, f.stream_id, 0, frame_bytes);
                    if let Ok(ct) = build_encrypted_packet(&crypto, &routing, msg) {
                        let _ = udp_retx.send(&peer, &ct).await;
                    }
                }
                for (sid, cnt) in per_stream_retx {
                    if cnt > 0 {
                        debug!(
                            stream_id = sid,
                            retransmit_frames = cnt,
                            "exit retransmit tick"
                        );
                    }
                }
            }
        });
    }

    loop {
        if drain::deadline_reached() {
            info!("[drain] shutdown complete");
            break Ok(());
        }
        let (peer, data) = udp.recv().await?;
        info!(
            protocol = ?peer.protocol,
            from = %peer,
            len = data.len(),
            "received via transport"
        );

        if session_crypto.is_none() {
            if drain::is_draining() {
                info!("[drain] rejecting new tunnels");
                continue;
            }
            // Expect a routed, but unencrypted, handshake message
            // (Stage 3.1: init → challenge or init+cookie → ack).
            let (routing, inner) = match parse_routing_header(&data) {
                Ok(v) => v,
                Err(e) => {
                    error!(%e, "exit: failed to parse routing header for handshake candidate");
                    continue;
                }
            };
            let msg = match decode(inner) {
                Ok(m) => m,
                Err(e) => {
                    error!(%e, "exit: failed to decode handshake candidate");
                    continue;
                }
            };
            if msg.header.version != PROTOCOL_VERSION {
                error!("exit: protocol version mismatch in handshake");
                continue;
            }

            // Stage 9.1: discovery plaintext handling before any AEAD session exists.
            if args.discovery_enabled {
                if let Some(event) = discovery::parse_discovery_message(&msg) {
                    let now = crate::ant::now_ms();
                    match event {
                        DiscoveryMessage::Advertise(advertisement) => {
                            let mut store = discovery_store.lock().unwrap();
                            store.purge_expired(now);
                            store.insert(advertisement.clone());
                            debug!(
                                event = "discovery_advertise_received",
                                role = ?advertisement.role,
                                node_id = ?advertisement.node_id,
                                peer = %peer,
                                "received discovery advertise"
                            );
                            continue;
                        }
                        DiscoveryMessage::Query(query) => {
                            let response_payload = {
                                let mut store = discovery_store.lock().unwrap();
                                store.on_query_received();
                                store.purge_expired(now);
                                discovery::build_discovery_response_payload(&store, &query, now)
                            };

                            let payload_bytes = match bincode::serialize(&response_payload) {
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
                            if let Ok(packet) = discovery::build_plaintext_packet(&peer, &resp_msg)
                            {
                                if udp.send(&peer, &packet).await.is_ok() {
                                    let mut store = discovery_store.lock().unwrap();
                                    store.on_response_sent();
                                }
                            }
                            debug!(
                                event = "discovery_response",
                                peer = %peer,
                                sent_ads = response_payload.advertisements.len(),
                                "answered discovery query"
                            );
                            continue;
                        }
                        DiscoveryMessage::Response(resp) => {
                            let mut store = discovery_store.lock().unwrap();
                            store.on_response_received();
                            store.purge_expired(now);
                            for adv in resp.advertisements.into_iter() {
                                store.insert(adv);
                            }
                            debug!(
                                event = "discovery_response_received",
                                peer = %peer,
                                "received discovery response"
                            );
                            continue;
                        }
                    }
                }
            }

            match msg.header.msg_type {
                MsgType::HandshakeInit => {
                    let payload: HandshakeInitPayload = match bincode::deserialize(&msg.payload) {
                        Ok(p) => p,
                        Err(e) => {
                            error!(%e, "exit: failed to decode HandshakeInit payload");
                            continue;
                        }
                    };
                    let session_id = msg.header.session_id;

                    if payload.cookie.is_none() {
                        // Stage 3.1: init without cookie → send challenge only; do NOT create session or do x25519.
                        if !rate_limiter.lock().unwrap().allow(peer.ip) {
                            debug!(peer = %peer, "exit: handshake rate limited");
                            continue;
                        }
                        let peer_sa = peer.as_socket_addr().ok_or_else(|| {
                            anyhow::anyhow!("exit: cookie challenge requires UDP peer addr")
                        })?;
                        let cookie = match generate_cookie(
                            &cookie_secret,
                            peer_sa,
                            &payload.client_pubkey,
                            &payload.client_nonce,
                        ) {
                            Ok(c) => c,
                            Err(e) => {
                                error!(%e, "exit: failed to generate cookie");
                                continue;
                            }
                        };
                        let challenge = HandshakeChallengePayload { cookie };
                        let challenge_bytes =
                            bincode::serialize(&challenge).expect("challenge serialize");
                        let challenge_msg = TunnelMessage::new(
                            MsgType::HandshakeChallenge,
                            session_id,
                            0,
                            0,
                            challenge_bytes,
                        );
                        let bytes = encode_plaintext(&challenge_msg)?;
                        // Send challenge back along the same route, from exit (last hop).
                        let routing_info = RoutingInfo {
                            hop_index: (routing.route.len().saturating_sub(1)) as u8,
                            route: routing.route.clone(),
                        };
                        let outer = crate::wire::build_handshake_packet(&routing_info, bytes)?;
                        udp.send(&peer, &outer).await?;
                        debug!(session_id, peer = %peer, "exit: sent HandshakeChallenge");
                        continue;
                    }

                    // Init with cookie: verify before any x25519 or session state.
                    let cookie = payload.cookie.as_ref().unwrap();
                    let peer_sa = peer.as_socket_addr().ok_or_else(|| {
                        anyhow::anyhow!("exit: cookie verify requires UDP peer addr")
                    })?;
                    if !verify_cookie(
                        &cookie_secret,
                        cookie,
                        peer_sa,
                        &payload.client_pubkey,
                        &payload.client_nonce,
                    )
                    .unwrap_or(false)
                    {
                        debug!(peer = %peer, "exit: cookie verification failed, dropping");
                        continue;
                    }

                    // Cookie valid: now do x25519 and create session.
                    let (ack_msg, _exit_secret, _exit_pub, aead_key) =
                        handle_handshake_init(&payload, session_id);
                    let bytes = encode_plaintext(&ack_msg)?;
                    // HandshakeAck also follows the routed path back.
                    let routing_info = RoutingInfo {
                        hop_index: (routing.route.len().saturating_sub(1)) as u8,
                        route: routing.route.clone(),
                    };
                    let outer = crate::wire::build_handshake_packet(&routing_info, bytes)?;
                    udp.send(&peer, &outer).await?;
                    let crypto = SessionCrypto::new(aead_key);
                    {
                        let mut guard = shared_crypto.lock().unwrap();
                        *guard = Some(crypto.clone());
                    }
                    session_crypto = Some(crypto);
                    info!(
                        session_id,
                        peer = %peer,
                        "exit: session established via HandshakeChallenge then HandshakeAck"
                    );
                    emit_exit_stage(
                        "session_established",
                        json!({
                            "session_id": session_id,
                            "peer": peer.to_string(),
                            "route_len": routing.route.len(),
                            "route_chain": routing
                                .route
                                .hops
                                .iter()
                                .map(|hop| hop.to_string())
                                .collect::<Vec<_>>()
                                .join(" -> "),
                        }),
                    );
                }
                other => {
                    error!(
                        msg_type = ?other,
                        "exit: received non-handshake message before session established"
                    );
                }
            }
            continue;
        }

        // Established session: all traffic must be AEAD-protected.
        // We clone to allow a fallback path to refresh session state if the
        // client is retrying handshake plaintext (e.g. handshake-ack loss).
        let crypto = session_crypto.as_ref().unwrap().clone();
        let (routing, inner) = match parse_routing_header(&data) {
            Ok(v) => v,
            Err(e) => {
                error!(%e, "exit failed to parse routing header on encrypted message");
                continue;
            }
        };
        let hops = &routing.route.hops;
        let hi = routing.hop_index as usize;
        if hi + 1 != hops.len() {
            error!(
                hop_index = hi,
                route_len = hops.len(),
                "exit: received tunnel message but not at final hop (dropping)"
            );
            continue;
        }
        info!(
            hop_index = hi,
            route_len = hops.len(),
            "exit reached final hop"
        );

        // Stage 9.1: discovery plaintext handling even if an AEAD session is established.
        if args.discovery_enabled {
            if let Ok(plain_msg) = decode(inner) {
                if let Some(event) = discovery::parse_discovery_message(&plain_msg) {
                    let now = crate::ant::now_ms();
                    match event {
                        DiscoveryMessage::Advertise(advertisement) => {
                            let mut store = discovery_store.lock().unwrap();
                            store.purge_expired(now);
                            store.insert(advertisement.clone());
                            debug!(
                                event = "discovery_advertise_received",
                                role = ?advertisement.role,
                                node_id = ?advertisement.node_id,
                                peer = %peer,
                                "received discovery advertise"
                            );
                            continue;
                        }
                        DiscoveryMessage::Query(query) => {
                            let response_payload = {
                                let mut store = discovery_store.lock().unwrap();
                                store.on_query_received();
                                store.purge_expired(now);
                                discovery::build_discovery_response_payload(&store, &query, now)
                            };

                            let payload_bytes = match bincode::serialize(&response_payload) {
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
                            if let Ok(packet) = discovery::build_plaintext_packet(&peer, &resp_msg)
                            {
                                if udp.send(&peer, &packet).await.is_ok() {
                                    let mut store = discovery_store.lock().unwrap();
                                    store.on_response_sent();
                                }
                            }
                            debug!(
                                event = "discovery_response",
                                peer = %peer,
                                sent_ads = response_payload.advertisements.len(),
                                "answered discovery query"
                            );
                            continue;
                        }
                        DiscoveryMessage::Response(resp) => {
                            let mut store = discovery_store.lock().unwrap();
                            store.on_response_received();
                            store.purge_expired(now);
                            for adv in resp.advertisements.into_iter() {
                                store.insert(adv);
                            }
                            debug!(
                                event = "discovery_response_received",
                                peer = %peer,
                                "received discovery response"
                            );
                            continue;
                        }
                    }
                }
            }
        }

        // Learn route and peer for this session on first data packet.
        if session_route.is_none() {
            session_route = Some(routing.route.clone());
            let mut g = shared_route.lock().unwrap();
            *g = Some(routing.route.clone());
            info!(
                route_len = hops.len(),
                "exit stored session route from first data packet"
            );
        }
        if session_peer.is_none() {
            session_peer = Some(peer.clone());
            let mut gp = shared_peer.lock().unwrap();
            *gp = Some(peer.clone());
            info!(%peer, "exit stored session peer from first data packet");
        }
        let msg = match crypto.open_message(inner) {
            Ok(m) => m,
            Err(e) => {
                // If the client is still retrying the Stage 3.1 plaintext handshake
                // while we have already switched into "encrypted" mode, handle
                // HandshakeInit again to allow recovery.
                if let Ok(plain_msg) = decode(inner) {
                    if plain_msg.header.version == PROTOCOL_VERSION
                        && plain_msg.header.msg_type == MsgType::HandshakeInit
                    {
                        let payload: HandshakeInitPayload = match bincode::deserialize(
                            &plain_msg.payload,
                        ) {
                            Ok(p) => p,
                            Err(_) => {
                                error!(%e, "exit failed to decode HandshakeInit during AEAD open fallback");
                                continue;
                            }
                        };
                        let session_id = plain_msg.header.session_id;

                        if payload.cookie.is_none() {
                            // Stage 3.1 init without cookie → send challenge only.
                            if !rate_limiter.lock().unwrap().allow(peer.ip) {
                                debug!(peer = %peer, "exit handshake rate limited (fallback)");
                                continue;
                            }
                            let peer_sa = match peer.as_socket_addr() {
                                Some(sa) => sa,
                                None => {
                                    error!(
                                        "exit: cookie challenge requires UDP peer addr (fallback)"
                                    );
                                    continue;
                                }
                            };
                            let cookie = match generate_cookie(
                                &cookie_secret,
                                peer_sa,
                                &payload.client_pubkey,
                                &payload.client_nonce,
                            ) {
                                Ok(c) => c,
                                Err(err) => {
                                    error!(%err, "exit: failed to generate cookie (fallback)");
                                    continue;
                                }
                            };
                            let challenge = HandshakeChallengePayload { cookie };
                            let challenge_bytes = match bincode::serialize(&challenge) {
                                Ok(b) => b,
                                Err(_) => continue,
                            };
                            let challenge_msg = TunnelMessage::new(
                                MsgType::HandshakeChallenge,
                                session_id,
                                0,
                                0,
                                challenge_bytes,
                            );
                            if let Ok(bytes) = encode_plaintext(&challenge_msg) {
                                let routing_info = RoutingInfo {
                                    hop_index: (routing.route.len().saturating_sub(1)) as u8,
                                    route: routing.route.clone(),
                                };
                                if let Ok(outer) =
                                    crate::wire::build_handshake_packet(&routing_info, bytes)
                                {
                                    let _ = udp.send(&peer, &outer).await;
                                }
                            }
                            // Keep session crypto as-is; client will retry until it completes.
                            continue;
                        }

                        // Stage 3.1 init with cookie: verify before creating a session.
                        let cookie = payload.cookie.as_ref().unwrap();
                        let peer_sa = match peer.as_socket_addr() {
                            Some(sa) => sa,
                            None => {
                                error!("exit: cookie verify requires UDP peer addr (fallback)");
                                continue;
                            }
                        };
                        let valid = verify_cookie(
                            &cookie_secret,
                            cookie,
                            peer_sa,
                            &payload.client_pubkey,
                            &payload.client_nonce,
                        )
                        .unwrap_or(false);
                        if !valid {
                            debug!(peer = %peer, "exit: cookie verification failed, dropping (fallback)");
                            continue;
                        }

                        let (ack_msg, _exit_secret, _exit_pub, aead_key) =
                            handle_handshake_init(&payload, session_id);
                        let bytes = match encode_plaintext(&ack_msg) {
                            Ok(b) => b,
                            Err(_) => continue,
                        };
                        let routing_info = RoutingInfo {
                            hop_index: (routing.route.len().saturating_sub(1)) as u8,
                            route: routing.route.clone(),
                        };
                        if let Ok(outer) = crate::wire::build_handshake_packet(&routing_info, bytes)
                        {
                            let _ = udp.send(&peer, &outer).await;
                        }

                        let crypto_new = SessionCrypto::new(aead_key);
                        {
                            let mut guard = shared_crypto.lock().unwrap();
                            *guard = Some(crypto_new.clone());
                        }
                        session_crypto = Some(crypto_new);
                        continue;
                    }
                }
                emit_exit_stage(
                    "open_message_failed",
                    json!({
                        "peer": peer.to_string(),
                        "route_len": session_route.as_ref().map(|route| route.len()),
                        "error": e.to_string(),
                    }),
                );
                error!(%e, "exit failed to open message");
                continue;
            }
        };
        if msg.header.version != PROTOCOL_VERSION {
            error!("exit: protocol version mismatch");
            continue;
        }
        match msg.header.msg_type {
            MsgType::Data | MsgType::OpenStream => {
                debug!(stream_id = msg.header.stream_id, msg_type = ?msg.header.msg_type, "exit received tunnel message");
                // Stage 4: DATA payload from client is a StreamFrame.
                if msg.header.msg_type == MsgType::Data {
                    let frame: StreamFrame = match bincode::deserialize(&msg.payload) {
                        Ok(f) => f,
                        Err(e) => {
                            error!(%e, "exit: failed to decode StreamFrame from client");
                            continue;
                        }
                    };

                    // If we've already completed a request for this stream, ignore duplicates/retransmits.
                    if completed_requests
                        .lock()
                        .unwrap()
                        .contains(&frame.stream_id)
                    {
                        debug!(
                            stream_id = frame.stream_id,
                            "exit: ignoring DATA for completed stream"
                        );
                        continue;
                    }

                    // Reliable receive path for request stream: reorder, handle duplicates, compute ACK.
                    let (to_forward, ack_seq, end_of_stream) = {
                        let mut map = reliable_streams.lock().unwrap();
                        let rs = map
                            .entry(frame.stream_id)
                            .or_insert_with(ReliableStream::new);
                        rs.process_incoming(&frame)
                    };

                    let peer_for_session = match session_peer.clone() {
                        Some(p) => p,
                        None => {
                            error!("exit: no session_peer when buffering DATA for target");
                            continue;
                        }
                    };

                    // Send cumulative ACK back to client (via relay) as Ping control message.
                    if let Some(route) = session_route.clone() {
                        let routing = RoutingInfo {
                            hop_index: (route.hops.len().saturating_sub(1)) as u8,
                            route,
                        };
                        let ack = AckFrame {
                            stream_id: frame.stream_id,
                            ack_seq,
                        };
                        if let Ok(ack_bytes) = bincode::serialize(&ack) {
                            let ack_msg =
                                TunnelMessage::new(MsgType::Ping, 1, frame.stream_id, 0, ack_bytes);
                            if let Ok(ct) = build_encrypted_packet(&crypto, &routing, ack_msg) {
                                let _ = udp.send(&peer_for_session, &ct).await;
                            }
                        }
                    }

                    // Accumulate chunks per stream_id until end-of-request marker.
                    for chunk in &to_forward {
                        let mut buf_map = request_buffer.lock().unwrap();
                        let entry = buf_map.entry(frame.stream_id).or_default();
                        entry.extend_from_slice(chunk);
                        info!(
                            stream_id = frame.stream_id,
                            appended = chunk.len(),
                            total = entry.len(),
                            "exit: appended request bytes from client"
                        );

                        // Fallback: if we have a complete HTTP request by Content-Length,
                        // trigger a flush even if the end-of-request empty frame is lost.
                        if let Some((hdr_end, body_len)) = http_content_length(entry) {
                            let need = hdr_end + body_len;
                            if entry.len() >= need {
                                info!(
                                    stream_id = frame.stream_id,
                                    bytes = entry.len(),
                                    need,
                                    "exit: buffered full HTTP request by content-length, triggering flush"
                                );
                                let _ = tx_map.send((
                                    frame.stream_id,
                                    Vec::new(),
                                    peer_for_session.clone(),
                                ));
                            }
                        }
                    }

                    if end_of_stream {
                        info!(
                            stream_id = frame.stream_id,
                            "exit: received end-of-request marker from client"
                        );
                        // Signal TCP worker that stream is complete; it will
                        // take the buffered request and forward it once.
                        let _ =
                            tx_map.send((frame.stream_id, Vec::new(), peer_for_session.clone()));
                    }

                    // For now, we rely on local UDP reliability and do not send
                    // explicit ACK-only frames back to the client from the exit.
                } else {
                    // OpenStream remains a simple "control" message for now: the
                    // TCP worker will treat the first non-empty DATA as request bytes.
                    let _ = tx_map.send((msg.header.stream_id, msg.payload, peer));
                }
            }
            MsgType::Ping => {
                // ACK-only control message from client for exit->client response stream.
                let ack: AckFrame = match bincode::deserialize(&msg.payload) {
                    Ok(a) => a,
                    Err(e) => {
                        error!(%e, "exit: failed to decode AckFrame");
                        continue;
                    }
                };
                let mut map = reliable_streams.lock().unwrap();
                let rs = map.entry(ack.stream_id).or_insert_with(ReliableStream::new);
                let (acked, avg_latency_ms) = rs.apply_ack_with_latency(ack.ack_seq);
                if acked > 0 {
                    debug!(
                        stream_id = ack.stream_id,
                        ack_seq = ack.ack_seq,
                        acked_frames = acked,
                        ack_latency_ms_avg = ?avg_latency_ms,
                        "exit: cumulative ACK applied"
                    );
                }
            }
            MsgType::CloseStream => {
                let _ = tx_map.send((msg.header.stream_id, Vec::new(), peer));
            }
            MsgType::Ant => {
                // Stage 7: measurement-only ants. Exit bounces Echo ants back to client.
                let mut ant: Ant = match bincode::deserialize(&msg.payload) {
                    Ok(a) => a,
                    Err(e) => {
                        debug!(%e, "exit: failed to decode ant");
                        continue;
                    }
                };
                if !ant.validate() || !ant.size_ok() {
                    debug!("exit: dropping invalid/oversize ant");
                    continue;
                }
                {
                    let mut d = ant_dedup.lock().unwrap();
                    if !d.check_and_mark(ant.id) {
                        debug!("exit: duplicate ant ignored");
                        continue;
                    }
                }

                // Append an observation at exit (cheap + bounded).
                if ant.observations.len() < crate::ant::MAX_OBSERVATIONS {
                    ant.observations.push(Observation {
                        node: self_node.clone(),
                        rtt_ms: None,
                        success: Some(true),
                        timestamp_ms: crate::ant::now_ms(),
                    });
                }

                if ant.ant_type == AntType::Echo {
                    let (Some(route), Some(peer_for_session), Some(crypto_for_task)) = (
                        session_route.clone(),
                        session_peer.clone(),
                        session_crypto.clone(),
                    ) else {
                        continue;
                    };
                    // Send back along reverse path (hop_index = last).
                    let routing = RoutingInfo {
                        hop_index: (route.len().saturating_sub(1)) as u8,
                        route,
                    };
                    let payload = match bincode::serialize(&ant) {
                        Ok(p) => p,
                        Err(_) => continue,
                    };
                    let msg2 = TunnelMessage::new(MsgType::Ant, 1, 0, 0, payload);
                    if let Ok(ct) = build_encrypted_packet(&crypto_for_task, &routing, msg2) {
                        let _ = udp.send(&peer_for_session, &ct).await;
                    }
                }
            }
            _ => {
                // ignore other messages for MVP
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream_reliable::AckLatencySummary;

    #[test]
    fn adaptive_retransmit_interval_uses_base_without_ack_samples() {
        let interval = adaptive_response_retransmit_interval(AckLatencySummary::default());
        assert_eq!(
            interval,
            Duration::from_millis(EXIT_RESPONSE_RETRANSMIT_BASE_MS)
        );
    }

    #[test]
    fn adaptive_retransmit_interval_tracks_high_ack_p95() {
        let interval = adaptive_response_retransmit_interval(AckLatencySummary {
            avg_ms: Some(260),
            min_ms: Some(220),
            p50_ms: Some(255),
            p95_ms: Some(420),
            max_ms: Some(460),
        });
        assert_eq!(
            interval,
            Duration::from_millis(420 + EXIT_RESPONSE_RETRANSMIT_SAFETY_MARGIN_MS)
        );
    }

    #[test]
    fn adaptive_retransmit_interval_is_clamped() {
        let interval = adaptive_response_retransmit_interval(AckLatencySummary {
            avg_ms: Some(1800),
            min_ms: Some(1600),
            p50_ms: Some(1700),
            p95_ms: Some(1900),
            max_ms: Some(2200),
        });
        assert_eq!(
            interval,
            Duration::from_millis(EXIT_RESPONSE_RETRANSMIT_MAX_MS)
        );
    }
}
