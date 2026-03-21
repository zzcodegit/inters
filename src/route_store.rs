use crate::addr::Protocol;
use crate::protocol::ResponseQualityFeedback;
use crate::route::Route;
use crate::routing::pheromone::{PheromoneStats, PheromoneStore};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tracing::debug;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TransportKey {
    pub protocol: Protocol,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TransportStats {
    pub success_count: u64,
    pub failure_count: u64,
    pub recent_rtt_ms: Option<u32>,
    pub last_success_at_ms: Option<u64>,
    pub last_failure_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RouteQualityStats {
    pub local_samples: u64,
    pub feedback_samples: u64,
    pub success_count: u64,
    pub failure_count: u64,
    pub last_success_at_ms: Option<u64>,
    pub last_failure_at_ms: Option<u64>,
    pub last_feedback_at_ms: Option<u64>,
    pub local_ttfb_ms: Option<u64>,
    pub local_total_ms: Option<u64>,
    pub response_bytes: Option<u64>,
    pub ack_latency_ms_avg: Option<u64>,
    pub ack_latency_ms_p95: Option<u64>,
    pub retransmit_rate_ppm: Option<u32>,
    pub window_wait_total_ms: Option<u64>,
    pub window_wait_ratio_ppm: Option<u32>,
    pub overlay_first_send_gap_ms: Option<u64>,
    pub last_http_code: Option<u16>,
}

#[derive(Debug, Clone, Default)]
pub struct LocalRouteObservation {
    pub success: bool,
    pub status_code: Option<u16>,
    pub ttfb_ms: Option<u64>,
    pub total_ms: u64,
    pub response_bytes: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RouteFailureKind {
    Timeout,
    Unreachable,
    Refused,
    ProtocolMismatch,
    Congestion,
    Unknown,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

/// Confidence from total samples. Gradually increases and caps at 1.0.
pub fn confidence_from_samples(total: u64, n: u64) -> f32 {
    if n == 0 {
        return 0.0;
    }
    (total as f32 / n as f32).clamp(0.0, 1.0)
}

fn age_weight(now_ms: u64, last_event_ms: u64, half_life_ms: u64) -> f32 {
    if half_life_ms == 0 {
        return 1.0;
    }
    if last_event_ms == 0 {
        return 1.0;
    }
    let age = now_ms.saturating_sub(last_event_ms) as f32;
    let hl = half_life_ms as f32;
    let w = (-0.69314718056f32 * (age / hl)).exp();
    w.clamp(0.05, 1.0)
}

fn ema_u64(slot: &mut Option<u64>, sample: u64) {
    *slot = Some(match *slot {
        None => sample,
        Some(prev) => ((prev as u128 * 7 + sample as u128 * 3) / 10) as u64,
    });
}

fn ema_u32(slot: &mut Option<u32>, sample: u32) {
    *slot = Some(match *slot {
        None => sample,
        Some(prev) => ((prev as u128 * 7 + sample as u128 * 3) / 10) as u32,
    });
}

fn lower_is_better_factor(
    value: Option<u64>,
    good_ms: u64,
    bad_ms: u64,
    best_factor: f32,
    worst_factor: f32,
) -> f32 {
    let Some(value) = value else {
        return 1.0;
    };
    if good_ms >= bad_ms {
        return 1.0;
    }
    if value <= good_ms {
        return best_factor;
    }
    if value >= bad_ms {
        return worst_factor;
    }
    let span = (bad_ms - good_ms) as f32;
    let pos = (value - good_ms) as f32 / span;
    best_factor + (worst_factor - best_factor) * pos
}

fn lower_ratio_is_better_factor(
    value_ppm: Option<u32>,
    good_ppm: u32,
    bad_ppm: u32,
    best_factor: f32,
    worst_factor: f32,
) -> f32 {
    let Some(value_ppm) = value_ppm else {
        return 1.0;
    };
    if good_ppm >= bad_ppm {
        return 1.0;
    }
    if value_ppm <= good_ppm {
        return best_factor;
    }
    if value_ppm >= bad_ppm {
        return worst_factor;
    }
    let span = (bad_ppm - good_ppm) as f32;
    let pos = (value_ppm - good_ppm) as f32 / span;
    best_factor + (worst_factor - best_factor) * pos
}

fn geometric_mean(factors: &[f32]) -> f32 {
    if factors.is_empty() {
        return 1.0;
    }
    let product = factors
        .iter()
        .copied()
        .fold(1.0f32, |acc, factor| acc * factor.max(0.01));
    product.powf(1.0 / factors.len() as f32)
}

#[derive(Debug, Clone)]
struct RouteQualityFactorDetails {
    factor: f32,
    hop_factor: f32,
    local_ttfb_ms: Option<u64>,
    local_total_ms: Option<u64>,
    ack_latency_p95_ms: Option<u64>,
    retransmit_rate_ppm: Option<u32>,
    window_wait_ratio_ppm: Option<u32>,
    success_count: u64,
    failure_count: u64,
}

impl RouteQualityStats {
    fn record_local_observation(&mut self, sample: &LocalRouteObservation) {
        let now = now_ms();
        self.local_samples = self.local_samples.saturating_add(1);
        self.last_http_code = sample.status_code;
        ema_u64(&mut self.local_total_ms, sample.total_ms);
        ema_u64(&mut self.response_bytes, sample.response_bytes as u64);
        if let Some(ttfb_ms) = sample.ttfb_ms {
            ema_u64(&mut self.local_ttfb_ms, ttfb_ms);
        }
        if sample.success {
            self.success_count = self.success_count.saturating_add(1);
            self.last_success_at_ms = Some(now);
            if self.failure_count > 0 {
                self.failure_count = self.failure_count.saturating_sub(1);
            }
        } else {
            self.failure_count = self.failure_count.saturating_add(1);
            self.last_failure_at_ms = Some(now);
        }
    }

    fn record_feedback(&mut self, feedback: &ResponseQualityFeedback) {
        self.feedback_samples = self.feedback_samples.saturating_add(1);
        self.last_feedback_at_ms = Some(now_ms());
        self.last_http_code = feedback.http_code.or(self.last_http_code);
        ema_u64(&mut self.response_bytes, feedback.resp_bytes);
        if let Some(value) = feedback.ack_latency_ms_avg {
            ema_u64(&mut self.ack_latency_ms_avg, value);
        }
        if let Some(value) = feedback.ack_latency_ms_p95 {
            ema_u64(&mut self.ack_latency_ms_p95, value);
        }
        ema_u64(
            &mut self.window_wait_total_ms,
            feedback.window_wait_total_ms,
        );
        ema_u32(
            &mut self.retransmit_rate_ppm,
            feedback.retransmit_rate_ppm.min(1_000_000),
        );
        if feedback.stream_duration_ms > 0 {
            let ratio_ppm = ((feedback.window_wait_total_ms as u128 * 1_000_000u128)
                / feedback.stream_duration_ms as u128)
                .min(u32::MAX as u128) as u32;
            ema_u32(&mut self.window_wait_ratio_ppm, ratio_ppm);
        }
        if let (Some(first_target), Some(first_overlay)) = (
            feedback.first_target_byte_ms,
            feedback.first_overlay_send_ms,
        ) {
            ema_u64(
                &mut self.overlay_first_send_gap_ms,
                first_overlay.saturating_sub(first_target),
            );
        }
    }

    fn score_details(&self, now_ms: u64, hop_count: usize) -> RouteQualityFactorDetails {
        let total_samples = self.local_samples.saturating_add(self.feedback_samples);
        let confidence = confidence_from_samples(total_samples, 8);

        let total_factor = lower_is_better_factor(self.local_total_ms, 700, 6_000, 1.12, 0.70);
        let ttfb_factor = lower_is_better_factor(self.local_ttfb_ms, 250, 4_000, 1.10, 0.68);
        let ack_factor = lower_is_better_factor(self.ack_latency_ms_p95, 180, 1_200, 1.08, 0.68);
        let retransmit_factor =
            lower_ratio_is_better_factor(self.retransmit_rate_ppm, 0, 120_000, 1.06, 0.65);
        let stall_factor =
            lower_ratio_is_better_factor(self.window_wait_ratio_ppm, 20_000, 500_000, 1.05, 0.68);
        let success_rel = (self.success_count as f32 + 1.0)
            / (self.success_count.saturating_add(self.failure_count) as f32 + 2.0);
        let success_factor = 0.85 + success_rel * 0.30;

        let mut factors = vec![total_factor, ttfb_factor, success_factor];
        if self.ack_latency_ms_p95.is_some() {
            factors.push(ack_factor);
        }
        if self.retransmit_rate_ppm.is_some() {
            factors.push(retransmit_factor);
        }
        if self.window_wait_ratio_ppm.is_some() {
            factors.push(stall_factor);
        }

        let measured_quality = geometric_mean(&factors).clamp(0.65, 1.25);
        let mut factor = ((1.0 - confidence) + confidence * measured_quality).clamp(0.75, 1.25);

        let last_failure = self.last_failure_at_ms.unwrap_or(0);
        let last_success = self.last_success_at_ms.unwrap_or(0);
        if last_failure > last_success && now_ms.saturating_sub(last_failure) < 20_000 {
            factor *= 0.88;
        }

        let last_feedback = self
            .last_feedback_at_ms
            .or(self.last_success_at_ms)
            .or(self.last_failure_at_ms)
            .unwrap_or(0);
        factor *= age_weight(now_ms, last_feedback, 5 * 60 * 1_000);
        factor = factor.clamp(0.60, 1.25);

        let hop_factor = (1.0 - (hop_count.saturating_sub(1) as f32 * 0.02)).clamp(0.92, 1.0);

        RouteQualityFactorDetails {
            factor,
            hop_factor,
            local_ttfb_ms: self.local_ttfb_ms,
            local_total_ms: self.local_total_ms,
            ack_latency_p95_ms: self.ack_latency_ms_p95,
            retransmit_rate_ppm: self.retransmit_rate_ppm,
            window_wait_ratio_ppm: self.window_wait_ratio_ppm,
            success_count: self.success_count,
            failure_count: self.failure_count,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RouteCandidate {
    pub route: Route,
    pub rtt_ms: Option<u64>,
    pub success_rate: f32,
    pub failure_count: u32,
    pub last_used: Instant,
    pub quality: RouteQualityStats,
}

impl RouteCandidate {
    fn base_score(&self) -> f32 {
        let rtt_part = match self.rtt_ms {
            Some(ms) if ms > 0 => 1.0f32 / (ms as f32),
            _ => 0.0,
        };
        let mut score = (self.success_rate.clamp(0.0, 1.0) * 0.7) + (rtt_part * 0.3);
        if self.failure_count > 0 {
            let penalty = 0.80f32.powi(self.failure_count.min(10) as i32);
            score *= penalty;
        }
        score
    }

    fn transport_factor(&self, now: Instant, ts: &HashMap<TransportKey, TransportStats>) -> f32 {
        let now_ms = now_ms();
        let cooldown_ms: u64 = 5_000;
        let confidence_n: u64 = 20;
        let stats_half_life_ms: u64 = 5 * 60 * 1_000;

        let mut hop_effect_prod: f32 = 1.0;
        let mut hop_effect_min: f32 = 1.0;
        for hop in &self.route.hops {
            let key = TransportKey {
                protocol: hop.protocol,
                port: hop.port,
            };
            if let Some(st) = ts.get(&key) {
                let succ_raw = st.success_count as f32;
                let fail_raw = st.failure_count as f32;
                let last_fail = st.last_failure_at_ms.unwrap_or(0);
                let last_succ = st.last_success_at_ms.unwrap_or(0);
                let last_event = last_fail.max(last_succ);

                let w = age_weight(now_ms, last_event, stats_half_life_ms);
                let succ = succ_raw * w;
                let fail = fail_raw * w;
                let total_eff = (succ + fail).round().max(0.0) as u64;
                let confidence = confidence_from_samples(total_eff, confidence_n);

                let fail_recovery = if last_succ > last_fail { 0.5 } else { 1.0 };
                let reliability = (succ + 1.0) / (succ + (fail * fail_recovery) + 2.0);
                let rel_adj = 0.85 + (reliability * 0.30);

                let mut recent_penalty = 1.0f32;
                if last_fail > last_succ && now_ms.saturating_sub(last_fail) < cooldown_ms {
                    recent_penalty *= 0.80;
                }
                if last_succ > last_fail && now_ms.saturating_sub(last_succ) < cooldown_ms {
                    recent_penalty = 1.0;
                }

                let transport_factor = (rel_adj * recent_penalty).clamp(0.7, 1.3);
                let effective = (1.0 - confidence) + confidence * transport_factor;
                hop_effect_prod *= effective;
                if effective < hop_effect_min {
                    hop_effect_min = effective;
                }
            }
        }

        let agg = (hop_effect_min * hop_effect_prod.sqrt()).clamp(0.7, 1.3);
        let staleness = self.staleness_factor(now);
        (agg * staleness).clamp(0.55, 1.40)
    }

    fn staleness_factor(&self, now: Instant) -> f32 {
        let age = now.saturating_duration_since(self.last_used);
        if age <= Duration::from_secs(30) {
            return 1.0;
        }
        let extra = age.as_secs().saturating_sub(30).min(300) as f32;
        (1.0 - (extra / 300.0) * 0.15).clamp(0.7, 1.0)
    }
}

#[derive(Debug, Clone)]
pub struct RouteScoreDetails {
    pub base: f32,
    pub transport_agg: f32,
    pub quality_agg: f32,
    pub hop_factor: f32,
    pub final_score: f32,
    pub pheromone_score: f64,
    pub pheromone_confidence: f64,
    pub recent_ttfb_ms: Option<u64>,
    pub recent_total_ms: Option<u64>,
    pub recent_ack_p95_ms: Option<u64>,
    pub recent_retransmit_rate_ppm: Option<u32>,
    pub recent_window_wait_ratio_ppm: Option<u32>,
    pub recent_success_count: u64,
    pub recent_failure_count: u64,
}

#[derive(Debug)]
pub struct RouteStore {
    routes: Vec<RouteCandidate>,
    transport: HashMap<TransportKey, TransportStats>,
    pheromones: PheromoneStore,
    route_selection_count: u64,
    route_switch_count: u64,
    last_selected_fingerprint: Option<u64>,
}

impl RouteStore {
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
            transport: HashMap::new(),
            pheromones: PheromoneStore::new(1000),
            route_selection_count: 0,
            route_switch_count: 0,
            last_selected_fingerprint: None,
        }
    }

    pub fn routes(&self) -> &[RouteCandidate] {
        &self.routes
    }

    pub fn add_route(&mut self, route: Route, initial_success_rate: f32) {
        if self
            .routes
            .iter()
            .any(|candidate| candidate.route.hops == route.hops)
        {
            return;
        }
        self.routes.push(RouteCandidate {
            route,
            rtt_ms: None,
            success_rate: initial_success_rate.clamp(0.0, 1.0),
            failure_count: 0,
            last_used: Instant::now(),
            quality: RouteQualityStats::default(),
        });
    }

    pub fn transport_stats(&self) -> &HashMap<TransportKey, TransportStats> {
        &self.transport
    }

    pub fn pheromone_stats(&self) -> PheromoneStats {
        self.pheromones.stats(now_ms())
    }

    pub fn reinforce_transport_pheromone(
        &mut self,
        protocol: Protocol,
        port: u16,
        rtt_ms: Option<u64>,
        success: bool,
    ) {
        self.pheromones
            .reinforce_transport(protocol, port, rtt_ms, success);
    }

    pub fn score_details(&self, idx: usize, now: Instant) -> Option<RouteScoreDetails> {
        let candidate = self.routes.get(idx)?;
        let base = candidate.base_score();
        let transport_agg = candidate.transport_factor(now, &self.transport);
        let quality = candidate
            .quality
            .score_details(now_ms(), candidate.route.len());
        let (pheromone_score, pheromone_confidence) =
            self.pheromones.score_for_route(&candidate.route);
        let pheromone_factor = (1.0f32 + 0.3f32 * pheromone_score as f32).clamp(0.7, 1.5);
        let final_score =
            base * transport_agg * quality.factor * quality.hop_factor * pheromone_factor;

        Some(RouteScoreDetails {
            base,
            transport_agg,
            quality_agg: quality.factor,
            hop_factor: quality.hop_factor,
            final_score,
            pheromone_score,
            pheromone_confidence,
            recent_ttfb_ms: quality.local_ttfb_ms,
            recent_total_ms: quality.local_total_ms,
            recent_ack_p95_ms: quality.ack_latency_p95_ms,
            recent_retransmit_rate_ppm: quality.retransmit_rate_ppm,
            recent_window_wait_ratio_ppm: quality.window_wait_ratio_ppm,
            recent_success_count: quality.success_count,
            recent_failure_count: quality.failure_count,
        })
    }

    pub fn record_transport_success(&mut self, key: TransportKey, rtt_ms: Option<u64>) {
        let now_ms = now_ms();
        let stats = self.transport.entry(key).or_default();
        stats.success_count = stats.success_count.saturating_add(1);
        stats.last_success_at_ms = Some(now_ms);
        if let Some(rtt_ms) = rtt_ms {
            stats.recent_rtt_ms = Some(rtt_ms.min(u32::MAX as u64) as u32);
        }
        if stats.failure_count > 0 {
            stats.failure_count = stats.failure_count.saturating_sub(1);
        }
    }

    pub fn record_transport_failure(&mut self, key: TransportKey, kind: RouteFailureKind) {
        let now_ms = now_ms();
        let stats = self.transport.entry(key).or_default();
        stats.failure_count = stats.failure_count.saturating_add(1);
        stats.last_failure_at_ms = Some(now_ms);
        if kind == RouteFailureKind::ProtocolMismatch {
            stats.failure_count = stats.failure_count.saturating_add(5);
        }
    }

    pub fn decay_scores(&mut self, now: Instant) {
        for candidate in &mut self.routes {
            let age = now.saturating_duration_since(candidate.last_used);
            if age > Duration::from_secs(120) {
                candidate.success_rate =
                    (candidate.success_rate * 0.98 + 0.5 * 0.02).clamp(0.0, 1.0);
            }
        }
        self.pheromones.decay(now_ms());
    }

    pub fn scored_candidates(
        &mut self,
        now: Instant,
        exclude: &[Route],
    ) -> Vec<(usize, RouteScoreDetails)> {
        self.decay_scores(now);
        let mut scored = Vec::new();
        for (idx, candidate) in self.routes.iter().enumerate() {
            if exclude
                .iter()
                .any(|route| route.hops == candidate.route.hops)
            {
                continue;
            }
            if candidate.failure_count >= 3
                && now.saturating_duration_since(candidate.last_used) < Duration::from_secs(10)
            {
                continue;
            }
            if let Some(details) = self.score_details(idx, now) {
                scored.push((idx, details));
            }
        }

        scored.sort_by(|left, right| {
            right
                .1
                .final_score
                .partial_cmp(&left.1.final_score)
                .unwrap_or(Ordering::Equal)
        });
        scored
    }

    pub fn note_route_selection(&mut self, idx: usize, score: f32) {
        self.route_selection_count = self.route_selection_count.saturating_add(1);
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        let route_chain = if let Some(candidate) = self.routes.get(idx) {
            candidate.route.hops.hash(&mut hasher);
            candidate
                .route
                .hops
                .iter()
                .map(|hop| hop.to_string())
                .collect::<Vec<_>>()
                .join(" -> ")
        } else {
            "<missing>".to_string()
        };
        let fingerprint = hasher.finish();
        let switched = matches!(
            self.last_selected_fingerprint,
            Some(previous) if previous != fingerprint
        );
        if switched {
            self.route_switch_count = self.route_switch_count.saturating_add(1);
        }
        self.last_selected_fingerprint = Some(fingerprint);
        debug!(
            event = "route_select",
            idx,
            score,
            route = %route_chain,
            selection_count = self.route_selection_count,
            switch_count = self.route_switch_count,
            switched
        );
    }

    pub fn get_best_route(&mut self, now: Instant, exclude: &[Route]) -> Option<(usize, f32)> {
        let scored = self.scored_candidates(now, exclude);
        let (idx, details) = scored.first()?.clone();
        self.note_route_selection(idx, details.final_score);
        Some((idx, details.final_score))
    }

    pub fn mark_failure(&mut self, idx: usize) {
        if let Some(candidate) = self.routes.get_mut(idx) {
            candidate.failure_count = candidate.failure_count.saturating_add(1);
            candidate.success_rate = (candidate.success_rate * 0.90).clamp(0.0, 1.0);
            candidate.last_used = Instant::now();
        }
    }

    pub fn update_metrics(&mut self, idx: usize, rtt_ms: Option<u64>, success: bool) {
        if let Some(candidate) = self.routes.get_mut(idx) {
            candidate.last_used = Instant::now();
            if let Some(rtt_ms) = rtt_ms {
                candidate.rtt_ms = Some(match candidate.rtt_ms {
                    None => rtt_ms,
                    Some(previous) => ((previous as u128 * 8 + rtt_ms as u128 * 2) / 10) as u64,
                });
            }
            if success {
                candidate.failure_count = 0;
                candidate.success_rate =
                    (candidate.success_rate * 0.85 + 1.0 * 0.15).clamp(0.0, 1.0);
            } else {
                candidate.failure_count = candidate.failure_count.saturating_add(1);
                candidate.success_rate = (candidate.success_rate * 0.90).clamp(0.0, 1.0);
            }
        }
    }

    pub fn record_success(&mut self, idx: usize, rtt_ms: Option<u64>) {
        self.update_metrics(idx, rtt_ms, true);
        let now_ms = now_ms();
        let Some(route) = self
            .routes
            .get(idx)
            .map(|candidate| candidate.route.clone())
        else {
            return;
        };
        for hop in &route.hops {
            let key = TransportKey {
                protocol: hop.protocol,
                port: hop.port,
            };
            let stats = self.transport.entry(key).or_default();
            stats.success_count = stats.success_count.saturating_add(1);
            if let (Some(last_fail), Some(last_succ)) =
                (stats.last_failure_at_ms, stats.last_success_at_ms)
            {
                if last_succ > last_fail {
                    stats.failure_count = stats.failure_count.saturating_sub(1);
                }
            }
            stats.last_success_at_ms = Some(now_ms);
            if let Some(rtt_ms) = rtt_ms {
                stats.recent_rtt_ms = Some(rtt_ms.min(u32::MAX as u64) as u32);
            }
            self.pheromones
                .reinforce_transport(hop.protocol, hop.port, rtt_ms, true);
        }
    }

    pub fn record_failure(&mut self, idx: usize, kind: RouteFailureKind) {
        self.update_metrics(idx, None, false);
        let now_ms = now_ms();
        let Some(route) = self
            .routes
            .get(idx)
            .map(|candidate| candidate.route.clone())
        else {
            return;
        };
        for hop in &route.hops {
            let key = TransportKey {
                protocol: hop.protocol,
                port: hop.port,
            };
            let stats = self.transport.entry(key).or_default();
            stats.failure_count = stats.failure_count.saturating_add(1);
            stats.last_failure_at_ms = Some(now_ms);
            if kind == RouteFailureKind::ProtocolMismatch {
                stats.failure_count = stats.failure_count.saturating_add(5);
            }
            self.pheromones
                .reinforce_transport(hop.protocol, hop.port, None, false);
        }
    }

    pub fn record_local_observation(&mut self, idx: usize, sample: &LocalRouteObservation) {
        if let Some(candidate) = self.routes.get_mut(idx) {
            candidate.quality.record_local_observation(sample);
        }
    }

    pub fn record_response_quality_for_route(
        &mut self,
        route: &Route,
        feedback: &ResponseQualityFeedback,
    ) -> bool {
        let Some(candidate) = self
            .routes
            .iter_mut()
            .find(|candidate| candidate.route.hops == route.hops)
        else {
            return false;
        };
        candidate.quality.record_feedback(feedback);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::addr::NodeAddr;

    fn r(s: &str) -> std::net::SocketAddr {
        s.parse().unwrap()
    }

    fn sample_feedback(
        route_len: u8,
        ack_p95_ms: u64,
        retransmit_rate_ppm: u32,
        window_wait_total_ms: u64,
        stream_duration_ms: u64,
    ) -> ResponseQualityFeedback {
        ResponseQualityFeedback {
            stream_id: 7,
            route_len,
            resp_bytes: 256 * 1024,
            frames_sent: 256,
            stream_duration_ms,
            first_target_byte_ms: Some(2),
            first_overlay_send_ms: Some(4),
            ack_latency_ms_avg: Some(ack_p95_ms / 2),
            ack_latency_ms_p50: Some(ack_p95_ms / 3),
            ack_latency_ms_p95: Some(ack_p95_ms),
            ack_latency_ms_max: Some(ack_p95_ms.saturating_add(50)),
            total_retransmits: 3,
            retransmit_rate_ppm,
            window_wait_events: 8,
            window_wait_total_ms,
            window_wait_max_ms: 80,
            http_code: Some(200),
        }
    }

    #[test]
    fn best_route_prefers_success_and_rtt() {
        let mut store = RouteStore::new();
        store.add_route(
            Route {
                hops: vec![NodeAddr::from(r("127.0.0.1:1"))],
            },
            0.6,
        );
        store.add_route(
            Route {
                hops: vec![
                    NodeAddr::from(r("127.0.0.1:2")),
                    NodeAddr::from(r("127.0.0.1:3")),
                ],
            },
            0.6,
        );

        store.update_metrics(1, Some(5), true);
        store.update_metrics(0, Some(200), true);

        let now = Instant::now();
        let (idx, _score) = store.get_best_route(now, &[]).unwrap();
        assert_eq!(idx, 1);
    }

    #[test]
    fn failures_are_penalized_and_temporarily_avoided() {
        let mut store = RouteStore::new();
        store.add_route(
            Route {
                hops: vec![NodeAddr::from(r("127.0.0.1:1"))],
            },
            0.9,
        );
        store.add_route(
            Route {
                hops: vec![NodeAddr::from(r("127.0.0.1:2"))],
            },
            0.9,
        );
        store.mark_failure(0);
        store.mark_failure(0);
        store.mark_failure(0);

        let now = Instant::now();
        let (idx, _score) = store.get_best_route(now, &[]).unwrap();
        assert_eq!(idx, 1, "recently failing route should be avoided");
    }

    #[test]
    fn transport_stats_recent_failure_penalizes_route() {
        let mut store = RouteStore::new();
        store.add_route(
            Route {
                hops: vec![NodeAddr::from(r("127.0.0.1:1111"))],
            },
            0.9,
        );
        store.add_route(
            Route {
                hops: vec![NodeAddr::from(r("127.0.0.1:2222"))],
            },
            0.9,
        );

        store.record_failure(0, RouteFailureKind::Timeout);
        store.record_success(1, Some(10));

        let now = Instant::now();
        let (idx, _score) = store.get_best_route(now, &[]).unwrap();
        assert_eq!(idx, 1);
    }

    #[test]
    fn confidence_prevents_overreaction_to_single_sample() {
        let mut store = RouteStore::new();
        store.add_route(
            Route {
                hops: vec![NodeAddr::from(r("127.0.0.1:1111"))],
            },
            0.8,
        );
        store.add_route(
            Route {
                hops: vec![NodeAddr::from(r("127.0.0.1:2222"))],
            },
            0.8,
        );
        store.update_metrics(0, Some(50), true);
        store.update_metrics(1, Some(50), true);
        store.record_failure(0, RouteFailureKind::Timeout);

        let now = Instant::now();
        let (_idx, score) = store.get_best_route(now, &[]).unwrap();
        assert!(score.is_finite());
        assert!(score > 0.0);
    }

    #[test]
    fn confidence_from_samples_ramps_up() {
        assert_eq!(confidence_from_samples(0, 20), 0.0);
        assert!(confidence_from_samples(1, 20) > 0.0);
        assert!(confidence_from_samples(10, 20) < 1.0);
        assert_eq!(confidence_from_samples(20, 20), 1.0);
        assert_eq!(confidence_from_samples(100, 20), 1.0);
    }

    #[test]
    fn longer_route_can_win_when_quality_is_better() {
        let mut store = RouteStore::new();
        let short = Route {
            hops: vec![NodeAddr::from(r("127.0.0.1:3001"))],
        };
        let long = Route {
            hops: vec![
                NodeAddr::from(r("127.0.0.1:3002")),
                NodeAddr::from(r("127.0.0.1:3003")),
            ],
        };
        store.add_route(short.clone(), 0.85);
        store.add_route(long.clone(), 0.85);
        store.update_metrics(0, Some(90), true);
        store.update_metrics(1, Some(110), true);

        store.record_local_observation(
            0,
            &LocalRouteObservation {
                success: true,
                status_code: Some(200),
                ttfb_ms: Some(4_200),
                total_ms: 5_500,
                response_bytes: 256 * 1024,
            },
        );
        store.record_local_observation(
            1,
            &LocalRouteObservation {
                success: true,
                status_code: Some(200),
                ttfb_ms: Some(850),
                total_ms: 1_300,
                response_bytes: 256 * 1024,
            },
        );
        assert!(store.record_response_quality_for_route(
            &short,
            &sample_feedback(1, 1_400, 120_000, 2_000, 5_000),
        ));
        assert!(store
            .record_response_quality_for_route(&long, &sample_feedback(2, 220, 0, 160, 1_300),));

        let now = Instant::now();
        let scored = store.scored_candidates(now, &[]);
        assert_eq!(scored.first().map(|(idx, _)| *idx), Some(1));
        assert!(
            scored[0].1.final_score > scored[1].1.final_score,
            "quality-aware scoring should let the longer but healthier route win"
        );
    }

    #[test]
    fn response_quality_feedback_matches_route_by_hops() {
        let mut store = RouteStore::new();
        let route = Route {
            hops: vec![
                NodeAddr::from(r("127.0.0.1:4001")),
                NodeAddr::from(r("127.0.0.1:4002")),
                NodeAddr::from(r("127.0.0.1:4003")),
            ],
        };
        store.add_route(route.clone(), 0.8);
        let applied = store.record_response_quality_for_route(
            &route,
            &sample_feedback(3, 310, 12_000, 450, 1_900),
        );
        assert!(applied);
        let details = store.score_details(0, Instant::now()).unwrap();
        assert_eq!(details.recent_ack_p95_ms, Some(310));
        assert_eq!(details.recent_retransmit_rate_ppm, Some(12_000));
        assert!(details.quality_agg.is_finite());
    }
}
