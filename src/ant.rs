use crate::addr::NodeAddr;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub const MAX_TTL: u8 = 32;
pub const MAX_OBSERVATIONS: usize = 16;
pub const MAX_PATH: usize = 32;
pub const MAX_ANT_BYTES: usize = 2048;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AntType {
    Scout,
    Probe,
    Echo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub node: NodeAddr,
    pub rtt_ms: Option<u32>,
    pub success: Option<bool>,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ant {
    pub id: [u8; 16],
    pub ant_type: AntType,
    pub ttl: u8,
    pub path: Vec<NodeAddr>,
    pub observations: Vec<Observation>,
    pub created_at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AntDropReason {
    Invalid,
    TtlExpired,
    ObservationLimit,
    Duplicate,
    Oversize,
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

impl Ant {
    pub fn validate(&self) -> bool {
        if self.ttl == 0 || self.ttl > MAX_TTL {
            return false;
        }
        if self.path.len() > MAX_PATH {
            return false;
        }
        if self.observations.len() > MAX_OBSERVATIONS {
            return false;
        }
        true
    }

    pub fn size_ok(&self) -> bool {
        bincode::serialized_size(self).map(|n| n as usize).unwrap_or(usize::MAX) <= MAX_ANT_BYTES
    }

    pub fn step(&mut self, node: NodeAddr, obs: Observation) -> Result<(), AntDropReason> {
        if self.ttl == 0 {
            return Err(AntDropReason::TtlExpired);
        }
        if self.observations.len() >= MAX_OBSERVATIONS {
            return Err(AntDropReason::ObservationLimit);
        }
        self.ttl = self.ttl.saturating_sub(1);
        if self.path.len() < MAX_PATH {
            self.path.push(node);
        }
        self.observations.push(obs);
        Ok(())
    }
}

/// Small bounded dedup cache for ants (by id).
pub struct AntDedup {
    cap: usize,
    ttl: Duration,
    order: VecDeque<[u8; 16]>,
    seen: HashMap<[u8; 16], u64>,
}

impl AntDedup {
    pub fn new(cap: usize, ttl: Duration) -> Self {
        Self {
            cap: cap.max(16),
            ttl,
            order: VecDeque::new(),
            seen: HashMap::new(),
        }
    }

    pub fn check_and_mark(&mut self, id: [u8; 16]) -> bool {
        let now = now_ms();
        self.prune(now);
        if self.seen.contains_key(&id) {
            return false;
        }
        self.seen.insert(id, now);
        self.order.push_back(id);
        while self.order.len() > self.cap {
            if let Some(old) = self.order.pop_front() {
                self.seen.remove(&old);
            }
        }
        true
    }

    fn prune(&mut self, now_ms: u64) {
        let ttl_ms = self.ttl.as_millis() as u64;
        while let Some(front) = self.order.front().copied() {
            let Some(t) = self.seen.get(&front).copied() else {
                let _ = self.order.pop_front();
                continue;
            };
            if now_ms.saturating_sub(t) > ttl_ms {
                let _ = self.order.pop_front();
                self.seen.remove(&front);
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::addr::Protocol;
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
    fn ant_validate_limits() {
        let a = Ant {
            id: [0u8; 16],
            ant_type: AntType::Echo,
            ttl: 1,
            path: vec![na("127.0.0.1:1"); MAX_PATH + 1],
            observations: vec![],
            created_at_ms: now_ms(),
        };
        assert!(!a.validate());
    }

    #[test]
    fn dedup_rejects_duplicates() {
        let mut d = AntDedup::new(32, Duration::from_secs(60));
        let id = [7u8; 16];
        assert!(d.check_and_mark(id));
        assert!(!d.check_and_mark(id));
    }
}

