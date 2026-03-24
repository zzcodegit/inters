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
use crate::session::{classify_open_message_error, SessionCrypto, SessionOpenRejectKind};
use crate::stage_trace;
use crate::stream_reliable::{
    AckDisposition, AckFrame, AckLatencySummary, ReliableStream, RetransmitBackoffSummary,
};
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
const EXIT_RESPONSE_RETRANSMIT_TIMEOUT_MIN_MS: u64 = 50;
const EXIT_RESPONSE_RETRANSMIT_TIMEOUT_MAX_MS: u64 = 1000;
const EXIT_RESPONSE_RETRANSMIT_FACTOR_NORMAL: f64 = 1.5;
const EXIT_RESPONSE_RETRANSMIT_FACTOR_SUSPECTED_LOSS: f64 = 2.0;
const EXIT_RESPONSE_RETRANSMIT_FACTOR_REPEATED_LOSS: f64 = 2.5;
const EXIT_RESPONSE_RETRANSMIT_FACTOR_REPEATED_STEP: f64 = 0.25;
const EXIT_RESPONSE_RETRANSMIT_FACTOR_REPEATED_MAX: f64 = 3.5;
const DEFAULT_EXIT_RESPONSE_PACING_BOOTSTRAP_RTT_MS: u64 = 200;
const DEFAULT_EXIT_RESPONSE_PACING_MIN_INTERVAL_MS: u64 = 1;
const EXIT_RESPONSE_PACING_BURST_CAP_FRAMES: f64 = 4.0;
const DEFAULT_EXIT_RESPONSE_INFLIGHT_DISCIPLINE_MIN_CAP_FRAMES: usize = 40;
const EXIT_RESPONSE_INFLIGHT_DISCIPLINE_REDUCE_STEP_FRAMES: usize = 4;
const EXIT_RESPONSE_INFLIGHT_DISCIPLINE_RESTORE_STEP_FRAMES: usize = 4;
const EXIT_RESPONSE_INFLIGHT_DISCIPLINE_STALL_PRESSURE_MS: u64 = 24;
const EXIT_RESPONSE_INFLIGHT_DISCIPLINE_STALL_SEVERE_MS: u64 = 96;
const EXIT_RESPONSE_INFLIGHT_DISCIPLINE_RESTORE_STREAK_SAMPLES: u32 = 3;
const EXIT_RESPONSE_INFLIGHT_DISCIPLINE_RESTORE_HEADROOM_FRAMES: usize = 4;
const EXIT_RESPONSE_INFLIGHT_DISCIPLINE_MODERATE_PRESSURE_STREAK_SAMPLES: u32 = 2;
const EXIT_RESPONSE_INFLIGHT_DISCIPLINE_SEVERE_PRESSURE_STREAK_SAMPLES: u32 = 2;

struct ExitStream {
    socket: TcpStream,
}

fn clamp_response_window_frames(frames: usize) -> usize {
    frames.clamp(
        MIN_EXIT_RESPONSE_WINDOW_FRAMES,
        MAX_EXIT_RESPONSE_WINDOW_FRAMES,
    )
}

#[derive(Debug, Clone, Copy)]
struct ResponseRetransmitDecision {
    timeout_ms: u64,
    rtt_ratio: f64,
    early: bool,
    late: bool,
}

fn response_retransmit_base_rtt_ms(summary: AckLatencySummary, bootstrap_rtt_ms: u64) -> u64 {
    summary
        .p50_ms
        .or(summary.avg_ms)
        .or(summary.p95_ms)
        .filter(|value| *value > 0)
        .unwrap_or(bootstrap_rtt_ms.max(1))
}

fn response_retransmit_factor(retransmit_count: u64) -> f64 {
    match retransmit_count {
        0 => EXIT_RESPONSE_RETRANSMIT_FACTOR_NORMAL,
        1 => EXIT_RESPONSE_RETRANSMIT_FACTOR_SUSPECTED_LOSS,
        _ => (EXIT_RESPONSE_RETRANSMIT_FACTOR_REPEATED_LOSS
            + (retransmit_count.saturating_sub(2) as f64
                * EXIT_RESPONSE_RETRANSMIT_FACTOR_REPEATED_STEP))
            .min(EXIT_RESPONSE_RETRANSMIT_FACTOR_REPEATED_MAX),
    }
}

fn response_retransmit_backoff_decision(
    summary: AckLatencySummary,
    retransmit_count: u64,
    bootstrap_rtt_ms: u64,
) -> ResponseRetransmitDecision {
    let rtt_ms = response_retransmit_base_rtt_ms(summary, bootstrap_rtt_ms);
    let factor = response_retransmit_factor(retransmit_count);
    let timeout_ms = ((rtt_ms as f64) * factor)
        .round()
        .clamp(
            EXIT_RESPONSE_RETRANSMIT_TIMEOUT_MIN_MS as f64,
            EXIT_RESPONSE_RETRANSMIT_TIMEOUT_MAX_MS as f64,
        ) as u64;
    let rtt_ratio = timeout_ms as f64 / rtt_ms.max(1) as f64;

    let (expected_factor, factor_margin) = match retransmit_count {
        0 => (EXIT_RESPONSE_RETRANSMIT_FACTOR_NORMAL, 0.25),
        1 => (EXIT_RESPONSE_RETRANSMIT_FACTOR_SUSPECTED_LOSS, 0.25),
        _ => (response_retransmit_factor(retransmit_count), 0.35),
    };
    ResponseRetransmitDecision {
        timeout_ms,
        rtt_ratio,
        early: rtt_ratio < (expected_factor - factor_margin),
        late: rtt_ratio > (expected_factor + factor_margin),
    }
}

#[cfg(test)]
fn adaptive_response_retransmit_interval(
    summary: AckLatencySummary,
    retransmit_count: u64,
    bootstrap_rtt_ms: u64,
) -> Duration {
    Duration::from_millis(
        response_retransmit_backoff_decision(summary, retransmit_count, bootstrap_rtt_ms)
            .timeout_ms,
    )
}

#[derive(Debug, Clone, Copy)]
struct ResponsePacingConfig {
    enabled: bool,
    window_frames: usize,
    bootstrap_rtt_ms: u64,
    min_interval_ms: u64,
}

#[derive(Debug, Clone, Copy)]
struct ResponseInflightDisciplineConfig {
    enabled: bool,
    hard_window_frames: usize,
    bootstrap_rtt_ms: u64,
    min_cap_frames: usize,
}

#[derive(Debug, Default, Clone)]
struct ResponsePacingState {
    pacing_budget_frames: f64,
    last_budget_refresh: Option<Instant>,
    pacing_delay_applied: u64,
    pacing_delay_applied_ms_total: u64,
    burst_prevented_count: u64,
    paced_send_batches: u64,
    max_send_burst_frames: u64,
    current_send_burst_frames: u64,
    pacing_interval_samples: u64,
    pacing_interval_ms_total: u64,
    pacing_interval_ms_max: u64,
}

