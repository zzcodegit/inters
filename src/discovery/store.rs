use crate::addr::Protocol;
use crate::discovery::types::{NodeAdvertisement, NodeId};
use crate::node_config::NodeRole;
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::debug;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[derive(Debug, Default, Clone)]
pub struct DiscoveryStats {
    pub advertisements_received: u64,
    pub advertisements_accepted: u64,
    pub advertisements_rejected_expired: u64,
    pub advertisements_rejected_invalid: u64,
    pub queries_sent: u64,
    pub queries_received: u64,
    pub responses_sent: u64,
    pub responses_received: u64,
    pub store_size: usize,
}

#[derive(Debug)]
pub struct DiscoveryStore {
    entries: HashMap<NodeId, NodeAdvertisement>,
    max_entries: usize,
    stats: DiscoveryStats,
}

impl DiscoveryStore {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            max_entries: max_entries.max(1),
            stats: DiscoveryStats::default(),
        }
    }

    pub fn insert(&mut self, adv: NodeAdvertisement) {
        let now = now_ms();
        self.stats.advertisements_received = self.stats.advertisements_received.saturating_add(1);

        if adv.ttl_ms == 0 || !adv.is_fresh(now) {
            self.stats.advertisements_rejected_expired =
                self.stats.advertisements_rejected_expired.saturating_add(1);
            debug!(
                event = "discovery_store_update",
                reason = "expired",
                role = ?adv.role,
                addr = %adv.addr,
                "rejected expired advertisement"
            );
            return;
        }

        if self.entries.len() >= self.max_entries && !self.entries.contains_key(&adv.node_id) {
            self.evict_one();
        }

        let id = adv.node_id;
        let existed = self.entries.insert(id, adv).is_some();
        if existed {
            debug!(
                event = "discovery_store_update",
                reason = "update",
                node_id = ?id,
                "updated advertisement"
            );
        } else {
            debug!(
                event = "discovery_store_update",
                reason = "insert",
                node_id = ?id,
                "inserted advertisement"
            );
        }
        self.stats.advertisements_accepted = self.stats.advertisements_accepted.saturating_add(1);
        self.stats.store_size = self.entries.len();
    }

    fn evict_one(&mut self) {
        if self.entries.is_empty() {
            return;
        }
        if let Some((old_id, _)) = self
            .entries
            .iter()
            .min_by_key(|(_, adv)| adv.expires_at_ms())
            .map(|(id, adv)| (*id, adv.expires_at_ms()))
        {
            self.entries.remove(&old_id);
            debug!(
                event = "discovery_store_purge",
                reason = "cap",
                node_id = ?old_id,
                "evicted oldest advertisement to enforce bound"
            );
        }
    }

    pub fn purge_expired(&mut self, now_ms: u64) {
        let mut removed = Vec::new();
        for (id, adv) in self.entries.iter() {
            if !adv.is_fresh(now_ms) {
                removed.push(*id);
            }
        }
        for id in removed {
            self.entries.remove(&id);
            debug!(
                event = "discovery_store_purge",
                reason = "expired",
                node_id = ?id,
                "purged expired advertisement"
            );
        }
        self.stats.store_size = self.entries.len();
    }

    pub fn get(&self, id: &NodeId) -> Option<&NodeAdvertisement> {
        self.entries.get(id)
    }

    pub fn all_known(&self) -> Vec<NodeAdvertisement> {
        self.entries.values().cloned().collect()
    }

    pub fn known_relays(&self) -> Vec<NodeAdvertisement> {
        self.entries
            .values()
            .filter(|a| a.role == NodeRole::Relay)
            .cloned()
            .collect()
    }

    pub fn known_exits(&self) -> Vec<NodeAdvertisement> {
        self.entries
            .values()
            .filter(|a| a.role == NodeRole::Exit)
            .cloned()
            .collect()
    }

    pub fn fresh_candidates(
        &self,
        now_ms: u64,
        protocol: Option<Protocol>,
        role: Option<NodeRole>,
        max: usize,
    ) -> Vec<NodeAdvertisement> {
        let mut v: Vec<_> = self
            .entries
            .values()
            .filter(|a| a.is_fresh(now_ms))
            .filter(|a| protocol.map_or(true, |p| a.addr.protocol == p))
            .filter(|a| role.map_or(true, |r| a.role == r))
            .cloned()
            .collect();
        v.sort_by_key(|a| std::cmp::Reverse(a.advertised_at_ms));
        v.truncate(max);
        v
    }

    pub fn stats(&self) -> DiscoveryStats {
        let mut s = self.stats.clone();
        s.store_size = self.entries.len();
        s
    }

    pub fn on_query_sent(&mut self) {
        self.stats.queries_sent = self.stats.queries_sent.saturating_add(1);
    }

    pub fn on_query_received(&mut self) {
        self.stats.queries_received = self.stats.queries_received.saturating_add(1);
    }

    pub fn on_response_sent(&mut self) {
        self.stats.responses_sent = self.stats.responses_sent.saturating_add(1);
    }

    pub fn on_response_received(&mut self) {
        self.stats.responses_received = self.stats.responses_received.saturating_add(1);
    }
}

