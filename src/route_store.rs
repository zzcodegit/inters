use crate::addr::Protocol;
use crate::route::Route;
use crate::routing::pheromone::{PheromoneStats, PheromoneStore};
use serde::{Deserialize, Serialize};
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
    // exp(-ln(2) * age/half_life)
    let w = (-0.69314718056f32 * (age / hl)).exp();
    w.clamp(0.05, 1.0)
}

#[derive(Debug, Clone)]
pub struct RouteCandidate {
    pub route: Route,
    pub rtt_ms: Option<u64>,
    pub success_rate: f32,
    pub failure_count: u32,
    pub last_used: Instant,
}

impl RouteCandidate {
    fn base_score(&self) -> f32 {
        let rtt_part = match self.rtt_ms {
            Some(ms) if ms > 0 => 1.0f32 / (ms as f32),
            _ => 0.0,
        };
        let mut s = (self.success_rate.clamp(0.0, 1.0) * 0.7) + (rtt_part * 0.3);
        if self.failure_count > 0 {
            let pen = 0.80f32.powi(self.failure_count.min(10) as i32);
            s *= pen;
        }
        s
    }

    fn score(&self, now: Instant, ts: &HashMap<TransportKey, TransportStats>) -> f32 {
        let mut s = self.base_score();

        // Transport-aware factor: per-hop protocol/port reliability and recent failure penalty.
        // Keep effect bounded for UDP-only stage: small per-hop adjustments with *confidence*.
        //
        // Key properties:
        // - low samples => low confidence => mostly fall back to baseline behavior
        // - recent success overrides old failure penalties quickly
        // - weakest hop should dominate (a bad hop breaks the path)
        let now_ms = now_ms();
        let cooldown_ms: u64 = 5_000;
        let confidence_n: u64 = 20;
        let stats_half_life_ms: u64 = 5 * 60 * 1_000; // 5 minutes

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

                // Aging: old stats lose influence (confidence and reliability).
                let w = age_weight(now_ms, last_event, stats_half_life_ms);
                let succ = succ_raw * w;
                let fail = fail_raw * w;
                let total_eff = (succ + fail).round().max(0.0) as u64;
                let confidence = confidence_from_samples(total_eff, confidence_n);

                // Smoothed reliability. With 0 samples it stays at 0.5-ish.
                // Recovery: if we had a success after failure, reduce the effect of historical failures.
                let fail_recovery = if last_succ > last_fail { 0.5 } else { 1.0 };
                let reliability = (succ + 1.0) / (succ + (fail * fail_recovery) + 2.0);
                // Map reliability into a narrow band to avoid large swings.
                let rel_adj = 0.85 + (reliability * 0.30); // ~[0.95..1.05] for typical values

                // Recent failure penalty only if it is newer than the last success.
                let mut recent_penalty = 1.0f32;
                if last_fail > last_succ && now_ms.saturating_sub(last_fail) < cooldown_ms {
                    recent_penalty *= 0.80;
                }
                // Stronger recovery: a success after failure reduces penalty quickly.
                if last_succ > last_fail && now_ms.saturating_sub(last_succ) < cooldown_ms {
                    recent_penalty = 1.0;
                }

                let transport_factor = (rel_adj * recent_penalty).clamp(0.7, 1.3);

                // Confidence gating: with low samples we mostly keep baseline (1.0).
                // effective = (1-confidence) + confidence*transport_factor
                let effective = (1.0 - confidence) + confidence * transport_factor;

                hop_effect_prod *= effective;
                if effective < hop_effect_min {
                    hop_effect_min = effective;
                }
            }
        }
        // Weakest-link aggregation: ensure one bad hop meaningfully penalizes the route.
        // Still keep it bounded (avoid collapsing to ~0 instantly).
        let agg = (hop_effect_min * hop_effect_prod.sqrt()).clamp(0.7, 1.3);
        s *= agg;

        // Mild decay for stale routes (older than 30s since last use).
        let age = now.saturating_duration_since(self.last_used);
        if age > Duration::from_secs(30) {
            let extra = age.as_secs().saturating_sub(30).min(300) as f32; // cap at 5 min
            let decay = (1.0 - (extra / 300.0) * 0.15).clamp(0.7, 1.0);
            s *= decay;
        }

        s
    }
}

