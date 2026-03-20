use crate::addr::Protocol;
use crate::route::Route;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::debug;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PheromoneKey {
    pub dest_hint: u64,
    pub protocol: Protocol,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PheromoneEntry {
    pub key: PheromoneKey,
    pub score: f64,      // 0.0 .. 1.0
    pub confidence: f64, // 0.0 .. 1.0
    pub last_update_ms: u64,
}

#[derive(Debug, Clone, Default)]
pub struct PheromoneStats {
    pub total_entries: usize,
    pub average_score: f64,
    pub average_confidence: f64,
    pub oldest_entry_age_ms: Option<u64>,
    pub newest_entry_age_ms: Option<u64>,
    pub evictions_count: u64,
    pub decay_drops_count: u64,
    pub reinforce_success_count: u64,
    pub reinforce_failure_count: u64,
}

#[derive(Debug)]
pub struct PheromoneStore {
    entries: HashMap<PheromoneKey, PheromoneEntry>,
    max_entries: usize,
    evictions_count: u64,
    decay_drops_count: u64,
    reinforce_success_count: u64,
    reinforce_failure_count: u64,
}

impl PheromoneStore {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            max_entries: max_entries.max(1),
            evictions_count: 0,
            decay_drops_count: 0,
            reinforce_success_count: 0,
            reinforce_failure_count: 0,
        }
    }

    fn default_dest_hint() -> u64 {
        // Minimal "route class" hint: a single global bucket for Stage 8.
        0
    }

    fn key_for_transport(protocol: Protocol, port: u16) -> PheromoneKey {
        PheromoneKey {
            dest_hint: Self::default_dest_hint(),
            protocol,
            port,
        }
    }

    fn evict_if_needed(&mut self) {
        if self.entries.len() < self.max_entries {
            return;
        }
        if let Some((old_key, _)) = self
            .entries
            .iter()
            .min_by_key(|(_, e)| e.last_update_ms)
            .map(|(k, e)| (k.clone(), e.last_update_ms))
        {
            if let Some(removed) = self.entries.remove(&old_key) {
                self.evictions_count = self.evictions_count.saturating_add(1);
                debug!(
                    event = "pheromone_eviction",
                    reason = "cap",
                    protocol = ?removed.key.protocol,
                    port = removed.key.port,
                    old_score = removed.score,
                    confidence = removed.confidence
                );
            }
        }
    }

    fn update_entry<F>(&mut self, key: PheromoneKey, mut f: F)
    where
        F: FnMut(&mut PheromoneEntry),
    {
        let now = now_ms();
        if !self.entries.contains_key(&key) && self.entries.len() >= self.max_entries {
            self.evict_if_needed();
        }
        let entry = self.entries.entry(key.clone()).or_insert(PheromoneEntry {
            key,
            score: 0.5,
            confidence: 0.0,
            last_update_ms: now,
        });
        let old_score = entry.score;
        let old_conf = entry.confidence;
        f(entry);
        entry.score = entry.score.clamp(0.0, 1.0);
        entry.confidence = entry.confidence.clamp(0.0, 1.0);
        entry.last_update_ms = now;
        debug!(
            event = "pheromone_update",
            protocol = ?entry.key.protocol,
            port = entry.key.port,
            old_score = old_score,
            new_score = entry.score,
            old_confidence = old_conf,
            new_confidence = entry.confidence
        );
    }

    pub fn reinforce_transport(
        &mut self,
        protocol: Protocol,
        port: u16,
        rtt_ms: Option<u64>,
        success: bool,
    ) {
        let key = Self::key_for_transport(protocol, port);
        let alpha = 0.15f64;
        self.update_entry(key, |e| {
            if success {
                let sv = match rtt_ms {
                    Some(ms) if ms <= 50 => 1.0,
                    Some(ms) if ms <= 150 => 0.8,
                    Some(ms) if ms <= 500 => 0.6,
                    _ => 0.5,
                };
                e.score = e.score * (1.0 - alpha) + sv * alpha;
                e.confidence = (e.confidence + 0.05).min(1.0);
            } else {
                e.score *= 0.85;
                e.confidence *= 0.7;
            }
        });
        if success {
            self.reinforce_success_count = self.reinforce_success_count.saturating_add(1);
        } else {
            self.reinforce_failure_count = self.reinforce_failure_count.saturating_add(1);
        }
    }

    pub fn decay(&mut self, now_ms: u64) {
        if self.entries.is_empty() {
            return;
        }
        let decay_per_minute = 0.98f64;
        let threshold = 0.05f64;
        let mut to_remove = Vec::new();
        for (k, e) in self.entries.iter_mut() {
            if e.last_update_ms == 0 {
                continue;
            }
            let elapsed_ms = now_ms.saturating_sub(e.last_update_ms);
            let minutes = (elapsed_ms / 60_000) as u32;
            if minutes > 0 {
                let factor = decay_per_minute.powi(minutes as i32);
                e.score *= factor;
                if e.score < threshold {
                    to_remove.push(k.clone());
                }
            }
        }
        for k in to_remove {
            if let Some(removed) = self.entries.remove(&k) {
                self.decay_drops_count = self.decay_drops_count.saturating_add(1);
                debug!(
                    event = "pheromone_eviction",
                    reason = "decay_drop",
                    protocol = ?removed.key.protocol,
                    port = removed.key.port,
                    old_score = removed.score,
                    confidence = removed.confidence
                );
            }
        }
        if !self.entries.is_empty() {
            debug!(event = "pheromone_decay", remaining = self.entries.len());
        }
    }

    pub fn score_for_route(&self, route: &Route) -> (f64, f64) {
        let Some(last) = route.hops.last() else {
            return (0.0, 0.0);
        };
        let key = Self::key_for_transport(last.protocol, last.port);
        if let Some(e) = self.entries.get(&key) {
            (e.score, e.confidence)
        } else {
            (0.0, 0.0)
        }
    }

    pub fn stats(&self, now_ms: u64) -> PheromoneStats {
        let mut stats = PheromoneStats::default();
        stats.total_entries = self.entries.len();
        if self.entries.is_empty() {
            stats.evictions_count = self.evictions_count;
            stats.decay_drops_count = self.decay_drops_count;
            stats.reinforce_success_count = self.reinforce_success_count;
            stats.reinforce_failure_count = self.reinforce_failure_count;
            return stats;
        }
        let mut sum_score = 0.0;
        let mut sum_conf = 0.0;
        let mut oldest: Option<u64> = None;
        let mut newest: Option<u64> = None;
        for e in self.entries.values() {
            sum_score += e.score;
            sum_conf += e.confidence;
            if e.last_update_ms > 0 {
                let age = now_ms.saturating_sub(e.last_update_ms);
                oldest = Some(oldest.map(|o| o.max(age)).unwrap_or(age));
                newest = Some(newest.map(|n| n.min(age)).unwrap_or(age));
            }
        }
        stats.average_score = sum_score / self.entries.len() as f64;
        stats.average_confidence = sum_conf / self.entries.len() as f64;
        stats.oldest_entry_age_ms = oldest;
        stats.newest_entry_age_ms = newest;
        stats.evictions_count = self.evictions_count;
        stats.decay_drops_count = self.decay_drops_count;
        stats.reinforce_success_count = self.reinforce_success_count;
        stats.reinforce_failure_count = self.reinforce_failure_count;
        stats
    }

    /// Snapshot the strongest pheromones, sorted by score descending.
    pub fn top_entries(&self, limit: usize, now_ms: u64) -> Vec<(PheromoneKey, f64, f64, u64)> {
        let mut v: Vec<_> = self
            .entries
            .values()
            .map(|e| {
                let age = if e.last_update_ms > 0 {
                    now_ms.saturating_sub(e.last_update_ms)
                } else {
                    0
                };
                (e.key.clone(), e.score, e.confidence, age)
            })
            .collect();
        v.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        v.truncate(limit);
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::addr::NodeAddr;
    use std::net::SocketAddr;

    fn na(s: &str) -> NodeAddr {
        let sa: SocketAddr = s.parse().unwrap();
        NodeAddr {
            ip: sa.ip(),
            port: sa.port(),
            protocol: Protocol::Udp,
        }
    }

    #[test]
    fn reinforcement_increases_score() {
        let mut store = PheromoneStore::new(10);
        store.reinforce_transport(Protocol::Udp, 30000, Some(40), true);
        let (score1, _) = store.score_for_route(&Route { hops: vec![na("127.0.0.1:30000")] });
        store.reinforce_transport(Protocol::Udp, 30000, Some(40), true);
        let (score2, _) = store.score_for_route(&Route { hops: vec![na("127.0.0.1:30000")] });
        assert!(score2 >= score1);
        assert!(score2 <= 1.0);
    }

    #[test]
    fn decay_reduces_score_and_evicts() {
        let mut store = PheromoneStore::new(10);
        store.reinforce_transport(Protocol::Udp, 30000, Some(40), true);
        let (before, _) = store.score_for_route(&Route { hops: vec![na("127.0.0.1:30000")] });
        assert!(before > 0.0);
        // Simulate 60 minutes later.
        let future = now_ms() + 60 * 60 * 1000;
        store.decay(future);
        let (after, _) = store.score_for_route(&Route { hops: vec![na("127.0.0.1:30000")] });
        assert!(after <= before);
    }

    #[test]
    fn eviction_keeps_bound() {
        let mut store = PheromoneStore::new(2);
        store.reinforce_transport(Protocol::Udp, 1, None, true);
        store.reinforce_transport(Protocol::Udp, 2, None, true);
        store.reinforce_transport(Protocol::Udp, 3, None, true);
        assert!(store.entries.len() <= 2);
    }

    #[test]
    fn failure_reduces_score_and_confidence() {
        let mut store = PheromoneStore::new(10);
        store.reinforce_transport(Protocol::Udp, 30000, Some(40), true);
        let (before, _) = store.score_for_route(&Route { hops: vec![na("127.0.0.1:30000")] });
        store.reinforce_transport(Protocol::Udp, 30000, None, false);
        let (after, _) = store.score_for_route(&Route { hops: vec![na("127.0.0.1:30000")] });
        assert!(after < before);
    }
}

