use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteEntry {
    pub relay_addr: String,
    pub exit_addr: String,
    pub transport: String,
    pub success_count: u64,
    pub failure_count: u64,
    pub avg_latency_ms: f64,
    pub last_success: SystemTime,
    pub ttl: Duration,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RouteCache {
    entries: HashMap<String, RouteEntry>,
}

impl RouteCache {
    pub fn load(path: &str) -> anyhow::Result<Self> {
        if let Ok(data) = fs::read(path) {
            let cache: RouteCache = serde_json::from_slice(&data)?;
            Ok(cache)
        } else {
            Ok(RouteCache::default())
        }
    }

    pub fn save(&self, path: &str) -> anyhow::Result<()> {
        let data = serde_json::to_vec_pretty(self)?;
        fs::write(path, data)?;
        Ok(())
    }

    pub fn key_for(&self, relay_addr: &str, exit_addr: &str, transport: &str) -> String {
        format!("{relay_addr}|{exit_addr}|{transport}")
    }

    pub fn record_success(
        &mut self,
        relay_addr: &str,
        exit_addr: &str,
        transport: &str,
        latency: Duration,
        ttl: Duration,
    ) {
        let key = self.key_for(relay_addr, exit_addr, transport);
        let now = SystemTime::now();
        let latency_ms = latency.as_secs_f64() * 1000.0;
        self.entries
            .entry(key)
            .and_modify(|e| {
                e.success_count += 1;
                e.last_success = now;
                e.avg_latency_ms =
                    (e.avg_latency_ms * ((e.success_count - 1) as f64) + latency_ms)
                        / (e.success_count as f64);
                e.ttl = ttl;
            })
            .or_insert(RouteEntry {
                relay_addr: relay_addr.to_string(),
                exit_addr: exit_addr.to_string(),
                transport: transport.to_string(),
                success_count: 1,
                failure_count: 0,
                avg_latency_ms: latency_ms,
                last_success: now,
                ttl,
            });
    }

    pub fn record_failure(&mut self, relay_addr: &str, exit_addr: &str, transport: &str) {
        let key = self.key_for(relay_addr, exit_addr, transport);
        self.entries
            .entry(key)
            .and_modify(|e| e.failure_count += 1)
            .or_insert(RouteEntry {
                relay_addr: relay_addr.to_string(),
                exit_addr: exit_addr.to_string(),
                transport: transport.to_string(),
                success_count: 0,
                failure_count: 1,
                avg_latency_ms: 0.0,
                last_success: SystemTime::now(),
                ttl: Duration::from_secs(60),
            });
    }

    pub fn lookup_hint(
        &mut self,
        relay_addr: &str,
        exit_addr: &str,
        transport: &str,
    ) -> Option<RouteEntry> {
        let key = self.key_for(relay_addr, exit_addr, transport);
        let now = SystemTime::now();
        if let Some(e) = self.entries.get(&key) {
            if now
                .duration_since(e.last_success)
                .unwrap_or_default()
                <= e.ttl
            {
                return Some(e.clone());
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[test]
    fn route_cache_basic() {
        let mut cache = RouteCache::default();
        let ttl = Duration::from_secs(60);
        cache.record_success("relay", "exit", "udp", Duration::from_millis(10), ttl);
        let hint = cache.lookup_hint("relay", "exit", "udp");
        assert!(hint.is_some());
        let e = hint.unwrap();
        assert_eq!(e.success_count, 1);

        cache.record_failure("relay", "exit", "udp");
        let hint2 = cache.lookup_hint("relay", "exit", "udp").unwrap();
        assert_eq!(hint2.failure_count, 1);
    }

    #[test]
    fn route_cache_ttl_decay() {
        let mut cache = RouteCache::default();
        let ttl = Duration::from_millis(50);
        cache.record_success("relay", "exit", "udp", Duration::from_millis(10), ttl);
        assert!(cache.lookup_hint("relay", "exit", "udp").is_some());
        sleep(Duration::from_millis(80));
        let expired = cache.lookup_hint("relay", "exit", "udp");
        assert!(expired.is_none());
    }
}

