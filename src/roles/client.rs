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
use crate::protocol::{
    decode, MsgType, ResponseQualityFeedback, StreamFrame, TunnelMessage, PROTOCOL_VERSION,
};
use crate::route::Route;
use crate::route_memory::RouteCache;
use crate::route_store::{LocalRouteObservation, RouteFailureKind, RouteStore};
use crate::session::{classify_open_message_error, SessionCrypto, SessionOpenRejectKind};
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

type ResponseTx = mpsc::UnboundedSender<ResponseEvent>;
type ResponseSenderTable = Arc<Mutex<HashMap<u32, ResponseTx>>>;
type ReliableStreamTable = Arc<Mutex<HashMap<u32, Arc<Mutex<ReliableStream>>>>>;
type StreamRouteTable = Arc<Mutex<HashMap<u32, Route>>>;
type ResponseStartedTable = Arc<Mutex<HashMap<u32, bool>>>;
type CompletedResponseStreamTable = Arc<Mutex<HashMap<u32, CompletedResponseStream>>>;
type CloseSignalSeenTable = Arc<Mutex<HashMap<u32, Instant>>>;

const COMPLETED_RESPONSE_STREAM_LINGER: Duration = Duration::from_secs(30);
const MAX_COMPLETED_RESPONSE_STREAMS: usize = 2048;
const CLOSE_FEEDBACK_LINGER: Duration = Duration::from_secs(30);
const MAX_CLOSE_FEEDBACK_STREAMS: usize = 2048;
const BUFFERED_RESPONSE_TRANSPORT_SETTLEMENT_QUIET_PERIOD: Duration = Duration::from_millis(250);
const BUFFERED_RESPONSE_TRANSPORT_SETTLEMENT_MAX: Duration = Duration::from_secs(2);
const BUFFERED_HTTP_RESPONSE_SOFT_CAP_SLACK: usize = 256 * 1024;
const BUFFERED_HTTP_RESPONSE_CONTENT_LENGTH_SLACK: usize = 4 * 1024;
const BUFFERED_HTTP_RESPONSE_HARD_CAP: usize = 16 * 1024 * 1024;

#[derive(Debug, Clone)]
enum ResponseEvent {
    Payload(Vec<u8>),
    EndOfStream,
    CloseStream,
}

enum ResponseDataDispatch {
    Active {
        tx: ResponseTx,
        reliable: Arc<Mutex<ReliableStream>>,
        route: Route,
    },
    Completed {
        reliable: Arc<Mutex<ReliableStream>>,
        route: Route,
        completion_reason: &'static str,
        pruned_stale_active_state: bool,
    },
    Orphaned {
        reliable: Arc<Mutex<ReliableStream>>,
        route: Route,
        pruned_stale_active_state: bool,
    },
    Unknown,
}