#[derive(Debug, Clone)]
struct ResponseInflightDisciplineState {
    effective_cap_frames: usize,
    effective_cap_sum: u64,
    effective_cap_samples: u64,
    effective_cap_min: usize,
    effective_cap_max: usize,
    inflight_cap_reduced_count: u64,
    inflight_cap_restore_count: u64,
    ack_pressure_events: u64,
    send_blocked_by_effective_cap: u64,
    pressure_streak: u32,
    max_pressure_streak: u32,
    restore_clean_streak: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResponseInflightPressureLevel {
    None,
    Moderate,
    Severe,
}

impl ResponseInflightPressureLevel {
    fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Moderate => "moderate",
            Self::Severe => "severe",
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ResponseInflightPressureSignal {
    level: ResponseInflightPressureLevel,
    target_cap_frames: usize,
    reason: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResponseInflightCapAction {
    None,
    Reduced,
    Restored,
}

#[derive(Debug, Clone, Copy)]
struct ResponseInflightDecision {
    previous_cap_frames: usize,
    effective_cap_frames: usize,
    pressure_level: ResponseInflightPressureLevel,
    pressure_reason: &'static str,
    pressure_streak: u32,
    restore_clean_streak: u32,
    action: ResponseInflightCapAction,
}

fn response_pacing_interval_ms(summary: AckLatencySummary, config: ResponsePacingConfig) -> u64 {
    let bootstrap_rtt_ms = config
        .bootstrap_rtt_ms
        .max(DEFAULT_EXIT_RESPONSE_PACING_BOOTSTRAP_RTT_MS)
        .max(1);
    let min_interval_ms = config
        .min_interval_ms
        .max(DEFAULT_EXIT_RESPONSE_PACING_MIN_INTERVAL_MS)
        .max(1);
    let observed_rtt_ms = summary
        .avg_ms
        .or(summary.p50_ms)
        .or(summary.p95_ms)
        .filter(|value| *value > 0)
        .unwrap_or(bootstrap_rtt_ms);
    let divisor = config.window_frames.max(1) as u64;
    ((observed_rtt_ms.saturating_add(divisor.saturating_sub(1))) / divisor).max(min_interval_ms)
}

fn response_inflight_pressure_signal(
    summary: AckLatencySummary,
    config: ResponseInflightDisciplineConfig,
    recent_window_wait_ms: u64,
    current_inflight: usize,
) -> ResponseInflightPressureSignal {
    let hard_window_frames = config.hard_window_frames.max(1);
    let min_cap_frames = config.min_cap_frames.clamp(1, hard_window_frames);
    let bootstrap_rtt_ms = config
        .bootstrap_rtt_ms
        .max(DEFAULT_EXIT_RESPONSE_PACING_BOOTSTRAP_RTT_MS)
        .max(1);
    let ack_p95_ms = summary
        .p95_ms
        .or(summary.avg_ms)
        .or(summary.p50_ms)
        .filter(|value| *value > 0)
        .unwrap_or(bootstrap_rtt_ms);
    let ack_avg_ms = summary
        .avg_ms
        .or(summary.p50_ms)
        .filter(|value| *value > 0)
        .unwrap_or(ack_p95_ms);
    let occupancy_near_window = current_inflight >= hard_window_frames.saturating_sub(4);
    let slow_ack_floor_ms = bootstrap_rtt_ms.saturating_mul(6) / 5;
    let moderate_ack_tail_ms = bootstrap_rtt_ms.saturating_mul(5) / 4;
    let severe_ack_tail_ms = bootstrap_rtt_ms.saturating_mul(3) / 2;

    let slow_ack = ack_avg_ms >= slow_ack_floor_ms && ack_p95_ms >= moderate_ack_tail_ms;
    let severe_slow_ack = ack_avg_ms >= slow_ack_floor_ms && ack_p95_ms >= severe_ack_tail_ms;

    if occupancy_near_window
        && recent_window_wait_ms >= EXIT_RESPONSE_INFLIGHT_DISCIPLINE_STALL_SEVERE_MS
        && severe_slow_ack
    {
        return ResponseInflightPressureSignal {
            level: ResponseInflightPressureLevel::Severe,
            target_cap_frames: hard_window_frames
                .saturating_sub(EXIT_RESPONSE_INFLIGHT_DISCIPLINE_REDUCE_STEP_FRAMES * 2)
                .clamp(min_cap_frames, hard_window_frames),
            reason: "severe_stall_with_slow_ack",
        };
    }

    if occupancy_near_window
        && recent_window_wait_ms >= EXIT_RESPONSE_INFLIGHT_DISCIPLINE_STALL_PRESSURE_MS
        && slow_ack
    {
        return ResponseInflightPressureSignal {
            level: ResponseInflightPressureLevel::Moderate,
            target_cap_frames: hard_window_frames
                .saturating_sub(EXIT_RESPONSE_INFLIGHT_DISCIPLINE_REDUCE_STEP_FRAMES)
                .clamp(min_cap_frames, hard_window_frames),
            reason: "stall_with_slow_ack",
        };
    }

    ResponseInflightPressureSignal {
        level: ResponseInflightPressureLevel::None,
        target_cap_frames: hard_window_frames,
        reason: "no_pressure",
    }
}

fn response_inflight_restore_is_clean(
    summary: AckLatencySummary,
    config: ResponseInflightDisciplineConfig,
    recent_window_wait_ms: u64,
    current_inflight: usize,
    current_cap: usize,
) -> bool {
    let bootstrap_rtt_ms = config
        .bootstrap_rtt_ms
        .max(DEFAULT_EXIT_RESPONSE_PACING_BOOTSTRAP_RTT_MS)
        .max(1);
    let ack_p95_ms = summary
        .p95_ms
        .or(summary.avg_ms)
        .or(summary.p50_ms)
        .filter(|value| *value > 0)
        .unwrap_or(bootstrap_rtt_ms);
    let ack_avg_ms = summary
        .avg_ms
        .or(summary.p50_ms)
        .filter(|value| *value > 0)
        .unwrap_or(ack_p95_ms);
    recent_window_wait_ms == 0
        && ack_p95_ms <= bootstrap_rtt_ms.saturating_mul(3) / 2
        && ack_avg_ms <= bootstrap_rtt_ms.saturating_mul(5) / 4
        && current_inflight
            .saturating_add(EXIT_RESPONSE_INFLIGHT_DISCIPLINE_RESTORE_HEADROOM_FRAMES)
            < current_cap
}

impl ResponsePacingState {
    fn refresh_budget(&mut self, interval_ms: u64) {
        let now = Instant::now();
        match self.last_budget_refresh {
            None => {
                self.last_budget_refresh = Some(now);
                self.pacing_budget_frames = EXIT_RESPONSE_PACING_BURST_CAP_FRAMES;
            }
            Some(previous) => {
                if interval_ms == 0 {
                    self.last_budget_refresh = Some(now);
                    self.pacing_budget_frames = EXIT_RESPONSE_PACING_BURST_CAP_FRAMES;
                    return;
                }
                let elapsed = now.duration_since(previous).as_secs_f64();
                let interval_secs = interval_ms as f64 / 1000.0;
                let earned = if interval_secs > 0.0 {
                    elapsed / interval_secs
                } else {
                    EXIT_RESPONSE_PACING_BURST_CAP_FRAMES
                };
                self.pacing_budget_frames = (self.pacing_budget_frames + earned)
                    .clamp(0.0, EXIT_RESPONSE_PACING_BURST_CAP_FRAMES);
                self.last_budget_refresh = Some(now);
            }
        }
    }

    async fn before_send(
        &mut self,
        config: ResponsePacingConfig,
        summary: AckLatencySummary,
    ) -> u64 {
        if !config.enabled {
            return 0;
        }

        let interval_ms = response_pacing_interval_ms(summary, config);
        self.pacing_interval_samples = self.pacing_interval_samples.saturating_add(1);
        self.pacing_interval_ms_total = self.pacing_interval_ms_total.saturating_add(interval_ms);
        self.pacing_interval_ms_max = self.pacing_interval_ms_max.max(interval_ms);

        self.refresh_budget(interval_ms);
        if self.pacing_budget_frames + f64::EPSILON < 1.0 {
            let wait_fraction = (1.0 - self.pacing_budget_frames).clamp(0.0, 1.0);
            let wait_started = Instant::now();
            tokio::time::sleep(Duration::from_secs_f64(
                (interval_ms as f64 / 1000.0) * wait_fraction,
            ))
            .await;
            let waited_ms = wait_started.elapsed().as_millis() as u64;
            self.pacing_delay_applied = self.pacing_delay_applied.saturating_add(1);
            self.pacing_delay_applied_ms_total = self
                .pacing_delay_applied_ms_total
                .saturating_add(waited_ms.max(interval_ms / 2).max(1));
            if self.current_send_burst_frames > 0 {
                self.burst_prevented_count = self.burst_prevented_count.saturating_add(1);
            }
            self.current_send_burst_frames = 0;
            self.refresh_budget(interval_ms);
        }

        interval_ms
    }

    fn on_send(&mut self, config: ResponsePacingConfig, interval_ms: u64) {
        self.current_send_burst_frames = self.current_send_burst_frames.saturating_add(1);
        self.max_send_burst_frames = self
            .max_send_burst_frames
            .max(self.current_send_burst_frames);
        if config.enabled {
            if self.current_send_burst_frames == 1 {
                self.paced_send_batches = self.paced_send_batches.saturating_add(1);
            }
            self.refresh_budget(interval_ms);
            self.pacing_budget_frames = (self.pacing_budget_frames - 1.0).max(0.0);
        }
    }

    fn note_external_pause(&mut self) {
        self.current_send_burst_frames = 0;
    }

    fn avg_interval_ms(&self) -> Option<u64> {
        if self.pacing_interval_samples == 0 {
            return None;
        }
        Some(self.pacing_interval_ms_total / self.pacing_interval_samples)
    }
}

impl ResponseInflightDisciplineState {
    fn current_cap(&mut self, config: ResponseInflightDisciplineConfig) -> usize {
        let hard_window_frames = config.hard_window_frames.max(1);
        if self.effective_cap_frames == 0 {
            self.effective_cap_frames = hard_window_frames;
            self.effective_cap_min = hard_window_frames;
            self.effective_cap_max = hard_window_frames;
        }
        self.effective_cap_frames
    }

    fn sample_cap(&mut self, cap_frames: usize) {
        self.effective_cap_sum = self.effective_cap_sum.saturating_add(cap_frames as u64);
        self.effective_cap_samples = self.effective_cap_samples.saturating_add(1);
        if self.effective_cap_min == 0 {
            self.effective_cap_min = cap_frames;
        } else {
            self.effective_cap_min = self.effective_cap_min.min(cap_frames);
        }
        self.effective_cap_max = self.effective_cap_max.max(cap_frames);
    }

    fn update(
        &mut self,
        config: ResponseInflightDisciplineConfig,
        summary: AckLatencySummary,
        recent_window_wait_ms: u64,
        current_inflight: usize,
    ) -> ResponseInflightDecision {
        let hard_window_frames = config.hard_window_frames.max(1);
        if !config.enabled {
            self.effective_cap_frames = hard_window_frames;
            self.sample_cap(hard_window_frames);
            return ResponseInflightDecision {
                previous_cap_frames: hard_window_frames,
                effective_cap_frames: hard_window_frames,
                pressure_level: ResponseInflightPressureLevel::None,
                pressure_reason: "discipline_disabled",
                pressure_streak: 0,
                restore_clean_streak: 0,
                action: ResponseInflightCapAction::None,
            };
        }

        let current = self.current_cap(config);
        let signal = response_inflight_pressure_signal(
            summary,
            config,
            recent_window_wait_ms,
            current_inflight,
        );
        let mut action = ResponseInflightCapAction::None;
        match signal.level {
            ResponseInflightPressureLevel::None => {
                self.pressure_streak = self.pressure_streak.saturating_sub(1);
            }
            ResponseInflightPressureLevel::Moderate | ResponseInflightPressureLevel::Severe => {
                self.pressure_streak = self.pressure_streak.saturating_add(1);
                self.max_pressure_streak = self.max_pressure_streak.max(self.pressure_streak);
            }
        }

        let required_pressure_streak = match signal.level {
            ResponseInflightPressureLevel::None => u32::MAX,
            ResponseInflightPressureLevel::Moderate => {
                EXIT_RESPONSE_INFLIGHT_DISCIPLINE_MODERATE_PRESSURE_STREAK_SAMPLES
            }
            ResponseInflightPressureLevel::Severe => {
                EXIT_RESPONSE_INFLIGHT_DISCIPLINE_SEVERE_PRESSURE_STREAK_SAMPLES
            }
        };

        if signal.level != ResponseInflightPressureLevel::None
            && signal.target_cap_frames < current
            && self.pressure_streak >= required_pressure_streak
        {
            self.restore_clean_streak = 0;
            self.ack_pressure_events = self.ack_pressure_events.saturating_add(1);
            self.effective_cap_frames = signal.target_cap_frames;
            self.inflight_cap_reduced_count = self.inflight_cap_reduced_count.saturating_add(1);
            action = ResponseInflightCapAction::Reduced;
        } else if current < hard_window_frames {
            if signal.level == ResponseInflightPressureLevel::None
                && response_inflight_restore_is_clean(
                    summary,
                    config,
                    recent_window_wait_ms,
                    current_inflight,
                    current,
                )
            {
                self.restore_clean_streak = self.restore_clean_streak.saturating_add(1);
                if self.restore_clean_streak
                    >= EXIT_RESPONSE_INFLIGHT_DISCIPLINE_RESTORE_STREAK_SAMPLES
                {
                    let next = current
                        .saturating_add(EXIT_RESPONSE_INFLIGHT_DISCIPLINE_RESTORE_STEP_FRAMES);
                    let adjusted = next.min(hard_window_frames);
                    if adjusted > current {
                        self.effective_cap_frames = adjusted;
                        self.inflight_cap_restore_count =
                            self.inflight_cap_restore_count.saturating_add(1);
                        action = ResponseInflightCapAction::Restored;
                    }
                    self.restore_clean_streak = 0;
                }
            } else {
                self.restore_clean_streak = 0;
            }
        }
        self.sample_cap(self.effective_cap_frames);
        ResponseInflightDecision {
            previous_cap_frames: current,
            effective_cap_frames: self.effective_cap_frames,
            pressure_level: signal.level,
            pressure_reason: signal.reason,
            pressure_streak: self.pressure_streak,
            restore_clean_streak: self.restore_clean_streak,
            action,
        }
    }

    fn avg_cap_frames(&self) -> Option<u64> {
        if self.effective_cap_samples == 0 {
            return None;
        }
        Some(self.effective_cap_sum / self.effective_cap_samples)
    }
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
    let response_pacing_config = ResponsePacingConfig {
        enabled: args.response_pacing_enabled,
        window_frames: response_window_frames,
        bootstrap_rtt_ms: args
            .response_pacing_bootstrap_rtt_ms
            .max(DEFAULT_EXIT_RESPONSE_PACING_BOOTSTRAP_RTT_MS),
        min_interval_ms: args
            .response_pacing_min_interval_ms
            .max(DEFAULT_EXIT_RESPONSE_PACING_MIN_INTERVAL_MS),
    };
    let response_inflight_discipline_config = ResponseInflightDisciplineConfig {
        enabled: args.response_inflight_discipline_enabled,
        hard_window_frames: response_window_frames,
        bootstrap_rtt_ms: response_pacing_config.bootstrap_rtt_ms,
        min_cap_frames: DEFAULT_EXIT_RESPONSE_INFLIGHT_DISCIPLINE_MIN_CAP_FRAMES,
    };
    info!(
        listen_addr = %args.listen,
        target_addr = %args.target_addr,
        response_window_frames,
        response_pacing_enabled = response_pacing_config.enabled,
        response_pacing_bootstrap_rtt_ms = response_pacing_config.bootstrap_rtt_ms,
        response_pacing_min_interval_ms = response_pacing_config.min_interval_ms,
        response_inflight_discipline_enabled = response_inflight_discipline_config.enabled,
        response_inflight_discipline_min_cap_frames = response_inflight_discipline_config.min_cap_frames,
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
    let response_pacing_config_for_task = response_pacing_config;
    let response_inflight_discipline_config_for_task = response_inflight_discipline_config;

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
            let mut effective_cap_wait_events: u64 = 0;
            let mut effective_cap_wait_total_ms: u64 = 0;
            let mut effective_cap_wait_max_ms: u64 = 0;
            let mut response_pacing = ResponsePacingState::default();
            let mut inflight_discipline = ResponseInflightDisciplineState {
                effective_cap_frames: window_frames,
                effective_cap_sum: 0,
                effective_cap_samples: 0,
                effective_cap_min: window_frames,
                effective_cap_max: window_frames,
                inflight_cap_reduced_count: 0,
                inflight_cap_restore_count: 0,
                ack_pressure_events: 0,
                send_blocked_by_effective_cap: 0,
                pressure_streak: 0,
                max_pressure_streak: 0,
                restore_clean_streak: 0,
            };
            let mut recent_window_wait_ms: u64 = 0;

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

                                let (current_inflight_for_cap, ack_summary) = {
                                    let map = reliable_streams_for_task.lock().unwrap();
                                    if let Some(rs) = map.get(&stream_id) {
                                        (rs.inflight(), rs.ack_latency_summary())
                                    } else {
                                        (0usize, AckLatencySummary::default())
                                    }
                                };
                                let decision = inflight_discipline.update(
                                    response_inflight_discipline_config_for_task,
                                    ack_summary,
                                    recent_window_wait_ms,
                                    current_inflight_for_cap,
                                );
                                let effective_cap = decision.effective_cap_frames;
                                if decision.action == ResponseInflightCapAction::Reduced {
                                    emit_exit_stage(
                                        "effective_inflight_cap_reduced",
                                        json!({
                                            "stream_id": stream_id,
                                            "site": site.as_str(),
                                            "route_len": routing.route.len(),
                                            "peer": peer.to_string(),
                                            "target_addr": target_addr.to_string(),
                                            "previous_cap": decision.previous_cap_frames,
                                            "effective_cap": effective_cap,
                                            "recent_window_wait_ms": recent_window_wait_ms,
                                            "ack_latency_ms_avg": ack_summary.avg_ms,
                                            "ack_latency_ms_p95": ack_summary.p95_ms,
                                            "pressure_level": decision.pressure_level.as_str(),
                                            "pressure_reason": decision.pressure_reason,
                                            "pressure_streak": decision.pressure_streak,
                                        }),
                                    );
                                } else if decision.action == ResponseInflightCapAction::Restored {
                                    emit_exit_stage(
                                        "effective_inflight_cap_restored",
                                        json!({
                                            "stream_id": stream_id,
                                            "site": site.as_str(),
                                            "route_len": routing.route.len(),
                                            "peer": peer.to_string(),
                                            "target_addr": target_addr.to_string(),
                                            "previous_cap": decision.previous_cap_frames,
                                            "effective_cap": effective_cap,
                                            "recent_window_wait_ms": recent_window_wait_ms,
                                            "ack_latency_ms_avg": ack_summary.avg_ms,
                                            "ack_latency_ms_p95": ack_summary.p95_ms,
                                            "pressure_level": decision.pressure_level.as_str(),
                                            "pressure_reason": decision.pressure_reason,
                                            "pressure_streak": decision.pressure_streak,
                                            "restore_clean_streak": decision.restore_clean_streak,
                                        }),
                                    );
                                }

                                // Sliding window cap: don't let in-flight response frames
                                // grow beyond the hard window or the current effective cap.
                                let window_wait_started = Instant::now();
                                let mut waited_for_window = false;
                                let mut blocked_by_effective_cap = false;
                                let mut blocked_by_hard_window = false;
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
                                    if inflight < effective_cap {
                                        break;
                                    }
                                    waited_for_window = true;
                                    if inflight >= window_frames {
                                        blocked_by_hard_window = true;
                                    } else if effective_cap < window_frames {
                                        blocked_by_effective_cap = true;
                                    }
                                    tokio::time::sleep(Duration::from_millis(2)).await;
                                }
                                if waited_for_window {
                                    let waited_ms =
                                        window_wait_started.elapsed().as_millis() as u64;
                                    if blocked_by_hard_window {
                                        window_wait_events = window_wait_events.saturating_add(1);
                                        window_wait_total_ms =
                                            window_wait_total_ms.saturating_add(waited_ms);
                                        window_wait_max_ms = window_wait_max_ms.max(waited_ms);
                                        recent_window_wait_ms = waited_ms;
                                    } else if blocked_by_effective_cap {
                                        effective_cap_wait_events =
                                            effective_cap_wait_events.saturating_add(1);
                                        effective_cap_wait_total_ms =
                                            effective_cap_wait_total_ms.saturating_add(waited_ms);
                                        effective_cap_wait_max_ms =
                                            effective_cap_wait_max_ms.max(waited_ms);
                                        recent_window_wait_ms = 0;
                                    }
                                    response_pacing.note_external_pause();
                                    if blocked_by_effective_cap {
                                        inflight_discipline.send_blocked_by_effective_cap =
                                            inflight_discipline
                                                .send_blocked_by_effective_cap
                                                .saturating_add(1);
                                        emit_exit_stage(
                                            "send_blocked_by_effective_cap",
                                            json!({
                                                "stream_id": stream_id,
                                                "site": site.as_str(),
                                                "route_len": routing.route.len(),
                                                "peer": peer.to_string(),
                                                "target_addr": target_addr.to_string(),
                                                "effective_cap": effective_cap,
                                                "window_frames": window_frames,
                                                "waited_ms": waited_ms,
                                            }),
                                        );
                                    }
                                } else {
                                    recent_window_wait_ms = 0;
                                }

                                let pacing_interval_ms = response_pacing
                                    .before_send(response_pacing_config_for_task, ack_summary)
                                    .await;

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
                                        response_pacing.on_send(
                                            response_pacing_config_for_task,
                                            pacing_interval_ms,
                                        );
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

                let (current_inflight_for_cap, ack_summary) = {
                    let map = reliable_streams_for_task.lock().unwrap();
                    if let Some(rs) = map.get(&stream_id) {
                        (rs.inflight(), rs.ack_latency_summary())
                    } else {
                        (0usize, AckLatencySummary::default())
                    }
                };
                let previous_effective_cap =
                    inflight_discipline.current_cap(response_inflight_discipline_config_for_task);
                let decision = inflight_discipline.update(
                    response_inflight_discipline_config_for_task,
                    ack_summary,
                    recent_window_wait_ms,
                    current_inflight_for_cap,
                );
                let effective_cap = decision.effective_cap_frames;
                if matches!(decision.action, ResponseInflightCapAction::Reduced) {
                    emit_exit_stage(
                        "effective_inflight_cap_reduced",
                        json!({
                            "stream_id": stream_id,
                            "site": site.as_str(),
                            "route_len": routing.route.len(),
                            "peer": peer.to_string(),
                            "target_addr": target_addr.to_string(),
                            "previous_cap": previous_effective_cap,
                            "effective_cap": effective_cap,
                            "pressure_level": decision.pressure_level.as_str(),
                            "pressure_reason": decision.pressure_reason,
                            "pressure_streak": decision.pressure_streak,
                            "recent_window_wait_ms": recent_window_wait_ms,
                            "ack_latency_ms_avg": ack_summary.avg_ms,
                            "ack_latency_ms_p95": ack_summary.p95_ms,
                        }),
                    );
                } else if matches!(decision.action, ResponseInflightCapAction::Restored) {
                    emit_exit_stage(
                        "effective_inflight_cap_restored",
                        json!({
                            "stream_id": stream_id,
                            "site": site.as_str(),
                            "route_len": routing.route.len(),
                            "peer": peer.to_string(),
                            "target_addr": target_addr.to_string(),
                            "previous_cap": previous_effective_cap,
                            "effective_cap": effective_cap,
                            "pressure_level": decision.pressure_level.as_str(),
                            "pressure_reason": decision.pressure_reason,
                            "restore_clean_streak": decision.restore_clean_streak,
                            "recent_window_wait_ms": recent_window_wait_ms,
                            "ack_latency_ms_avg": ack_summary.avg_ms,
                            "ack_latency_ms_p95": ack_summary.p95_ms,
                        }),
                    );
                }

                let window_wait_started = Instant::now();
                let mut waited_for_window = false;
                let mut blocked_by_effective_cap = false;
                let mut blocked_by_hard_window = false;
                loop {
                    let inflight = {
                        let map = reliable_streams_for_task.lock().unwrap();
                        map.get(&stream_id).map(|rs| rs.inflight()).unwrap_or(0)
                    };
                    if inflight > max_inflight {
                        max_inflight = inflight;
                    }
                    if inflight < effective_cap {
                        break;
                    }
                    waited_for_window = true;
                    if inflight >= window_frames {
                        blocked_by_hard_window = true;
                    } else if effective_cap < window_frames {
                        blocked_by_effective_cap = true;
                    }
                    tokio::time::sleep(Duration::from_millis(2)).await;
                }
                if waited_for_window {
                    let waited_ms = window_wait_started.elapsed().as_millis() as u64;
                    if blocked_by_hard_window {
                        window_wait_events = window_wait_events.saturating_add(1);
                        window_wait_total_ms = window_wait_total_ms.saturating_add(waited_ms);
                        window_wait_max_ms = window_wait_max_ms.max(waited_ms);
                        recent_window_wait_ms = waited_ms;
                    } else if blocked_by_effective_cap {
                        effective_cap_wait_events = effective_cap_wait_events.saturating_add(1);
                        effective_cap_wait_total_ms =
                            effective_cap_wait_total_ms.saturating_add(waited_ms);
                        effective_cap_wait_max_ms = effective_cap_wait_max_ms.max(waited_ms);
                        recent_window_wait_ms = 0;
                    }
                    response_pacing.note_external_pause();
                    if blocked_by_effective_cap {
                        inflight_discipline.send_blocked_by_effective_cap = inflight_discipline
                            .send_blocked_by_effective_cap
                            .saturating_add(1);
                        emit_exit_stage(
                            "send_blocked_by_effective_cap",
                            json!({
                                "stream_id": stream_id,
                                "site": site.as_str(),
                                "route_len": routing.route.len(),
                                "peer": peer.to_string(),
                                "target_addr": target_addr.to_string(),
                                "effective_cap": effective_cap,
                                "window_frames": window_frames,
                                "waited_ms": waited_ms,
                            }),
                        );
                    }
                } else {
                    recent_window_wait_ms = 0;
                }

                let pacing_interval_ms = response_pacing
                    .before_send(response_pacing_config_for_task, ack_summary)
                    .await;

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
                        response_pacing
                            .on_send(response_pacing_config_for_task, pacing_interval_ms);
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
            let pacing_interval_ms_avg = response_pacing.avg_interval_ms();
            let pacing_interval_ms_max = if response_pacing.pacing_interval_ms_max > 0 {
                Some(response_pacing.pacing_interval_ms_max)
            } else {
                None
            };
            let effective_inflight_cap_avg = inflight_discipline.avg_cap_frames();
            let effective_inflight_cap_min = if inflight_discipline.effective_cap_min > 0 {
                Some(inflight_discipline.effective_cap_min as u64)
            } else {
                None
            };
            let effective_inflight_cap_max = if inflight_discipline.effective_cap_max > 0 {
                Some(inflight_discipline.effective_cap_max as u64)
            } else {
                None
            };

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
                retransmit_backoff,
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
                        rs.retransmit_backoff_summary(),
                    )
                } else {
                    (
                        0u64,
                        0u64,
                        0u64,
                        None,
                        None,
                        None,
                        None,
                        None,
                        RetransmitBackoffSummary {
                            timeout_ms_avg: None,
                            timeout_ms_min: None,
                            timeout_ms_p50: None,
                            timeout_ms_p95: None,
                            timeout_ms_max: None,
                            trigger_count: 0,
                            early_count: 0,
                            late_count: 0,
                            rtt_ratio_avg: None,
                        },
                    )
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
                effective_cap_wait_events,
                effective_cap_wait_total_ms,
                effective_cap_wait_max_ms,
                pacing_delay_applied = response_pacing.pacing_delay_applied,
                pacing_delay_applied_ms_total = response_pacing.pacing_delay_applied_ms_total,
                pacing_interval_ms_avg = ?pacing_interval_ms_avg,
                pacing_interval_ms_max = ?pacing_interval_ms_max,
                burst_prevented_count = response_pacing.burst_prevented_count,
                paced_send_batches = response_pacing.paced_send_batches,
                max_send_burst_frames = response_pacing.max_send_burst_frames,
                effective_inflight_cap_avg = ?effective_inflight_cap_avg,
                effective_inflight_cap_min = ?effective_inflight_cap_min,
                effective_inflight_cap_max = ?effective_inflight_cap_max,
                inflight_cap_reduced_count = inflight_discipline.inflight_cap_reduced_count,
                inflight_cap_restore_count = inflight_discipline.inflight_cap_restore_count,
                ack_pressure_events = inflight_discipline.ack_pressure_events,
                max_pressure_streak = inflight_discipline.max_pressure_streak,
                send_blocked_by_effective_cap = inflight_discipline.send_blocked_by_effective_cap,
                frames_sent_total,
                frames_acked_total,
                total_retransmits,
                retransmit_rate,
                retransmit_timeout_ms_avg = ?retransmit_backoff.timeout_ms_avg,
                retransmit_timeout_ms_p50 = ?retransmit_backoff.timeout_ms_p50,
                retransmit_timeout_ms_p95 = ?retransmit_backoff.timeout_ms_p95,
                retransmit_timeout_ms_max = ?retransmit_backoff.timeout_ms_max,
                retransmit_trigger_count = retransmit_backoff.trigger_count,
                retransmit_early_count = retransmit_backoff.early_count,
                retransmit_late_count = retransmit_backoff.late_count,
                retransmit_rtt_ratio = ?retransmit_backoff.rtt_ratio_avg,
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
            let mut stream_complete_payload = serde_json::Map::new();
            stream_complete_payload.insert("stream_id".to_string(), json!(stream_id));
            stream_complete_payload.insert("site".to_string(), json!(site.as_str()));
            stream_complete_payload.insert("route_len".to_string(), json!(route_hops));
            stream_complete_payload.insert("peer".to_string(), json!(peer.to_string()));
            stream_complete_payload
                .insert("target_addr".to_string(), json!(target_addr.to_string()));
            stream_complete_payload.insert(
                "first_target_byte_ms".to_string(),
                json!(first_target_byte_ms),
            );
            stream_complete_payload.insert(
                "first_overlay_send_ms".to_string(),
                json!(first_overlay_send_ms),
            );
            stream_complete_payload.insert(
                "stream_duration_ms".to_string(),
                json!(stream_duration_ms_total),
            );
            stream_complete_payload.insert(
                "resp_bytes".to_string(),
                json!(sent_payload_bytes_total_u64),
            );
            stream_complete_payload.insert(
                "frames_sent".to_string(),
                json!(sent_frames_payload_total_u64),
            );
            stream_complete_payload.insert("avg_inflight".to_string(), json!(avg_inflight));
            stream_complete_payload.insert("max_inflight".to_string(), json!(max_inflight));
            stream_complete_payload.insert("window_frames".to_string(), json!(window_frames));
            stream_complete_payload
                .insert("window_wait_events".to_string(), json!(window_wait_events));
            stream_complete_payload.insert(
                "window_wait_total_ms".to_string(),
                json!(window_wait_total_ms),
            );
            stream_complete_payload
                .insert("window_wait_max_ms".to_string(), json!(window_wait_max_ms));
            stream_complete_payload.insert(
                "effective_cap_wait_events".to_string(),
                json!(effective_cap_wait_events),
            );
            stream_complete_payload.insert(
                "effective_cap_wait_total_ms".to_string(),
                json!(effective_cap_wait_total_ms),
            );
            stream_complete_payload.insert(
                "effective_cap_wait_max_ms".to_string(),
                json!(effective_cap_wait_max_ms),
            );
            stream_complete_payload.insert(
                "pacing_enabled".to_string(),
                json!(response_pacing_config_for_task.enabled),
            );
            stream_complete_payload.insert(
                "pacing_delay_applied".to_string(),
                json!(response_pacing.pacing_delay_applied),
            );
            stream_complete_payload.insert(
                "pacing_delay_applied_ms_total".to_string(),
                json!(response_pacing.pacing_delay_applied_ms_total),
            );
            stream_complete_payload.insert(
                "pacing_interval_ms_avg".to_string(),
                json!(pacing_interval_ms_avg),
            );
            stream_complete_payload.insert(
                "pacing_interval_ms_max".to_string(),
                json!(pacing_interval_ms_max),
            );
            stream_complete_payload.insert(
                "burst_prevented_count".to_string(),
                json!(response_pacing.burst_prevented_count),
            );
            stream_complete_payload.insert(
                "paced_send_batches".to_string(),
                json!(response_pacing.paced_send_batches),
            );
            stream_complete_payload.insert(
                "max_send_burst_frames".to_string(),
                json!(response_pacing.max_send_burst_frames),
            );
            stream_complete_payload.insert(
                "effective_inflight_cap_avg".to_string(),
                json!(effective_inflight_cap_avg),
            );
            stream_complete_payload.insert(
                "effective_inflight_cap_min".to_string(),
                json!(effective_inflight_cap_min),
            );
            stream_complete_payload.insert(
                "effective_inflight_cap_max".to_string(),
                json!(effective_inflight_cap_max),
            );
            stream_complete_payload.insert(
                "inflight_cap_reduced_count".to_string(),
                json!(inflight_discipline.inflight_cap_reduced_count),
            );
            stream_complete_payload.insert(
                "inflight_cap_restore_count".to_string(),
                json!(inflight_discipline.inflight_cap_restore_count),
            );
            stream_complete_payload.insert(
                "ack_pressure_events".to_string(),
                json!(inflight_discipline.ack_pressure_events),
            );
            stream_complete_payload.insert(
                "max_pressure_streak".to_string(),
                json!(inflight_discipline.max_pressure_streak),
            );
            stream_complete_payload.insert(
                "send_blocked_by_effective_cap".to_string(),
                json!(inflight_discipline.send_blocked_by_effective_cap),
            );
            stream_complete_payload.insert(
                "time_at_inflight_1_ms".to_string(),
                json!(time_at_inflight_1_ms),
            );
            stream_complete_payload
                .insert("total_retransmits".to_string(), json!(total_retransmits));
            stream_complete_payload.insert("retransmit_rate".to_string(), json!(retransmit_rate));
            stream_complete_payload.insert(
                "retransmit_timeout_ms_avg".to_string(),
                json!(retransmit_backoff.timeout_ms_avg),
            );
            stream_complete_payload.insert(
                "retransmit_timeout_ms_min".to_string(),
                json!(retransmit_backoff.timeout_ms_min),
            );
            stream_complete_payload.insert(
                "retransmit_timeout_ms_p50".to_string(),
                json!(retransmit_backoff.timeout_ms_p50),
            );
            stream_complete_payload.insert(
                "retransmit_timeout_ms_p95".to_string(),
                json!(retransmit_backoff.timeout_ms_p95),
            );
            stream_complete_payload.insert(
                "retransmit_timeout_ms_max".to_string(),
                json!(retransmit_backoff.timeout_ms_max),
            );
            stream_complete_payload.insert(
                "retransmit_trigger_count".to_string(),
                json!(retransmit_backoff.trigger_count),
            );
            stream_complete_payload.insert(
                "retransmit_early_count".to_string(),
                json!(retransmit_backoff.early_count),
            );
            stream_complete_payload.insert(
                "retransmit_late_count".to_string(),
                json!(retransmit_backoff.late_count),
            );
            stream_complete_payload.insert(
                "retransmit_rtt_ratio".to_string(),
                json!(retransmit_backoff.rtt_ratio_avg),
            );
            stream_complete_payload.insert(
                "ack_latency_ms_avg".to_string(),
                json!(ack_latency_ms_avg_val),
            );
            stream_complete_payload
                .insert("ack_latency_ms_min".to_string(), json!(ack_latency_min_ms));
            stream_complete_payload
                .insert("ack_latency_ms_p50".to_string(), json!(ack_latency_p50_ms));
            stream_complete_payload
                .insert("ack_latency_ms_p95".to_string(), json!(ack_latency_p95_ms));
            stream_complete_payload
                .insert("ack_latency_ms_max".to_string(), json!(ack_latency_max_ms));
            stream_complete_payload.insert("http_code".to_string(), json!(http_code_val));
            stream_complete_payload.insert("connection_alive".to_string(), json!(connection_alive));
            emit_exit_stage(
                "stream_complete",
                serde_json::Value::Object(stream_complete_payload),
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
        let response_pacing_config_retx = response_pacing_config;
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
                        let ack_summary = rs.ack_latency_summary();
                        let ack_seq = rs.recv_next;
                        let due: Vec<(u64, ResponseRetransmitDecision)> = rs
                            .unacked
                            .iter()
                            .filter_map(|(&seq, entry)| {
                                let decision = response_retransmit_backoff_decision(
                                    ack_summary,
                                    entry.retransmit_count,
                                    response_pacing_config_retx.bootstrap_rtt_ms,
                                );
                                (now.duration_since(entry.last_sent)
                                    >= Duration::from_millis(decision.timeout_ms))
                                .then_some((seq, decision))
                            })
                            .take(16)
                            .collect();
                        for (seq, decision) in due {
                            let payload = match rs.unacked.get_mut(&seq) {
                                Some(entry) => {
                                    entry.last_sent = now;
                                    entry.retransmit_count =
                                        entry.retransmit_count.saturating_add(1);
                                    entry.payload.clone()
                                }
                                None => continue,
                            };
                            rs.total_retransmits = rs.total_retransmits.saturating_add(1);
                            rs.record_retransmit_timeout_sample(
                                decision.timeout_ms,
                                decision.rtt_ratio,
                                decision.early,
                                decision.late,
                            );
                            out.push(StreamFrame {
                                stream_id: sid,
                                frame_seq: seq,
                                ack_seq,
                                payload,
                            });
                        }
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
                let reject = classify_open_message_error(&e);
                if reject.kind == SessionOpenRejectKind::Duplicate {
                    emit_exit_stage(
                        "duplicate_packet_dropped",
                        json!({
                            "peer": peer.to_string(),
                            "route_len": session_route.as_ref().map(|route| route.len()),
                            "seq": reject.seq,
                            "highest": reject.highest,
                            "behind": reject.behind,
                            "window": reject.window,
                        }),
                    );
                    debug!(
                        peer = %peer,
                        seq = ?reject.seq,
                        highest = ?reject.highest,
                        behind = ?reject.behind,
                        window = ?reject.window,
                        "exit dropped duplicate packet before decode"
                    );
                    continue;
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
                let details = rs.apply_ack_with_latency_details(ack.ack_seq);
                match details.disposition {
                    AckDisposition::Advanced => {
                        emit_exit_stage(
                            "cumulative_ack_advanced",
                            json!({
                                "stream_id": ack.stream_id,
                                "ack_seq": details.ack_seq,
                                "previous_ack_seq": details.previous_ack_seq,
                                "acked_frames": details.acked_frames,
                                "ack_latency_ms_avg": details.avg_latency_ms,
                                "inflight_before": details.inflight_before,
                                "inflight_after": details.inflight_after,
                                "first_acked_seq": details.first_acked_seq,
                                "last_acked_seq_exclusive": details.last_acked_seq_exclusive,
                            }),
                        );
                        if details.acked_frames > 0 {
                            emit_exit_stage(
                                "inflight_cleanup_by_ack_range",
                                json!({
                                    "stream_id": ack.stream_id,
                                    "ack_seq": details.ack_seq,
                                    "acked_frames": details.acked_frames,
                                    "inflight_before": details.inflight_before,
                                    "inflight_after": details.inflight_after,
                                    "first_acked_seq": details.first_acked_seq,
                                    "last_acked_seq_exclusive": details.last_acked_seq_exclusive,
                                }),
                            );
                        }
                        if details.gap_detected {
                            emit_exit_stage(
                                "ack_gap_detected",
                                json!({
                                    "stream_id": ack.stream_id,
                                    "ack_seq": details.ack_seq,
                                    "previous_ack_seq": details.previous_ack_seq,
                                    "acked_frames": details.acked_frames,
                                    "first_acked_seq": details.first_acked_seq,
                                    "last_acked_seq_exclusive": details.last_acked_seq_exclusive,
                                }),
                            );
                        }
                    }
                    AckDisposition::Stale => {
                        emit_exit_stage(
                            "stale_ack_ignored",
                            json!({
                                "stream_id": ack.stream_id,
                                "ack_seq": details.ack_seq,
                                "previous_ack_seq": details.previous_ack_seq,
                                "inflight_before": details.inflight_before,
                                "inflight_after": details.inflight_after,
                            }),
                        );
                    }
                    AckDisposition::Regression => {
                        emit_exit_stage(
                            "ack_regression_ignored",
                            json!({
                                "stream_id": ack.stream_id,
                                "ack_seq": details.ack_seq,
                                "previous_ack_seq": details.previous_ack_seq,
                                "inflight_before": details.inflight_before,
                                "inflight_after": details.inflight_after,
                            }),
                        );
                    }
                }
                if details.acked_frames > 0 {
                    debug!(
                        stream_id = ack.stream_id,
                        ack_seq = details.ack_seq,
                        acked_frames = details.acked_frames,
                        ack_latency_ms_avg = ?details.avg_latency_ms,
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
    use crate::stream_reliable::{AckLatencySummary, ReliableStream};

    #[test]
    fn response_retransmit_backoff_matches_rtt() {
        let short = response_retransmit_backoff_decision(
            AckLatencySummary {
                avg_ms: Some(180),
                min_ms: Some(160),
                p50_ms: Some(175),
                p95_ms: Some(190),
                max_ms: Some(220),
            },
            0,
            200,
        );
        assert_eq!(short.timeout_ms, 263);
        assert!(!short.early);
        assert!(!short.late);

        let suspected_loss = response_retransmit_backoff_decision(
            AckLatencySummary {
                avg_ms: Some(220),
                min_ms: Some(200),
                p50_ms: Some(210),
                p95_ms: Some(260),
                max_ms: Some(280),
            },
            1,
            200,
        );
        assert_eq!(suspected_loss.timeout_ms, 420);
        assert!(!suspected_loss.early);
        assert!(!suspected_loss.late);

        let repeated_loss = response_retransmit_backoff_decision(
            AckLatencySummary {
                avg_ms: Some(420),
                min_ms: Some(390),
                p50_ms: Some(410),
                p95_ms: Some(520),
                max_ms: Some(560),
            },
            4,
            200,
        );
        assert_eq!(repeated_loss.timeout_ms, 1000);
        assert!(repeated_loss.rtt_ratio > 2.0);
    }

    #[test]
    fn adaptive_retransmit_interval_uses_bootstrap_without_ack_samples() {
        let interval =
            adaptive_response_retransmit_interval(AckLatencySummary::default(), 0, 200);
        assert_eq!(interval, Duration::from_millis(300));
    }

    #[test]
    fn response_pacing_interval_uses_bootstrap_when_ack_samples_missing() {
        let interval_ms = response_pacing_interval_ms(
            AckLatencySummary::default(),
            ResponsePacingConfig {
                enabled: true,
                window_frames: 64,
                bootstrap_rtt_ms: 200,
                min_interval_ms: 1,
            },
        );
        assert_eq!(interval_ms, 4);
    }

    #[test]
    fn response_pacing_interval_tracks_ack_rtt_and_respects_minimum() {
        let interval_ms = response_pacing_interval_ms(
            AckLatencySummary {
                avg_ms: Some(320),
                min_ms: Some(280),
                p50_ms: Some(300),
                p95_ms: Some(410),
                max_ms: Some(430),
            },
            ResponsePacingConfig {
                enabled: true,
                window_frames: 64,
                bootstrap_rtt_ms: 200,
                min_interval_ms: 1,
            },
        );
        assert_eq!(interval_ms, 5);

        let min_interval_ms = response_pacing_interval_ms(
            AckLatencySummary {
                avg_ms: Some(20),
                min_ms: Some(15),
                p50_ms: Some(18),
                p95_ms: Some(24),
                max_ms: Some(28),
            },
            ResponsePacingConfig {
                enabled: true,
                window_frames: 64,
                bootstrap_rtt_ms: 200,
                min_interval_ms: 2,
            },
        );
        assert_eq!(min_interval_ms, 2);
    }

    #[test]
    fn response_inflight_discipline_ignores_short_path_retransmit_age_noise() {
        let mut stream = ReliableStream::new();
        let base = Instant::now();

        for _ in 0..8 {
            let frame = stream.build_outgoing_frame(7, vec![1; 1000]);
            let entry = stream.unacked.get_mut(&frame.frame_seq).unwrap();
            entry.first_sent = base - Duration::from_millis(760);
            entry.last_sent = base - Duration::from_millis(185);
            entry.retransmit_count = 1;
        }

        let details = stream.apply_ack_with_latency_details(8);
        assert_eq!(details.acked_frames, 8);
        let summary = stream.ack_latency_summary();
        assert!(summary.avg_ms.unwrap() < 300);
        assert!(summary.p95_ms.unwrap() < 300);

        let pacing_interval_ms = response_pacing_interval_ms(
            summary,
            ResponsePacingConfig {
                enabled: true,
                window_frames: 64,
                bootstrap_rtt_ms: 200,
                min_interval_ms: 1,
            },
        );
        assert_eq!(pacing_interval_ms, 3);

        let retransmit_interval = adaptive_response_retransmit_interval(summary, 0, 200);
        assert_eq!(retransmit_interval, Duration::from_millis(278));

        let pressure = response_inflight_pressure_signal(
            summary,
            ResponseInflightDisciplineConfig {
                enabled: true,
                hard_window_frames: 64,
                bootstrap_rtt_ms: 200,
                min_cap_frames: 40,
            },
            0,
            48,
        );
        assert_eq!(pressure.level, ResponseInflightPressureLevel::None);
        assert_eq!(pressure.target_cap_frames, 64);
    }

    #[test]
    fn response_inflight_pressure_signal_detects_severe_stall_with_slow_ack() {
        let signal = response_inflight_pressure_signal(
            AckLatencySummary {
                avg_ms: Some(260),
                min_ms: Some(180),
                p50_ms: Some(250),
                p95_ms: Some(360),
                max_ms: Some(360),
            },
            ResponseInflightDisciplineConfig {
                enabled: true,
                hard_window_frames: 64,
                bootstrap_rtt_ms: 200,
                min_cap_frames: 40,
            },
            120,
            60,
        );
        assert_eq!(signal.level, ResponseInflightPressureLevel::Severe);
        assert_eq!(signal.reason, "severe_stall_with_slow_ack");
        assert_eq!(signal.target_cap_frames, 56);
    }

    #[test]
    fn response_inflight_pressure_signal_ignores_transient_fast_route_stall() {
        let signal = response_inflight_pressure_signal(
            AckLatencySummary {
                avg_ms: Some(189),
                min_ms: Some(140),
                p50_ms: Some(182),
                p95_ms: Some(202),
                max_ms: Some(605),
            },
            ResponseInflightDisciplineConfig {
                enabled: true,
                hard_window_frames: 64,
                bootstrap_rtt_ms: 200,
                min_cap_frames: 40,
            },
            410,
            64,
        );
        assert_eq!(signal.level, ResponseInflightPressureLevel::None);
        assert_eq!(signal.reason, "no_pressure");
        assert_eq!(signal.target_cap_frames, 64);
    }

    #[test]
    fn response_inflight_pressure_signal_detects_moderate_stall_with_slow_ack() {
        let signal = response_inflight_pressure_signal(
            AckLatencySummary {
                avg_ms: Some(243),
                min_ms: Some(230),
                p50_ms: Some(235),
                p95_ms: Some(251),
                max_ms: Some(366),
            },
            ResponseInflightDisciplineConfig {
                enabled: true,
                hard_window_frames: 64,
                bootstrap_rtt_ms: 200,
                min_cap_frames: 40,
            },
            24,
            61,
        );
        assert_eq!(signal.level, ResponseInflightPressureLevel::Moderate);
        assert_eq!(signal.reason, "stall_with_slow_ack");
        assert_eq!(signal.target_cap_frames, 60);
    }

    #[test]
    fn response_inflight_state_requires_sustained_pressure_before_reducing() {
        let config = ResponseInflightDisciplineConfig {
            enabled: true,
            hard_window_frames: 64,
            bootstrap_rtt_ms: 200,
            min_cap_frames: 40,
        };
        let mut state = ResponseInflightDisciplineState {
            effective_cap_frames: 64,
            effective_cap_sum: 0,
            effective_cap_samples: 0,
            effective_cap_min: 64,
            effective_cap_max: 64,
            inflight_cap_reduced_count: 0,
            inflight_cap_restore_count: 0,
            ack_pressure_events: 0,
            send_blocked_by_effective_cap: 0,
            pressure_streak: 0,
            max_pressure_streak: 0,
            restore_clean_streak: 0,
        };
        let first = state.update(
            config,
            AckLatencySummary {
                avg_ms: Some(243),
                min_ms: Some(230),
                p50_ms: Some(235),
                p95_ms: Some(251),
                max_ms: Some(366),
            },
            24,
            61,
        );
        assert_eq!(first.effective_cap_frames, 64);
        assert_eq!(first.action, ResponseInflightCapAction::None);
        assert_eq!(first.pressure_streak, 1);

        let second = state.update(
            config,
            AckLatencySummary {
                avg_ms: Some(245),
                min_ms: Some(231),
                p50_ms: Some(236),
                p95_ms: Some(255),
                max_ms: Some(370),
            },
            30,
            61,
        );
        assert_eq!(second.effective_cap_frames, 60);
        assert_eq!(second.action, ResponseInflightCapAction::Reduced);
        assert_eq!(state.inflight_cap_reduced_count, 1);
    }

    #[test]
    fn response_inflight_state_restores_after_shorter_clean_streak() {
        let config = ResponseInflightDisciplineConfig {
            enabled: true,
            hard_window_frames: 64,
            bootstrap_rtt_ms: 200,
            min_cap_frames: 40,
        };
        let mut state = ResponseInflightDisciplineState {
            effective_cap_frames: 56,
            effective_cap_sum: 0,
            effective_cap_samples: 0,
            effective_cap_min: 56,
            effective_cap_max: 56,
            inflight_cap_reduced_count: 0,
            inflight_cap_restore_count: 0,
            ack_pressure_events: 0,
            send_blocked_by_effective_cap: 0,
            pressure_streak: 0,
            max_pressure_streak: 0,
            restore_clean_streak: 0,
        };
        let clean_summary = AckLatencySummary {
            avg_ms: Some(180),
            min_ms: Some(150),
            p50_ms: Some(175),
            p95_ms: Some(220),
            max_ms: Some(250),
        };

        let cap_while_still_busy = state.update(config, clean_summary, 0, 52);
        assert_eq!(cap_while_still_busy.effective_cap_frames, 56);
        assert_eq!(state.inflight_cap_restore_count, 0);

        for _ in 0..(EXIT_RESPONSE_INFLIGHT_DISCIPLINE_RESTORE_STREAK_SAMPLES - 1) {
            let decision = state.update(config, clean_summary, 0, 24);
            assert_eq!(decision.effective_cap_frames, 56);
        }
        let restored = state.update(config, clean_summary, 0, 24);
        assert_eq!(restored.effective_cap_frames, 60);
        assert_eq!(restored.action, ResponseInflightCapAction::Restored);
        assert_eq!(state.inflight_cap_restore_count, 1);
    }
}