#[derive(Debug, Clone)]
pub struct RouteScoreDetails {
    pub base: f32,
    pub transport_agg: f32,
    pub final_score: f32,
    pub pheromone_score: f64,
    pub pheromone_confidence: f64,
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
        // Avoid duplicates by hop list.
        if self
            .routes
            .iter()
            .any(|c| c.route.hops == route.hops)
        {
            return;
        }
        self.routes.push(RouteCandidate {
            route,
            rtt_ms: None,
            success_rate: initial_success_rate.clamp(0.0, 1.0),
            failure_count: 0,
            last_used: Instant::now(),
        });
    }

    pub fn transport_stats(&self) -> &HashMap<TransportKey, TransportStats> {
        &self.transport
    }

    pub fn pheromone_stats(&self) -> PheromoneStats {
        self.pheromones.stats(now_ms())
    }

    /// Expose a minimal API for pheromone reinforcement for callers that only know transport.
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
        let c = self.routes.get(idx)?;
        let base = c.base_score();
        let mut final_score = c.score(now, &self.transport);
        let (pher_score, pher_conf) = self.pheromones.score_for_route(&c.route);
        let pher_weight = 0.3f32;
        let pher_factor = (1.0f32 + pher_weight * (pher_score as f32)).clamp(0.7, 1.5);
        final_score *= pher_factor;
        let transport_agg = if base > 0.0 {
            (final_score / base).clamp(0.0, 10.0)
        } else {
            1.0
        };
        Some(RouteScoreDetails {
            base,
            transport_agg,
            final_score,
            pheromone_score: pher_score,
            pheromone_confidence: pher_conf,
        })
    }

    /// Stage 7+: feed transport-only samples (no route idx needed).
    pub fn record_transport_success(&mut self, key: TransportKey, rtt_ms: Option<u64>) {
        let now_ms = now_ms();
        let st = self.transport.entry(key).or_default();
        st.success_count += 1;
        st.last_success_at_ms = Some(now_ms);
        if let Some(ms) = rtt_ms {
            st.recent_rtt_ms = Some(ms.min(u32::MAX as u64) as u32);
        }
        // Recovery smoothing: success reduces stored failure count slowly.
        if st.failure_count > 0 {
            st.failure_count = st.failure_count.saturating_sub(1);
        }
    }

    pub fn record_transport_failure(&mut self, key: TransportKey, kind: RouteFailureKind) {
        let now_ms = now_ms();
        let st = self.transport.entry(key).or_default();
        st.failure_count += 1;
        st.last_failure_at_ms = Some(now_ms);
        if kind == RouteFailureKind::ProtocolMismatch {
            st.failure_count = st.failure_count.saturating_add(5);
        }
    }

    pub fn decay_scores(&mut self, now: Instant) {
        // Pull success_rate slowly back towards neutral if route is not used.
        for c in &mut self.routes {
            let age = now.saturating_duration_since(c.last_used);
            if age > Duration::from_secs(120) {
                // After 2 minutes of inactivity, drift toward 0.5.
                c.success_rate = (c.success_rate * 0.98 + 0.5 * 0.02).clamp(0.0, 1.0);
            }
        }
        let now_ms = now_ms();
        self.pheromones.decay(now_ms);
    }

    pub fn get_best_route(&mut self, now: Instant, exclude: &[Route]) -> Option<(usize, f32)> {
        self.decay_scores(now);
        let mut best: Option<(usize, f32)> = None;
        for (i, c) in self.routes.iter().enumerate() {
            if exclude.iter().any(|r| r.hops == c.route.hops) {
                continue;
            }

            // Temporary avoidance for repeatedly failing routes used very recently.
            if c.failure_count >= 3 && now.saturating_duration_since(c.last_used) < Duration::from_secs(10) {
                continue;
            }

            let mut sc = c.score(now, &self.transport);
            let (pher_score, _pher_conf) = self.pheromones.score_for_route(&c.route);
            let pher_weight = 0.3f32;
            let pher_factor = (1.0f32 + pher_weight * (pher_score as f32)).clamp(0.8, 1.3);
            sc *= pher_factor;
            match best {
                None => best = Some((i, sc)),
                Some((_, bsc)) if sc > bsc => best = Some((i, sc)),
                _ => {}
            }
        }
        if let Some((idx, score)) = best {
            // Route stability diagnostics: track how often best route changes.
            self.route_selection_count = self.route_selection_count.saturating_add(1);
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            if let Some(c) = self.routes.get(idx) {
                c.route.hops.hash(&mut hasher);
            }
            let fp = hasher.finish();
            let switched = match self.last_selected_fingerprint {
                Some(prev) if prev != fp => true,
                None => false,
                _ => false,
            };
            if switched {
                self.route_switch_count = self.route_switch_count.saturating_add(1);
            }
            self.last_selected_fingerprint = Some(fp);
            debug!(
                event = "route_select",
                idx,
                score,
                selection_count = self.route_selection_count,
                switch_count = self.route_switch_count,
                switched
            );
            Some((idx, score))
        } else {
            None
        }
    }

    pub fn mark_failure(&mut self, idx: usize) {
        if let Some(c) = self.routes.get_mut(idx) {
            c.failure_count = c.failure_count.saturating_add(1);
            c.success_rate = (c.success_rate * 0.90).clamp(0.0, 1.0);
            c.last_used = Instant::now();
        }
    }

    pub fn update_metrics(&mut self, idx: usize, rtt_ms: Option<u64>, success: bool) {
        // Backward-compatible entry point: update route-level state only.
        if let Some(c) = self.routes.get_mut(idx) {
            c.last_used = Instant::now();
            if let Some(ms) = rtt_ms {
                c.rtt_ms = Some(match c.rtt_ms {
                    None => ms,
                    Some(prev) => ((prev as u128 * 8 + ms as u128 * 2) / 10) as u64, // EMA-ish
                });
            }
            if success {
                c.failure_count = 0;
                c.success_rate = (c.success_rate * 0.85 + 1.0 * 0.15).clamp(0.0, 1.0);
            } else {
                c.failure_count = c.failure_count.saturating_add(1);
                c.success_rate = (c.success_rate * 0.90).clamp(0.0, 1.0);
            }
        }
    }

    pub fn record_success(&mut self, idx: usize, rtt_ms: Option<u64>) {
        self.update_metrics(idx, rtt_ms, true);
        let now_ms = now_ms();
        let Some(route) = self.routes.get(idx).map(|c| c.route.clone()) else {
            return;
        };
        for hop in &route.hops {
            let key = TransportKey {
                protocol: hop.protocol,
                port: hop.port,
            };
            let st = self.transport.entry(key).or_default();
            st.success_count += 1;
            // If we recovered (success after a failure), decay the stored failure count a bit.
            if let (Some(last_fail), Some(last_succ)) = (st.last_failure_at_ms, st.last_success_at_ms) {
                if last_succ > last_fail {
                    st.failure_count = st.failure_count.saturating_sub(1);
                }
            }
            st.last_success_at_ms = Some(now_ms);
            if let Some(ms) = rtt_ms {
                st.recent_rtt_ms = Some(ms.min(u32::MAX as u64) as u32);
            }
            // Pheromone reinforcement: treat a successful route as a positive signal for its hops.
            self.pheromones
                .reinforce_transport(hop.protocol, hop.port, rtt_ms, true);
        }
    }

    pub fn record_failure(&mut self, idx: usize, kind: RouteFailureKind) {
        self.update_metrics(idx, None, false);
        let now_ms = now_ms();
        let Some(route) = self.routes.get(idx).map(|c| c.route.clone()) else {
            return;
        };
        for hop in &route.hops {
            let key = TransportKey {
                protocol: hop.protocol,
                port: hop.port,
            };
            let st = self.transport.entry(key).or_default();
            st.failure_count += 1;
            st.last_failure_at_ms = Some(now_ms);

            // Small hint: protocol mismatch should basically quarantine this key.
            if kind == RouteFailureKind::ProtocolMismatch {
                st.failure_count = st.failure_count.saturating_add(5);
            }
            // Pheromone reinforcement for failure (negative signal).
            self.pheromones
                .reinforce_transport(hop.protocol, hop.port, None, false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::addr::NodeAddr;

    fn r(s: &str) -> std::net::SocketAddr {
        s.parse().unwrap()
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

        // Give second route much better RTT so it can win.
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
        // mark route 0 failing multiple times
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

        // Mark first route as failing; second as succeeding.
        store.record_failure(0, RouteFailureKind::Timeout);
        store.record_success(1, Some(10));

        let now = Instant::now();
        let (idx, _score) = store.get_best_route(now, &[]).unwrap();
        assert_eq!(idx, 1);
    }

    #[test]
    fn confidence_prevents_overreaction_to_single_sample() {
        // Two identical routes by base metrics, but one has a single failure sample
        // on its transport key. With confidence gating (N=20) it should not dominate.
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

        // One failure on route 0 key.
        store.record_failure(0, RouteFailureKind::Timeout);

        // It should still be eligible; we just want to ensure score doesn't collapse.
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
}