enum ResponseAckDispatch {
    Active(Arc<Mutex<ReliableStream>>),
    Completed {
        reliable: Arc<Mutex<ReliableStream>>,
        completion_reason: &'static str,
        pruned_stale_active_state: bool,
    },
    Unknown {
        pruned_stale_active_state: bool,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct BufferedHttpResponseInspection {
    header_end: Option<usize>,
    content_length: Option<usize>,
    expected_total: Option<usize>,
    fallback_cap: usize,
    hard_cap: usize,
    active_cap: usize,
    content_length_exceeds_cap: bool,
    completion_reason: Option<&'static str>,
}

fn inspect_buffered_http_response(
    request_len: usize,
    response: &[u8],
) -> BufferedHttpResponseInspection {
    let fallback_cap = request_len.saturating_add(BUFFERED_HTTP_RESPONSE_SOFT_CAP_SLACK);
    let hard_cap = fallback_cap.max(BUFFERED_HTTP_RESPONSE_HARD_CAP);
    let mut inspection = BufferedHttpResponseInspection {
        header_end: http_header_end(response),
        content_length: None,
        expected_total: None,
        fallback_cap,
        hard_cap,
        active_cap: fallback_cap,
        content_length_exceeds_cap: false,
        completion_reason: None,
    };

    if let Some((header_end, body_len)) = http_content_length(response) {
        let expected_total = header_end.saturating_add(body_len);
        inspection.header_end = Some(header_end);
        inspection.content_length = Some(body_len);
        inspection.expected_total = Some(expected_total);
        inspection.active_cap = fallback_cap
            .max(expected_total.saturating_add(BUFFERED_HTTP_RESPONSE_CONTENT_LENGTH_SLACK))
            .min(hard_cap);
        inspection.content_length_exceeds_cap = expected_total > hard_cap;

        if !inspection.content_length_exceeds_cap && response.len() >= expected_total {
            inspection.completion_reason = Some("content_length_reached");
        } else if response.len() >= inspection.active_cap {
            inspection.completion_reason = Some("buffer_cap_reached");
        }
        return inspection;
    }

    if response.len() >= inspection.active_cap {
        inspection.completion_reason = Some("buffer_cap_reached");
    }
    inspection
}

#[derive(Clone)]
struct CompletedResponseStream {
    reliable: Arc<Mutex<ReliableStream>>,
    route: Route,
    completed_at: Instant,
    completion_reason: &'static str,
    late_payloads: u64,
    duplicate_payload_signals: u64,
    late_close_signals: u64,
    terminal_payload_signals: u64,
    terminal_close_signals: u64,
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
            duplicate_payload_signals: 0,
            late_close_signals: 0,
            terminal_payload_signals: 0,
            terminal_close_signals: 0,
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

fn completion_reason_uses_terminal_settlement(completion_reason: &'static str) -> bool {
    completion_reason == "content_length_reached"
}

fn note_completed_response_terminal_payload(
    completed_streams: &CompletedResponseStreamTable,
    stream_id: u32,
) -> Option<(u64, &'static str, bool, u64)> {
    let mut map = completed_streams.lock().unwrap();
    purge_completed_response_streams(&mut map, Instant::now());
    let entry = map.get_mut(&stream_id)?;
    let first_signal = entry.terminal_payload_signals == 0;
    entry.terminal_payload_signals = entry.terminal_payload_signals.saturating_add(1);
    Some((
        entry.completed_at.elapsed().as_millis() as u64,
        entry.completion_reason,
        first_signal,
        entry.terminal_payload_signals,
    ))
}

fn note_completed_response_duplicate_payload(
    completed_streams: &CompletedResponseStreamTable,
    stream_id: u32,
) -> Option<(u64, &'static str, bool, u64)> {
    let mut map = completed_streams.lock().unwrap();
    purge_completed_response_streams(&mut map, Instant::now());
    let entry = map.get_mut(&stream_id)?;
    let first_signal = entry.duplicate_payload_signals == 0;
    entry.duplicate_payload_signals = entry.duplicate_payload_signals.saturating_add(1);
    Some((
        entry.completed_at.elapsed().as_millis() as u64,
        entry.completion_reason,
        first_signal,
        entry.duplicate_payload_signals,
    ))
}

fn note_completed_response_terminal_close(
    completed_streams: &CompletedResponseStreamTable,
    stream_id: u32,
) -> Option<(u64, &'static str, bool, u64)> {
    let mut map = completed_streams.lock().unwrap();
    purge_completed_response_streams(&mut map, Instant::now());
    let entry = map.get_mut(&stream_id)?;
    let first_signal = entry.terminal_close_signals == 0;
    entry.terminal_close_signals = entry.terminal_close_signals.saturating_add(1);
    Some((
        entry.completed_at.elapsed().as_millis() as u64,
        entry.completion_reason,
        first_signal,
        entry.terminal_close_signals,
    ))
}

fn purge_close_signal_seen(map: &mut HashMap<u32, Instant>, now: Instant) {
    map.retain(|_, seen_at| now.duration_since(*seen_at) <= CLOSE_FEEDBACK_LINGER);
    if map.len() <= MAX_CLOSE_FEEDBACK_STREAMS {
        return;
    }
    let mut by_age = map
        .iter()
        .map(|(&stream_id, seen_at)| (stream_id, *seen_at))
        .collect::<Vec<_>>();
    by_age.sort_by_key(|(_, seen_at)| *seen_at);
    let remove_count = map.len().saturating_sub(MAX_CLOSE_FEEDBACK_STREAMS);
    for (stream_id, _) in by_age.into_iter().take(remove_count) {
        map.remove(&stream_id);
    }
}

fn mark_close_signal_seen(close_signal_seen: &CloseSignalSeenTable, stream_id: u32) -> bool {
    let mut map = close_signal_seen.lock().unwrap();
    let now = Instant::now();
    purge_close_signal_seen(&mut map, now);
    if map.contains_key(&stream_id) {
        return false;
    }
    map.insert(stream_id, now);
    true
}

fn buffered_close_requires_wait(expected_total: Option<usize>, response_len: usize) -> bool {
    expected_total
        .map(|expected_total| response_len < expected_total)
        .unwrap_or(false)
}

fn prune_active_response_stream_state(
    reliable_streams: &ReliableStreamTable,
    response_started: &ResponseStartedTable,
    stream_routes: &StreamRouteTable,
    stream_id: u32,
) -> bool {
    let removed_reliable = {
        let mut map = reliable_streams.lock().unwrap();
        map.remove(&stream_id).is_some()
    };
    let removed_started = {
        let mut map = response_started.lock().unwrap();
        map.remove(&stream_id).is_some()
    };
    let removed_route = {
        let mut map = stream_routes.lock().unwrap();
        map.remove(&stream_id).is_some()
    };
    removed_reliable || removed_started || removed_route
}

fn finalize_response_stream_cleanup(
    response_senders: &ResponseSenderTable,
    reliable_streams: &ReliableStreamTable,
    response_started: &ResponseStartedTable,
    stream_routes: &StreamRouteTable,
    completed_response_streams: &CompletedResponseStreamTable,
    stream_id: u32,
    reliable: Arc<Mutex<ReliableStream>>,
    route: Route,
    completion_reason: &'static str,
) {
    remember_completed_response_stream(
        completed_response_streams,
        stream_id,
        reliable,
        route,
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
}

async fn settle_buffered_response_transport(
    stream_id: u32,
    request_site: String,
    route: Route,
    completion_reason: &'static str,
    response_bytes_total: usize,
    initial_terminal_close_signal: bool,
    reliable: Arc<Mutex<ReliableStream>>,
    mut rx_from_udp_stream: mpsc::UnboundedReceiver<ResponseEvent>,
    response_senders: ResponseSenderTable,
    reliable_streams: ReliableStreamTable,
    response_started: ResponseStartedTable,
    stream_routes: StreamRouteTable,
    completed_response_streams: CompletedResponseStreamTable,
) {
    let route_chain = route
        .hops
        .iter()
        .map(|hop| hop.to_string())
        .collect::<Vec<_>>()
        .join(" -> ");
    let settlement_start = Instant::now();
    let mut terminal_payload_signals = 0u64;
    let mut terminal_close_signals: u64 = if initial_terminal_close_signal { 1 } else { 0 };
    let mut payload_after_local_completion_events = 0u64;
    let mut payload_after_local_completion_bytes = 0usize;
    emit_client_stage(
        "response_transport_settlement_started",
        json!({
            "stream_id": stream_id,
            "site": request_site.as_str(),
            "route_len": route.len(),
            "route_chain": route_chain.as_str(),
            "completion_reason": completion_reason,
            "response_bytes_total": response_bytes_total,
            "quiet_period_ms": BUFFERED_RESPONSE_TRANSPORT_SETTLEMENT_QUIET_PERIOD.as_millis() as u64,
            "max_settlement_ms": BUFFERED_RESPONSE_TRANSPORT_SETTLEMENT_MAX.as_millis() as u64,
            "initial_terminal_close_signal": initial_terminal_close_signal,
        }),
    );
    if initial_terminal_close_signal {
        emit_client_stage(
            "response_transport_terminal_close_observed",
            json!({
                "stream_id": stream_id,
                "site": request_site.as_str(),
                "route_len": route.len(),
                "route_chain": route_chain.as_str(),
                "completion_reason": completion_reason,
                "terminal_close_signals": terminal_close_signals,
                "settlement_elapsed_ms": 0,
                "source": "queued_pre_local_completion",
            }),
        );
    }

    let settlement_max_deadline = settlement_start + BUFFERED_RESPONSE_TRANSPORT_SETTLEMENT_MAX;
    let mut last_transport_event_at = settlement_start;
    let settlement_reason = loop {
        let elapsed = settlement_start.elapsed();
        if terminal_payload_signals > 0
            && terminal_close_signals > 0
            && last_transport_event_at.elapsed()
                >= BUFFERED_RESPONSE_TRANSPORT_SETTLEMENT_QUIET_PERIOD
        {
            break "terminal_quiet_period_elapsed";
        }
        if Instant::now() >= settlement_max_deadline {
            break "settlement_max_elapsed";
        }
        let until_max = settlement_max_deadline
            .checked_duration_since(Instant::now())
            .unwrap_or_default();
        let wait_budget = if terminal_payload_signals > 0 && terminal_close_signals > 0 {
            until_max.min(
                BUFFERED_RESPONSE_TRANSPORT_SETTLEMENT_QUIET_PERIOD
                    .checked_sub(last_transport_event_at.elapsed())
                    .unwrap_or_default(),
            )
        } else {
            until_max
        };
        let next = match timeout(wait_budget, rx_from_udp_stream.recv()).await {
            Ok(value) => value,
            Err(_) => {
                if terminal_payload_signals > 0
                    && terminal_close_signals > 0
                    && last_transport_event_at.elapsed()
                        >= BUFFERED_RESPONSE_TRANSPORT_SETTLEMENT_QUIET_PERIOD
                {
                    break "terminal_quiet_period_elapsed";
                }
                if Instant::now() >= settlement_max_deadline {
                    break "settlement_max_elapsed";
                }
                continue;
            }
        };
        let Some(event) = next else {
            break "response_channel_closed";
        };
        last_transport_event_at = Instant::now();
        match event {
            ResponseEvent::Payload(chunk) => {
                if chunk.is_empty() {
                    continue;
                }
                payload_after_local_completion_events =
                    payload_after_local_completion_events.saturating_add(1);
                payload_after_local_completion_bytes =
                    payload_after_local_completion_bytes.saturating_add(chunk.len());
                emit_client_stage(
                    "payload_after_local_completion_during_settlement",
                    json!({
                        "stream_id": stream_id,
                        "site": request_site.as_str(),
                        "route_len": route.len(),
                        "route_chain": route_chain.as_str(),
                        "completion_reason": completion_reason,
                        "chunk_bytes": chunk.len(),
                        "payload_after_local_completion_events": payload_after_local_completion_events,
                        "payload_after_local_completion_bytes": payload_after_local_completion_bytes,
                        "settlement_elapsed_ms": elapsed.as_millis() as u64,
                    }),
                );
                debug!(
                    stream_id,
                    chunk_bytes = chunk.len(),
                    payload_after_local_completion_events,
                    payload_after_local_completion_bytes,
                    "client: payload arrived after local buffered completion during transport settlement"
                );
            }
            ResponseEvent::EndOfStream => {
                terminal_payload_signals = terminal_payload_signals.saturating_add(1);
                let stage = if terminal_payload_signals == 1 {
                    "response_transport_terminal_payload_observed"
                } else {
                    "response_transport_terminal_payload_duplicate"
                };
                emit_client_stage(
                    stage,
                    json!({
                        "stream_id": stream_id,
                        "site": request_site.as_str(),
                        "route_len": route.len(),
                        "route_chain": route_chain.as_str(),
                        "completion_reason": completion_reason,
                        "terminal_payload_signals": terminal_payload_signals,
                        "settlement_elapsed_ms": elapsed.as_millis() as u64,
                    }),
                );
            }
            ResponseEvent::CloseStream => {
                terminal_close_signals = terminal_close_signals.saturating_add(1);
                let stage = if terminal_close_signals == 1 {
                    "response_transport_terminal_close_observed"
                } else {
                    "response_transport_terminal_close_duplicate"
                };
                emit_client_stage(
                    stage,
                    json!({
                        "stream_id": stream_id,
                        "site": request_site.as_str(),
                        "route_len": route.len(),
                        "route_chain": route_chain.as_str(),
                        "completion_reason": completion_reason,
                        "terminal_close_signals": terminal_close_signals,
                        "settlement_elapsed_ms": elapsed.as_millis() as u64,
                    }),
                );
            }
        }
    };

    finalize_response_stream_cleanup(
        &response_senders,
        &reliable_streams,
        &response_started,
        &stream_routes,
        &completed_response_streams,
        stream_id,
        reliable,
        route.clone(),
        completion_reason,
    );
    emit_client_stage(
        "response_channel_removed",
        json!({
            "stream_id": stream_id,
            "site": request_site.as_str(),
            "route_len": route.len(),
            "route_chain": route_chain.as_str(),
            "completion_reason": completion_reason,
            "write_response_to_tcp": false,
            "transport_settled": true,
            "settlement_reason": settlement_reason,
        }),
    );
    emit_client_stage(
        "response_transport_settlement_completed",
        json!({
            "stream_id": stream_id,
            "site": request_site.as_str(),
            "route_len": route.len(),
            "route_chain": route_chain.as_str(),
            "completion_reason": completion_reason,
            "response_bytes_total": response_bytes_total,
            "settlement_reason": settlement_reason,
            "settlement_duration_ms": settlement_start.elapsed().as_millis() as u64,
            "quiet_period_ms": BUFFERED_RESPONSE_TRANSPORT_SETTLEMENT_QUIET_PERIOD.as_millis() as u64,
            "max_settlement_ms": BUFFERED_RESPONSE_TRANSPORT_SETTLEMENT_MAX.as_millis() as u64,
            "terminal_payload_signals": terminal_payload_signals,
            "terminal_close_signals": terminal_close_signals,
            "payload_after_local_completion_events": payload_after_local_completion_events,
            "payload_after_local_completion_bytes": payload_after_local_completion_bytes,
        }),
    );
}

fn resolve_response_data_dispatch(
    stream_id: u32,
    response_senders: &ResponseSenderTable,
    reliable_streams: &ReliableStreamTable,
    response_started: &ResponseStartedTable,
    stream_routes: &StreamRouteTable,
    completed_streams: &CompletedResponseStreamTable,
    default_route: &Route,
) -> ResponseDataDispatch {
    let active_tx = {
        let map = response_senders.lock().unwrap();
        map.get(&stream_id).cloned()
    };
    let active_rs = {
        let map = reliable_streams.lock().unwrap();
        map.get(&stream_id).cloned()
    };
    let active_route = {
        let map = stream_routes.lock().unwrap();
        map.get(&stream_id).cloned()
    };
    let completed_entry = if active_tx.is_none() {
        snapshot_completed_response_stream(completed_streams, stream_id)
    } else {
        None
    };

    if let Some(tx) = active_tx {
        let reliable = if let Some(reliable) = active_rs {
            reliable
        } else {
            let mut map = reliable_streams.lock().unwrap();
            map.entry(stream_id)
                .or_insert_with(|| Arc::new(Mutex::new(ReliableStream::new())))
                .clone()
        };
        return ResponseDataDispatch::Active {
            tx,
            reliable,
            route: active_route.unwrap_or_else(|| default_route.clone()),
        };
    }

    if let Some(completed) = completed_entry {
        let pruned_stale_active_state = if active_rs.is_some() {
            prune_active_response_stream_state(
                reliable_streams,
                response_started,
                stream_routes,
                stream_id,
            )
        } else {
            false
        };
        return ResponseDataDispatch::Completed {
            reliable: completed.reliable.clone(),
            route: completed.route.clone(),
            completion_reason: completed.completion_reason,
            pruned_stale_active_state,
        };
    }

    if let Some(reliable) = active_rs {
        let pruned_stale_active_state = prune_active_response_stream_state(
            reliable_streams,
            response_started,
            stream_routes,
            stream_id,
        );
        return ResponseDataDispatch::Orphaned {
            reliable,
            route: active_route.unwrap_or_else(|| default_route.clone()),
            pruned_stale_active_state,
        };
    }

    ResponseDataDispatch::Unknown
}

fn resolve_response_ack_dispatch(
    stream_id: u32,
    response_senders: &ResponseSenderTable,
    reliable_streams: &ReliableStreamTable,
    response_started: &ResponseStartedTable,
    stream_routes: &StreamRouteTable,
    completed_streams: &CompletedResponseStreamTable,
) -> ResponseAckDispatch {
    let has_active_consumer = {
        let map = response_senders.lock().unwrap();
        map.contains_key(&stream_id)
    };
    let active_rs = {
        let map = reliable_streams.lock().unwrap();
        map.get(&stream_id).cloned()
    };
    let completed_entry = if !has_active_consumer {
        snapshot_completed_response_stream(completed_streams, stream_id)
    } else {
        None
    };

    if has_active_consumer {
        if let Some(reliable) = active_rs {
            return ResponseAckDispatch::Active(reliable);
        }
        return ResponseAckDispatch::Unknown {
            pruned_stale_active_state: false,
        };
    }

    if let Some(completed) = completed_entry {
        let pruned_stale_active_state = if active_rs.is_some() {
            prune_active_response_stream_state(
                reliable_streams,
                response_started,
                stream_routes,
                stream_id,
            )
        } else {
            false
        };
        return ResponseAckDispatch::Completed {
            reliable: completed.reliable.clone(),
            completion_reason: completed.completion_reason,
            pruned_stale_active_state,
        };
    }

    let pruned_stale_active_state = if active_rs.is_some() {
        prune_active_response_stream_state(
            reliable_streams,
            response_started,
            stream_routes,
            stream_id,
        )
    } else {
        false
    };
    ResponseAckDispatch::Unknown {
        pruned_stale_active_state,
    }
}

fn build_probe_request() -> Vec<u8> {
    let path = std::env::var("VPNNODE_CLIENT_PROBE_PATH").unwrap_or_else(|_| "/".to_string());
    let host = std::env::var("VPNNODE_CLIENT_PROBE_HOST").unwrap_or_else(|_| "probe".to_string());
    format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n").into_bytes()
}

#[derive(Debug, Clone)]
struct HttpRoundtripOutcome {
    response: Vec<u8>,
    first_client_byte_ms: Option<u64>,
    total_ms: u64,
    completion_reason: &'static str,
    status_code: Option<u16>,
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
        // Channel carries response payloads and stream-finish control signals.
        let response_senders: ResponseSenderTable = Arc::new(Mutex::new(HashMap::new()));
        // Per-stream reliable state.
        let reliable_streams: ReliableStreamTable = Arc::new(Mutex::new(HashMap::new()));
        // Track which streams have received at least one response chunk (optional).
        let response_started: ResponseStartedTable = Arc::new(Mutex::new(HashMap::new()));
        // Stage 6: track per-stream route for ACKs/retransmits.
        let stream_routes_for_io: StreamRouteTable = Arc::new(Mutex::new(HashMap::new()));
        let completed_response_streams: CompletedResponseStreamTable =
            Arc::new(Mutex::new(HashMap::new()));
        let close_signal_seen: CloseSignalSeenTable = Arc::new(Mutex::new(HashMap::new()));

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
        let close_signal_seen_recv = close_signal_seen.clone();
        let default_route_for_ack = primary_route.clone();
        let route_store_for_ants = route_store.clone();
        let route_store_for_quality = route_store.clone();
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
                        let reject = classify_open_message_error(&e);
                        if reject.kind == SessionOpenRejectKind::Duplicate {
                            emit_client_stage(
                                "duplicate_packet_dropped",
                                json!({
                                    "peer": from.to_string(),
                                    "seq": reject.seq,
                                    "highest": reject.highest,
                                    "behind": reject.behind,
                                    "window": reject.window,
                                }),
                            );
                            debug!(
                                peer = %from,
                                seq = ?reject.seq,
                                highest = ?reject.highest,
                                behind = ?reject.behind,
                                window = ?reject.window,
                                "client dropped duplicate packet before decode"
                            );
                            continue;
                        }
                        emit_client_stage(
                            "open_message_failed",
                            json!({
                                "peer": from.to_string(),
                                "error": e.to_string(),
                            }),
                        );
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

                    let dispatch = resolve_response_data_dispatch(
                        sid,
                        &response_senders_recv,
                        &reliable_streams_recv,
                        &response_started_recv,
                        &stream_routes_for_ack,
                        &completed_response_streams_recv,
                        &default_route_for_ack,
                    );
                    let (
                        rs_arc,
                        route_for_this_stream,
                        maybe_tx,
                        completion_reason,
                        orphaned_without_consumer,
                        pruned_stale_active_state,
                    ) = match dispatch {
                        ResponseDataDispatch::Active {
                            tx,
                            reliable,
                            route,
                        } => (reliable, route, Some(tx), None, false, false),
                        ResponseDataDispatch::Completed {
                            reliable,
                            route,
                            completion_reason,
                            pruned_stale_active_state,
                        } => (
                            reliable,
                            route,
                            None,
                            Some(completion_reason),
                            false,
                            pruned_stale_active_state,
                        ),
                        ResponseDataDispatch::Orphaned {
                            reliable,
                            route,
                            pruned_stale_active_state,
                        } => (reliable, route, None, None, true, pruned_stale_active_state),
                        ResponseDataDispatch::Unknown => {
                            emit_client_stage(
                                "response_payload_unknown_stream",
                                json!({
                                    "stream_id": sid,
                                    "frame_seq": frame.frame_seq,
                                    "payload_len": frame.payload.len(),
                                }),
                            );
                            debug!(
                                stream_id = sid,
                                frame_seq = frame.frame_seq,
                                payload_len = frame.payload.len(),
                                "client: response payload for unknown stream"
                            );
                            continue;
                        }
                    };

                    if pruned_stale_active_state {
                        emit_client_stage(
                            "stale_active_response_state_pruned",
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
                            }),
                        );
                    }

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
                                let _ = tx.send(ResponseEvent::Payload(chunk));
                            }
                        }
                        if end_of_stream {
                            debug!(stream_id = sid, "client: end-of-stream from reliable layer");
                            let _ = tx.send(ResponseEvent::EndOfStream);
                        }
                    } else if let Some(completion_reason) = completion_reason {
                        let late_bytes: usize = deliver.iter().map(|chunk| chunk.len()).sum();
                        let terminal_after_local_completion =
                            completion_reason_uses_terminal_settlement(completion_reason)
                                && frame.payload.is_empty()
                                && late_bytes == 0;
                        let duplicate_payload_after_local_completion =
                            completion_reason_uses_terminal_settlement(completion_reason)
                                && late_bytes == 0
                                && !end_of_stream
                                && frame.frame_seq < ack_seq;
                        if terminal_after_local_completion {
                            if let Some((
                                completed_age_ms,
                                _,
                                first_signal,
                                terminal_signal_count,
                            )) = note_completed_response_terminal_payload(
                                &completed_response_streams_recv,
                                sid,
                            ) {
                                let stage = if first_signal {
                                    "terminal_payload_after_local_completion"
                                } else {
                                    "duplicate_terminal_payload_after_local_completion"
                                };
                                emit_client_stage(
                                    stage,
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
                                        "completed_age_ms": completed_age_ms,
                                        "completion_reason": completion_reason,
                                        "end_of_stream": end_of_stream,
                                        "terminal_signal_count": terminal_signal_count,
                                        "duplicate_like": !first_signal || !end_of_stream,
                                    }),
                                );
                                debug!(
                                    stream_id = sid,
                                    frame_seq = frame.frame_seq,
                                    ack_seq,
                                    end_of_stream,
                                    completion_reason,
                                    completed_age_ms,
                                    terminal_signal_count,
                                    first_signal,
                                    "client: terminal payload signal after local completion"
                                );
                            }
                        } else if duplicate_payload_after_local_completion {
                            if let Some((
                                completed_age_ms,
                                _,
                                first_signal,
                                duplicate_payload_count,
                            )) = note_completed_response_duplicate_payload(
                                &completed_response_streams_recv,
                                sid,
                            ) {
                                let stage = if first_signal {
                                    "duplicate_payload_after_local_completion"
                                } else {
                                    "duplicate_payload_after_local_completion_repeat"
                                };
                                emit_client_stage(
                                    stage,
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
                                        "completed_age_ms": completed_age_ms,
                                        "completion_reason": completion_reason,
                                        "duplicate_payload_count": duplicate_payload_count,
                                    }),
                                );
                                debug!(
                                    stream_id = sid,
                                    frame_seq = frame.frame_seq,
                                    ack_seq,
                                    completion_reason,
                                    completed_age_ms,
                                    duplicate_payload_count,
                                    first_signal,
                                    "client: duplicate payload signal after local completion"
                                );
                            }
                        } else {
                            let (completed_age_ms, _, late_count) =
                                note_completed_response_late_payload(
                                    &completed_response_streams_recv,
                                    sid,
                                )
                                .unwrap_or((
                                    0,
                                    completion_reason,
                                    0,
                                ));
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
                        }
                    } else if orphaned_without_consumer {
                        let late_bytes: usize = deliver.iter().map(|chunk| chunk.len()).sum();
                        emit_client_stage(
                            "orphaned_response_payload",
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
                                "duplicate_like": late_bytes == 0 && !end_of_stream,
                            }),
                        );
                        debug!(
                            stream_id = sid,
                            frame_seq = frame.frame_seq,
                            ack_seq,
                            delivered_bytes = late_bytes,
                            end_of_stream,
                            "client: orphaned response payload handled without active consumer"
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
                    match resolve_response_ack_dispatch(
                        ack.stream_id,
                        &response_senders_recv,
                        &reliable_streams_recv,
                        &response_started_recv,
                        &stream_routes_for_ack,
                        &completed_response_streams_recv,
                    ) {
                        ResponseAckDispatch::Active(rs) => {
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
                        }
                        ResponseAckDispatch::Completed {
                            reliable,
                            completion_reason,
                            pruned_stale_active_state,
                        } => {
                            if pruned_stale_active_state {
                                emit_client_stage(
                                    "stale_active_response_state_pruned",
                                    json!({
                                        "stream_id": ack.stream_id,
                                        "ack_seq": ack.ack_seq,
                                        "source": "ack",
                                    }),
                                );
                            }
                            let mut guard = reliable.lock().unwrap();
                            let (acked, avg_latency_ms) = guard.apply_ack_with_latency(ack.ack_seq);
                            emit_client_stage(
                                "late_ack_after_completion",
                                json!({
                                    "stream_id": ack.stream_id,
                                    "ack_seq": ack.ack_seq,
                                    "acked_frames": acked,
                                    "ack_latency_ms_avg": avg_latency_ms,
                                    "completion_reason": completion_reason,
                                }),
                            );
                            debug!(
                                stream_id = ack.stream_id,
                                ack_seq = ack.ack_seq,
                                acked_frames = acked,
                                ack_latency_ms_avg = ?avg_latency_ms,
                                completion_reason,
                                "client: late ACK applied after completion"
                            );
                        }
                        ResponseAckDispatch::Unknown {
                            pruned_stale_active_state,
                        } => {
                            if pruned_stale_active_state {
                                emit_client_stage(
                                    "stale_active_response_state_pruned",
                                    json!({
                                        "stream_id": ack.stream_id,
                                        "ack_seq": ack.ack_seq,
                                        "source": "ack",
                                    }),
                                );
                            }
                            emit_client_stage(
                                "ack_for_unknown_stream",
                                json!({
                                    "stream_id": ack.stream_id,
                                    "ack_seq": ack.ack_seq,
                                }),
                            );
                            debug!(
                                stream_id = ack.stream_id,
                                ack_seq = ack.ack_seq,
                                "client: ACK for unknown stream ignored"
                            );
                        }
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
                        let _ = tx.send(ResponseEvent::Payload(full));
                        let _ = tx.send(ResponseEvent::CloseStream);
                    }
                } else if msg_type == MsgType::CloseStream {
                    // Treat CloseStream as end-of-response as well. This makes the TCP-mode
                    // client robust even if the empty DATA end marker is lost.
                    let active_route = {
                        let map = stream_routes_for_ack.lock().unwrap();
                        map.get(&sid).cloned()
                    };
                    let completed_entry =
                        snapshot_completed_response_stream(&completed_response_streams_recv, sid);
                    let route_for_quality = active_route
                        .clone()
                        .or_else(|| completed_entry.as_ref().map(|entry| entry.route.clone()));
                    let quality_feedback = if msg.payload.is_empty() {
                        None
                    } else {
                        match bincode::deserialize::<ResponseQualityFeedback>(&msg.payload) {
                            Ok(feedback) => Some(feedback),
                            Err(e) => {
                                error!(
                                    stream_id = sid,
                                    payload_len = msg.payload.len(),
                                    %e,
                                    "client: failed to decode ResponseQualityFeedback"
                                );
                                None
                            }
                        }
                    };
                    let first_close_signal = mark_close_signal_seen(&close_signal_seen_recv, sid);
                    if let (Some(route_for_quality), Some(feedback)) =
                        (route_for_quality.as_ref(), quality_feedback.as_ref())
                    {
                        let applied = if first_close_signal {
                            let mut store = route_store_for_quality.lock().await;
                            store.record_response_quality_for_route(route_for_quality, feedback)
                        } else {
                            false
                        };
                        emit_client_stage(
                            "route_quality_feedback_received",
                            json!({
                                "stream_id": sid,
                                "route_len": route_for_quality.len(),
                                "route_chain": route_for_quality
                                    .hops
                                    .iter()
                                    .map(|hop| hop.to_string())
                                    .collect::<Vec<_>>()
                                    .join(" -> "),
                                "feedback_applied": applied,
                                "duplicate_close_signal": !first_close_signal,
                                "ack_latency_ms_p95": feedback.ack_latency_ms_p95,
                                "retransmit_rate_ppm": feedback.retransmit_rate_ppm,
                                "window_wait_total_ms": feedback.window_wait_total_ms,
                                "stream_duration_ms": feedback.stream_duration_ms,
                                "http_code": feedback.http_code,
                            }),
                        );
                    }
                    let maybe_tx = {
                        let map = response_senders_recv.lock().unwrap();
                        map.get(&sid).cloned()
                    };
                    if !first_close_signal {
                        emit_client_stage(
                            "duplicate_close_stream_suppressed",
                            json!({
                                "stream_id": sid,
                                "has_active_consumer": maybe_tx.is_some(),
                                "completed_state_present": completed_entry.is_some(),
                                "route_len": route_for_quality.as_ref().map(|route| route.len()),
                                "route_chain": route_for_quality.as_ref().map(|route| route
                                    .hops
                                    .iter()
                                    .map(|hop| hop.to_string())
                                    .collect::<Vec<_>>()
                                    .join(" -> ")),
                                "completion_reason": completed_entry
                                    .as_ref()
                                    .map(|entry| entry.completion_reason),
                            }),
                        );
                        debug!(
                            stream_id = sid,
                            has_active_consumer = maybe_tx.is_some(),
                            completed_state_present = completed_entry.is_some(),
                            "client: duplicate CloseStream suppressed before lifecycle handling"
                        );
                    } else if let Some(tx) = maybe_tx {
                        debug!(
                            stream_id = sid,
                            "client: received CloseStream, ending response"
                        );
                        let _ = tx.send(ResponseEvent::CloseStream);
                    } else if let Some(completed_entry) =
                        snapshot_completed_response_stream(&completed_response_streams_recv, sid)
                    {
                        if completion_reason_uses_terminal_settlement(
                            completed_entry.completion_reason,
                        ) {
                            let (
                                completed_age_ms,
                                completion_reason,
                                first_signal,
                                terminal_close_count,
                            ) = note_completed_response_terminal_close(
                                &completed_response_streams_recv,
                                sid,
                            )
                            .unwrap_or((
                                0,
                                completed_entry.completion_reason,
                                true,
                                0,
                            ));
                            let stage = if first_signal {
                                "terminal_close_after_local_completion"
                            } else {
                                "duplicate_terminal_close_after_local_completion"
                            };
                            emit_client_stage(
                                stage,
                                json!({
                                    "stream_id": sid,
                                    "completed_age_ms": completed_age_ms,
                                    "completion_reason": completion_reason,
                                    "terminal_close_count": terminal_close_count,
                                }),
                            );
                            debug!(
                                stream_id = sid,
                                completed_age_ms,
                                completion_reason,
                                terminal_close_count,
                                first_signal,
                                "client: terminal CloseStream after local completion"
                            );
                        } else if let Some((
                            completed_age_ms,
                            completion_reason,
                            late_close_count,
                        )) =
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
                let rtt_ms = res
                    .as_ref()
                    .map(|outcome| outcome.total_ms)
                    .unwrap_or_else(|_| 0);
                let ok = res
                    .as_ref()
                    .ok()
                    .and_then(|outcome| outcome.status_code)
                    .map(|code| code >= 200 && code < 500)
                    .unwrap_or(false);
                let mut store = route_store_probe.lock().await;
                store.update_metrics(idx, Some(rtt_ms), ok);
                if let Ok(outcome) = &res {
                    store.record_local_observation(
                        idx,
                        &LocalRouteObservation {
                            success: ok,
                            status_code: outcome.status_code,
                            ttfb_ms: outcome.first_client_byte_ms,
                            total_ms: outcome.total_ms,
                            response_bytes: outcome.response.len(),
                        },
                    );
                } else {
                    store.record_local_observation(
                        idx,
                        &LocalRouteObservation {
                            success: false,
                            status_code: None,
                            ttfb_ms: None,
                            total_ms: 0,
                            response_bytes: 0,
                        },
                    );
                }
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
                let (decision, candidate_snapshot) = {
                    let mut store = route_store.lock().await;
                    let scored = store.scored_candidates(now, &excluded);
                    let candidate_snapshot = scored
                        .iter()
                        .map(|(candidate_idx, details)| {
                            let route = &store.routes()[*candidate_idx].route;
                            json!({
                                "candidate_idx": candidate_idx,
                                "route_len": route.len(),
                                "route_chain": route
                                    .hops
                                    .iter()
                                    .map(|hop| hop.to_string())
                                    .collect::<Vec<_>>()
                                    .join(" -> "),
                                "final_score": details.final_score,
                                "base": details.base,
                                "transport_agg": details.transport_agg,
                                "quality_agg": details.quality_agg,
                                "hop_factor": details.hop_factor,
                                "quality_confidence": details.quality_confidence,
                                "warmup_confidence": details.warmup_confidence,
                                "instability_factor": details.instability_factor,
                                "metric_instability_ppm": details.metric_instability_ppm,
                                "flap_penalty_factor": details.flap_penalty_factor,
                                "recent_flap_count": details.recent_flap_count,
                                "recent_ttfb_ms": details.recent_ttfb_ms,
                                "recent_total_ms": details.recent_total_ms,
                                "recent_ack_p95_ms": details.recent_ack_p95_ms,
                                "recent_retransmit_rate_ppm": details.recent_retransmit_rate_ppm,
                                "recent_window_wait_ratio_ppm": details.recent_window_wait_ratio_ppm,
                                "recent_success_count": details.recent_success_count,
                                "recent_failure_count": details.recent_failure_count,
                            })
                        })
                        .collect::<Vec<_>>();
                    let decision = store.apply_selection_policy(&scored, now);
                    (decision, candidate_snapshot)
                };
                let decision_json = decision.as_ref().map(|decision| {
                    let selected_route = candidate_snapshot
                        .iter()
                        .find(|candidate| {
                            candidate
                                .get("candidate_idx")
                                .and_then(|value| value.as_u64())
                                == Some(decision.selected_idx as u64)
                        })
                        .and_then(|candidate| candidate.get("route_chain"))
                        .and_then(|value| value.as_str())
                        .unwrap_or("<missing>")
                        .to_string();
                    let best_route = candidate_snapshot
                        .iter()
                        .find(|candidate| {
                            candidate
                                .get("candidate_idx")
                                .and_then(|value| value.as_u64())
                                == Some(decision.best_idx as u64)
                        })
                        .and_then(|candidate| candidate.get("route_chain"))
                        .and_then(|value| value.as_str())
                        .unwrap_or("<missing>")
                        .to_string();
                    let previous_route = decision.previous_idx.and_then(|previous_idx| {
                        candidate_snapshot
                            .iter()
                            .find(|candidate| {
                                candidate
                                    .get("candidate_idx")
                                    .and_then(|value| value.as_u64())
                                    == Some(previous_idx as u64)
                            })
                            .and_then(|candidate| candidate.get("route_chain"))
                            .and_then(|value| value.as_str())
                            .map(|value| value.to_string())
                    });
                    json!({
                        "decision_reason": decision.decision_reason,
                        "tie_break_reason": decision.tie_break_reason,
                        "switched": decision.switched,
                        "selected_idx": decision.selected_idx,
                        "selected_route": selected_route,
                        "best_idx": decision.best_idx,
                        "best_route": best_route,
                        "best_score": decision.best_details.final_score,
                        "previous_idx": decision.previous_idx,
                        "previous_route": previous_route,
                        "previous_score": decision.previous_details.as_ref().map(|details| details.final_score),
                        "score_delta_abs": decision.score_delta_abs,
                        "score_delta_ratio": decision.score_delta_ratio,
                        "required_abs_margin": decision.required_abs_margin,
                        "required_rel_margin": decision.required_rel_margin,
                        "hold_remaining_ms": decision.hold_remaining_ms,
                        "selected_quality_confidence": decision.selected_details.quality_confidence,
                        "best_quality_confidence": decision.best_details.quality_confidence,
                        "previous_quality_confidence": decision.previous_details.as_ref().map(|details| details.quality_confidence),
                        "selected_metric_instability_ppm": decision.selected_details.metric_instability_ppm,
                        "best_metric_instability_ppm": decision.best_details.metric_instability_ppm,
                        "previous_metric_instability_ppm": decision.previous_details.as_ref().and_then(|details| details.metric_instability_ppm),
                    })
                });
                emit_client_stage(
                    "route_candidates_scored",
                    json!({
                        "stream_id": sid0,
                        "site": request_site.as_str(),
                        "attempt": attempt,
                        "excluded_routes": excluded
                            .iter()
                            .map(|route| route
                                .hops
                                .iter()
                                .map(|hop| hop.to_string())
                                .collect::<Vec<_>>()
                                .join(" -> "))
                            .collect::<Vec<_>>(),
                        "candidates": candidate_snapshot,
                        "selection_policy": decision_json,
                    }),
                );
                let Some(decision) = decision else {
                    break;
                };
                let idx = decision.selected_idx;
                let details = decision.selected_details.clone();
                let score = details.final_score;
                let route = {
                    let store = route_store.lock().await;
                    store.routes().get(idx).map(|c| c.route.clone())
                };
                let Some(route) = route else { break };
                let hops = route.len();
                info!(
                    stream_id = sid0,
                    attempt,
                    hops,
                    score = details.final_score,
                    base = details.base,
                    transport_agg = details.transport_agg,
                    quality_agg = details.quality_agg,
                    hop_factor = details.hop_factor,
                    recent_ttfb_ms = ?details.recent_ttfb_ms,
                    recent_total_ms = ?details.recent_total_ms,
                    recent_ack_p95_ms = ?details.recent_ack_p95_ms,
                    recent_retransmit_rate_ppm = ?details.recent_retransmit_rate_ppm,
                    recent_window_wait_ratio_ppm = ?details.recent_window_wait_ratio_ppm,
                    recent_success_count = details.recent_success_count,
                    recent_failure_count = details.recent_failure_count,
                    decision_reason = decision.decision_reason,
                    switched = decision.switched,
                    score_delta_abs = decision.score_delta_abs,
                    score_delta_ratio = decision.score_delta_ratio,
                    "route selected: quality-aware policy applied"
                );
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
                        "base": details.base,
                        "transport_agg": details.transport_agg,
                        "quality_agg": details.quality_agg,
                        "hop_factor": details.hop_factor,
                        "recent_ttfb_ms": details.recent_ttfb_ms,
                        "recent_total_ms": details.recent_total_ms,
                        "recent_ack_p95_ms": details.recent_ack_p95_ms,
                        "recent_retransmit_rate_ppm": details.recent_retransmit_rate_ppm,
                        "recent_window_wait_ratio_ppm": details.recent_window_wait_ratio_ppm,
                        "recent_success_count": details.recent_success_count,
                        "recent_failure_count": details.recent_failure_count,
                        "decision_reason": decision.decision_reason,
                        "tie_break_reason": decision.tie_break_reason,
                        "switched": decision.switched,
                        "best_idx": decision.best_idx,
                        "best_score": decision.best_details.final_score,
                        "previous_idx": decision.previous_idx,
                        "previous_score": decision.previous_details.as_ref().map(|prev| prev.final_score),
                        "score_delta_abs": decision.score_delta_abs,
                        "score_delta_ratio": decision.score_delta_ratio,
                        "required_abs_margin": decision.required_abs_margin,
                        "required_rel_margin": decision.required_rel_margin,
                        "hold_remaining_ms": decision.hold_remaining_ms,
                        "selected_quality_confidence": decision.selected_details.quality_confidence,
                        "best_quality_confidence": decision.best_details.quality_confidence,
                        "previous_quality_confidence": decision.previous_details.as_ref().map(|details| details.quality_confidence),
                        "selected_metric_instability_ppm": decision.selected_details.metric_instability_ppm,
                        "best_metric_instability_ppm": decision.best_details.metric_instability_ppm,
                        "previous_metric_instability_ppm": decision.previous_details.as_ref().and_then(|details| details.metric_instability_ppm),
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
                    Ok(outcome) => {
                        let code = outcome.status_code;
                        // Stage 6 failure rules: treat route-level timeouts (504) as route failure.
                        // Do NOT treat upstream target errors (502) as route failure.
                        let is_fail = matches!(code, Some(504));
                        {
                            let mut store = route_store.lock().await;
                            store.record_local_observation(
                                idx,
                                &LocalRouteObservation {
                                    success: !is_fail,
                                    status_code: code,
                                    ttfb_ms: outcome.first_client_byte_ms,
                                    total_ms: outcome.total_ms,
                                    response_bytes: outcome.response.len(),
                                },
                            );
                            if is_fail {
                                store.record_failure(idx, RouteFailureKind::Timeout);
                            } else {
                                store.record_success(idx, Some(outcome.total_ms));
                            }
                        }
                        if is_fail {
                            excluded.push(route.clone());
                            info!(
                                stream_id = sid0,
                                attempt,
                                hops,
                                score,
                                code = ?code,
                                total_ms = outcome.total_ms,
                                completion_reason = outcome.completion_reason,
                                "route failed, switching"
                            );
                            continue;
                        }
                        final_resp = Some(outcome.response);
                        used_idx = Some(idx);
                        used_score = score;
                        used_hops = hops;
                        rtt_ms = Some(outcome.total_ms);
                        break;
                    }
                    Err(e) => {
                        excluded.push(route.clone());
                        let elapsed = start_rtt.elapsed().as_millis() as u64;
                        {
                            let mut store = route_store.lock().await;
                            store.record_local_observation(
                                idx,
                                &LocalRouteObservation {
                                    success: false,
                                    status_code: None,
                                    ttfb_ms: None,
                                    total_ms: elapsed,
                                    response_bytes: 0,
                                },
                            );
                            store.record_failure(idx, classify_anyhow_failure(&e));
                        }
                        info!(
                            stream_id = sid0,
                            attempt,
                            hops,
                            score,
                            total_ms = elapsed,
                            %e,
                            "route failed, switching"
                        );
                    }
                }
            }

            if let (Some(_idx), Some(resp)) = (used_idx, final_resp.as_ref()) {
                info!(
                    stream_id = sid0,
                    hops = used_hops,
                    score = used_score,
                    rtt_ms = rtt_ms.unwrap_or(0),
                    status_code = ?http_status_code(resp),
                    "route success recorded"
                );
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
                        Ok(outcome) => {
                            if outcome.response.is_empty() {
                                continue; // empty response is OK for TLS ACKs
                            }
                            if tcp.write_all(&outcome.response).await.is_err() {
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
) -> Result<HttpRoundtripOutcome> {
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

    let (tx_from_udp, mut rx_from_udp_stream) = mpsc::unbounded_channel::<ResponseEvent>();
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
    let mut buffered_header_end: Option<usize> = None;
    let mut buffered_content_length: Option<usize> = None;
    let mut buffered_expected_total: Option<usize> = None;
    let mut buffered_fallback_cap = request
        .len()
        .saturating_add(BUFFERED_HTTP_RESPONSE_SOFT_CAP_SLACK);
    let mut buffered_active_cap = buffered_fallback_cap;
    let mut buffered_hard_cap = buffered_fallback_cap.max(BUFFERED_HTTP_RESPONSE_HARD_CAP);
    let mut buffered_content_length_exceeds_cap = false;
    let mut buffered_completion_reason_hint: Option<&'static str> = None;
    let mut buffered_pending_terminal_close = false;
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
            ResponseEvent::Payload(chunk) => {
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
                    let inspection = inspect_buffered_http_response(request.len(), &full);
                    buffered_fallback_cap = inspection.fallback_cap;
                    buffered_hard_cap = inspection.hard_cap;
                    buffered_active_cap = inspection.active_cap;
                    buffered_content_length_exceeds_cap = inspection.content_length_exceeds_cap;

                    if inspection.header_end != buffered_header_end {
                        buffered_header_end = inspection.header_end;
                        emit_client_stage(
                            "buffered_headers_parsed",
                            json!({
                                "stream_id": stream_id,
                                "site": request_site.as_str(),
                                "route_len": route.len(),
                                "route_chain": route_chain.as_str(),
                                "response_bytes_total": full.len(),
                                "header_end": inspection.header_end,
                                "content_length_detected": inspection.content_length.is_some(),
                                "content_length": inspection.content_length,
                                "expected_total": inspection.expected_total,
                                "fallback_cap": inspection.fallback_cap,
                                "active_cap": inspection.active_cap,
                                "hard_cap": inspection.hard_cap,
                            }),
                        );
                    }

                    if inspection.content_length != buffered_content_length {
                        buffered_content_length = inspection.content_length;
                        buffered_expected_total = inspection.expected_total;
                        if inspection.content_length.is_some() {
                            emit_client_stage(
                                "buffered_content_length_detected",
                                json!({
                                    "stream_id": stream_id,
                                    "site": request_site.as_str(),
                                    "route_len": route.len(),
                                    "route_chain": route_chain.as_str(),
                                    "response_bytes_total": full.len(),
                                    "header_end": inspection.header_end,
                                    "content_length": inspection.content_length,
                                    "expected_total": inspection.expected_total,
                                    "fallback_cap": inspection.fallback_cap,
                                    "active_cap": inspection.active_cap,
                                    "hard_cap": inspection.hard_cap,
                                    "content_length_exceeds_cap": inspection.content_length_exceeds_cap,
                                }),
                            );
                        }
                    }

                    if let Some(reason) = inspection.completion_reason {
                        buffered_completion_reason_hint = Some(reason);
                        break reason;
                    }
                }
            }
            ResponseEvent::EndOfStream => {
                if !write_response_to_tcp {
                    buffered_completion_reason_hint = Some("peer_closed");
                }
                break "peer_closed";
            }
            ResponseEvent::CloseStream => {
                if !write_response_to_tcp {
                    if full.is_empty() {
                        let duplicate_queue = buffered_pending_terminal_close;
                        buffered_pending_terminal_close = true;
                        emit_client_stage(
                            "terminal_close_before_response_start_queued",
                            json!({
                                "stream_id": stream_id,
                                "site": request_site.as_str(),
                                "route_len": route.len(),
                                "route_chain": route_chain.as_str(),
                                "response_bytes_total": full.len(),
                                "content_length": buffered_content_length,
                                "expected_total": buffered_expected_total,
                                "duplicate_queue": duplicate_queue,
                            }),
                        );
                        continue;
                    }
                    if buffered_close_requires_wait(buffered_expected_total, full.len()) {
                        let duplicate_queue = buffered_pending_terminal_close;
                        buffered_pending_terminal_close = true;
                        emit_client_stage(
                            "terminal_close_before_local_completion_queued",
                            json!({
                                "stream_id": stream_id,
                                "site": request_site.as_str(),
                                "route_len": route.len(),
                                "route_chain": route_chain.as_str(),
                                "response_bytes_total": full.len(),
                                "content_length": buffered_content_length,
                                "expected_total": buffered_expected_total,
                                "duplicate_queue": duplicate_queue,
                            }),
                        );
                        continue;
                    }
                    buffered_completion_reason_hint = Some("peer_closed");
                }
                break "peer_closed";
            }
        }
    };

    let transport_settlement_handed_off =
        !write_response_to_tcp && completion_reason_uses_terminal_settlement(completion_reason);
    if transport_settlement_handed_off {
        emit_client_stage(
            "response_transport_local_completion",
            json!({
                "stream_id": stream_id,
                "site": request_site.as_str(),
                "route_len": route.len(),
                "route_chain": route_chain.as_str(),
                "completion_reason": completion_reason,
                "response_bytes_total": full.len(),
            }),
        );
        let response_senders = response_senders.clone();
        let reliable_streams = reliable_streams.clone();
        let response_started = response_started.clone();
        let stream_routes = stream_routes.clone();
        let completed_response_streams = completed_response_streams.clone();
        let route_for_settlement = route.clone();
        let request_site_for_settlement = request_site.clone();
        let rs_arc_for_settlement = rs_arc.clone();
        let response_bytes_total = full.len();
        tokio::spawn(async move {
            settle_buffered_response_transport(
                stream_id,
                request_site_for_settlement,
                route_for_settlement,
                completion_reason,
                response_bytes_total,
                buffered_pending_terminal_close,
                rs_arc_for_settlement,
                rx_from_udp_stream,
                response_senders,
                reliable_streams,
                response_started,
                stream_routes,
                completed_response_streams,
            )
            .await;
        });
    } else {
        finalize_response_stream_cleanup(
            response_senders,
            reliable_streams,
            response_started,
            stream_routes,
            completed_response_streams,
            stream_id,
            rs_arc.clone(),
            route.clone(),
            completion_reason,
        );
        emit_client_stage(
            "response_channel_removed",
            json!({
                "stream_id": stream_id,
                "site": request_site.as_str(),
                "route_len": route.len(),
                "route_chain": route_chain.as_str(),
                "completion_reason": completion_reason,
                "write_response_to_tcp": write_response_to_tcp,
                "transport_settled": false,
            }),
        );
    }
    if !write_response_to_tcp {
        emit_client_stage(
            "buffered_completion_decision",
            json!({
                "stream_id": stream_id,
                "site": request_site.as_str(),
                "route_len": route.len(),
                "route_chain": route_chain.as_str(),
                "response_bytes_total": full.len(),
                "header_end": buffered_header_end,
                "content_length": buffered_content_length,
                "expected_total": buffered_expected_total,
                "fallback_cap": buffered_fallback_cap,
                "active_cap": buffered_active_cap,
                "hard_cap": buffered_hard_cap,
                "content_length_exceeds_cap": buffered_content_length_exceeds_cap,
                "completion_reason": buffered_completion_reason_hint.unwrap_or(completion_reason),
            }),
        );
    }

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
        Ok(HttpRoundtripOutcome {
            response: Vec::new(),
            first_client_byte_ms,
            total_ms: stream_start.elapsed().as_millis() as u64,
            completion_reason,
            status_code: None,
        })
    } else {
        Ok(HttpRoundtripOutcome {
            status_code: http_status_code(&full),
            response: full,
            first_client_byte_ms,
            total_ms: stream_start.elapsed().as_millis() as u64,
            completion_reason,
        })
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
                    let reject = classify_open_message_error(&e);
                    if reject.kind == SessionOpenRejectKind::Duplicate {
                        debug!(
                            seq = ?reject.seq,
                            highest = ?reject.highest,
                            behind = ?reject.behind,
                            window = ?reject.window,
                            "client tun dropped duplicate packet before decode"
                        );
                        continue;
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

    fn response_with_content_length(body_len: usize) -> Vec<u8> {
        let header =
            format!("HTTP/1.1 200 OK\r\nContent-Length: {body_len}\r\nConnection: close\r\n\r\n");
        let mut response = Vec::with_capacity(header.len() + body_len);
        response.extend_from_slice(header.as_bytes());
        response.extend(std::iter::repeat_n(b'X', body_len));
        response
    }

    #[test]
    fn buffered_completion_uses_content_length_for_small_response() {
        let request = b"GET / HTTP/1.1\r\nHost: example\r\nConnection: close\r\n\r\n";
        let response = response_with_content_length(64);
        let inspection = inspect_buffered_http_response(request.len(), &response);

        assert_eq!(inspection.content_length, Some(64));
        assert_eq!(inspection.expected_total, Some(response.len()));
        assert_eq!(inspection.completion_reason, Some("content_length_reached"));
        assert!(!inspection.content_length_exceeds_cap);
    }

    #[test]
    fn buffered_completion_uses_content_length_for_medium_response() {
        let request =
            b"GET /perf-262144.bin HTTP/1.1\r\nHost: example\r\nConnection: close\r\n\r\n";
        let response = response_with_content_length(262_144);
        let inspection = inspect_buffered_http_response(request.len(), &response);

        assert_eq!(inspection.content_length, Some(262_144));
        assert_eq!(inspection.expected_total, Some(response.len()));
        assert_eq!(inspection.completion_reason, Some("content_length_reached"));
        assert!(
            inspection.active_cap > response.len(),
            "content-length aware cap must leave headroom past the exact response size"
        );
    }

    #[test]
    fn buffered_completion_does_not_stop_at_old_soft_cap_before_content_length_total() {
        let request =
            b"GET /perf-262144.bin HTTP/1.1\r\nHost: example\r\nConnection: close\r\n\r\n";
        let full_response =
            response_with_content_length(BUFFERED_HTTP_RESPONSE_SOFT_CAP_SLACK + 256);
        let old_soft_cap = request
            .len()
            .saturating_add(BUFFERED_HTTP_RESPONSE_SOFT_CAP_SLACK);
        assert!(
            full_response.len() > old_soft_cap,
            "test response must exceed the old soft cap to reproduce the premature-cap scenario"
        );
        let partial = &full_response[..old_soft_cap];
        let inspection = inspect_buffered_http_response(request.len(), partial);

        assert_eq!(
            inspection.content_length,
            Some(BUFFERED_HTTP_RESPONSE_SOFT_CAP_SLACK + 256)
        );
        assert_eq!(inspection.expected_total, Some(full_response.len()));
        assert_eq!(
            inspection.completion_reason,
            None,
            "buffered completion must keep waiting when Content-Length says more bytes are expected"
        );
    }

    #[test]
    fn buffered_completion_can_use_cap_as_safety_fallback_for_oversized_response() {
        let request = b"GET /huge HTTP/1.1\r\nHost: example\r\nConnection: close\r\n\r\n";
        let oversize_body = BUFFERED_HTTP_RESPONSE_HARD_CAP + 1024;
        let header = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {oversize_body}\r\nConnection: close\r\n\r\n"
        );
        let mut partial = Vec::with_capacity(BUFFERED_HTTP_RESPONSE_HARD_CAP);
        partial.extend_from_slice(header.as_bytes());
        partial.extend(std::iter::repeat_n(
            b'Y',
            BUFFERED_HTTP_RESPONSE_HARD_CAP.saturating_sub(header.len()),
        ));
        let inspection = inspect_buffered_http_response(request.len(), &partial);

        assert!(inspection.content_length_exceeds_cap);
        assert_eq!(inspection.completion_reason, Some("buffer_cap_reached"));
        assert_eq!(inspection.active_cap, inspection.hard_cap);
    }

    #[test]
    fn buffered_close_waits_for_remaining_content_length_bytes() {
        assert!(buffered_close_requires_wait(Some(1024), 1000));
        assert!(buffered_close_requires_wait(Some(1024), 0));
        assert!(!buffered_close_requires_wait(Some(1024), 1024));
        assert!(!buffered_close_requires_wait(Some(1024), 1400));
    }

    #[test]
    fn buffered_close_without_content_length_finishes_immediately() {
        assert!(!buffered_close_requires_wait(None, 0));
        assert!(!buffered_close_requires_wait(None, 512));
    }

    #[test]
    fn close_signal_is_seen_once_per_stream() {
        let seen: CloseSignalSeenTable = Arc::new(Mutex::new(HashMap::new()));
        assert!(mark_close_signal_seen(&seen, 7));
        assert!(
            !mark_close_signal_seen(&seen, 7),
            "duplicate CloseStream must not re-enter lifecycle handling"
        );
        assert!(mark_close_signal_seen(&seen, 8));
    }

    #[test]
    fn response_data_dispatch_prefers_completed_stream_over_stale_active_state() {
        let stream_id = 77u32;
        let response_senders: ResponseSenderTable = Arc::new(Mutex::new(HashMap::new()));
        let reliable_streams: ReliableStreamTable = Arc::new(Mutex::new(HashMap::new()));
        let response_started: ResponseStartedTable = Arc::new(Mutex::new(HashMap::new()));
        let stream_routes: StreamRouteTable = Arc::new(Mutex::new(HashMap::new()));
        let completed: CompletedResponseStreamTable = Arc::new(Mutex::new(HashMap::new()));

        let completed_reliable = Arc::new(Mutex::new(ReliableStream::new()));
        remember_completed_response_stream(
            &completed,
            stream_id,
            completed_reliable.clone(),
            test_route(),
            "content_length_satisfied",
        );

        let stale_reliable = Arc::new(Mutex::new(ReliableStream::new()));
        reliable_streams
            .lock()
            .unwrap()
            .insert(stream_id, stale_reliable.clone());
        stream_routes
            .lock()
            .unwrap()
            .insert(stream_id, test_route());

        match resolve_response_data_dispatch(
            stream_id,
            &response_senders,
            &reliable_streams,
            &response_started,
            &stream_routes,
            &completed,
            &test_route(),
        ) {
            ResponseDataDispatch::Completed {
                reliable,
                pruned_stale_active_state,
                ..
            } => {
                assert!(Arc::ptr_eq(&reliable, &completed_reliable));
                assert!(pruned_stale_active_state);
            }
            _ => panic!("expected completed dispatch"),
        }

        assert!(
            !reliable_streams.lock().unwrap().contains_key(&stream_id),
            "stale active reliable state must be pruned"
        );
    }

    #[test]
    fn late_ack_after_completion_uses_completed_stream_state() {
        let stream_id = 88u32;
        let response_senders: ResponseSenderTable = Arc::new(Mutex::new(HashMap::new()));
        let reliable_streams: ReliableStreamTable = Arc::new(Mutex::new(HashMap::new()));
        let response_started: ResponseStartedTable = Arc::new(Mutex::new(HashMap::new()));
        let stream_routes: StreamRouteTable = Arc::new(Mutex::new(HashMap::new()));
        let completed: CompletedResponseStreamTable = Arc::new(Mutex::new(HashMap::new()));

        let reliable = Arc::new(Mutex::new(ReliableStream::new()));
        {
            let mut guard = reliable.lock().unwrap();
            let _ = guard.build_outgoing_frame(stream_id, b"hello".to_vec());
            let _ = guard.build_outgoing_frame(stream_id, Vec::new());
        }
        remember_completed_response_stream(
            &completed,
            stream_id,
            reliable.clone(),
            test_route(),
            "content_length_satisfied",
        );

        match resolve_response_ack_dispatch(
            stream_id,
            &response_senders,
            &reliable_streams,
            &response_started,
            &stream_routes,
            &completed,
        ) {
            ResponseAckDispatch::Completed { reliable, .. } => {
                let mut guard = reliable.lock().unwrap();
                let (acked, _) = guard.apply_ack_with_latency(2);
                assert_eq!(acked, 2);
            }
            _ => panic!("expected completed ACK dispatch"),
        }

        assert!(
            reliable_streams.lock().unwrap().is_empty(),
            "late ACK must not recreate active reliable state"
        );
    }

    #[test]
    fn orphaned_active_state_without_consumer_is_pruned() {
        let stream_id = 99u32;
        let response_senders: ResponseSenderTable = Arc::new(Mutex::new(HashMap::new()));
        let reliable_streams: ReliableStreamTable = Arc::new(Mutex::new(HashMap::new()));
        let response_started: ResponseStartedTable = Arc::new(Mutex::new(HashMap::new()));
        let stream_routes: StreamRouteTable = Arc::new(Mutex::new(HashMap::new()));
        let completed: CompletedResponseStreamTable = Arc::new(Mutex::new(HashMap::new()));

        reliable_streams
            .lock()
            .unwrap()
            .insert(stream_id, Arc::new(Mutex::new(ReliableStream::new())));
        stream_routes
            .lock()
            .unwrap()
            .insert(stream_id, test_route());

        match resolve_response_data_dispatch(
            stream_id,
            &response_senders,
            &reliable_streams,
            &response_started,
            &stream_routes,
            &completed,
            &test_route(),
        ) {
            ResponseDataDispatch::Orphaned {
                pruned_stale_active_state,
                ..
            } => {
                assert!(pruned_stale_active_state);
            }
            _ => panic!("expected orphaned dispatch"),
        }

        assert!(
            reliable_streams.lock().unwrap().is_empty(),
            "orphaned active state must be pruned"
        );
    }
}
