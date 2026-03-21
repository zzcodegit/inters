use crate::addr::{NodeAddr, Protocol};
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, SocketAddr};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NodeRole {
    Client,
    Relay,
    Exit,
}

/// Unified node configuration for manual multi-node deployments.
///
/// Keep it explicit and small: this config is meant for operators.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub role: NodeRole,
    pub bind_ip: IpAddr,
    pub bind_port: u16,

    /// Optional route seed / bootstrap peers.
    /// For manual setups this can be the ordered hop list (relay(s) then exit).
    #[serde(default)]
    pub peers: Vec<NodeAddr>,

    /// Optional operational toggles.
    #[serde(default)]
    pub ants_enabled: bool,

    /// Ops: drain timeout for graceful shutdown.
    #[serde(default = "default_drain_timeout_sec")]
    pub drain_timeout_sec: u64,

    /// Ops: allow healthcheck command for this node (foundation only).
    #[serde(default = "default_health_enabled")]
    pub health_enabled: bool,

    /// Client-only: local TCP listener.
    #[serde(default = "default_client_local_listen")]
    pub client_local_listen: SocketAddr,
    /// Client-only: route length (1..=16).
    #[serde(default = "default_route_length")]
    pub route_length: u8,

    /// Exit-only: target service address (TCP).
    #[serde(default = "default_exit_target_addr")]
    pub exit_target_addr: String,
    /// Exit-only: max in-flight response DATA frames per stream.
    #[serde(default = "default_exit_response_window_frames")]
    pub exit_response_window_frames: usize,
    /// Exit-only: allowlist for target hosts (foundation only; not enforced yet).
    #[serde(default)]
    pub exit_target_allowlist: Option<Vec<String>>,

    /// Stage 9.1: minimal discovery (control-plane only).
    #[serde(default = "default_discovery_enabled")]
    pub discovery_enabled: bool,

    /// Stage 9.1: optionally send a discovery query on startup.
    #[serde(default = "default_discovery_query_on_start")]
    pub discovery_query_on_start: bool,

    /// Stage 9.1: bounded local cache size for node advertisements.
    #[serde(default = "default_discovery_max_entries")]
    pub discovery_max_entries: usize,

    /// Stage 9.1: freshness TTL (seconds) for advertisements we accept/store.
    #[serde(default = "default_discovery_advertise_ttl_sec")]
    pub discovery_advertise_ttl_sec: u64,
}

fn default_client_local_listen() -> SocketAddr {
    "127.0.0.1:10080".parse().expect("default listen")
}
fn default_route_length() -> u8 {
    2
}
fn default_exit_target_addr() -> String {
    "127.0.0.1:8080".to_string()
}
fn default_exit_response_window_frames() -> usize {
    64
}
fn default_drain_timeout_sec() -> u64 {
    20
}
fn default_health_enabled() -> bool {
    true
}

fn default_discovery_enabled() -> bool {
    false
}
fn default_discovery_query_on_start() -> bool {
    false
}
fn default_discovery_max_entries() -> usize {
    256
}
fn default_discovery_advertise_ttl_sec() -> u64 {
    300
}

impl NodeConfig {
    pub fn bind_addr(&self) -> SocketAddr {
        SocketAddr::new(self.bind_ip, self.bind_port)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        if self.bind_port == 0 {
            anyhow::bail!("bind_port must be non-zero");
        }
        if !(1..=16).contains(&self.route_length) {
            anyhow::bail!("route_length must be in 1..=16");
        }
        if self.drain_timeout_sec == 0 || self.drain_timeout_sec > 600 {
            anyhow::bail!("drain_timeout_sec must be in 1..=600");
        }
        if self.discovery_max_entries == 0 || self.discovery_max_entries > 4096 {
            anyhow::bail!("discovery_max_entries must be in 1..=4096");
        }
        if self.discovery_advertise_ttl_sec == 0 || self.discovery_advertise_ttl_sec > 3600 {
            anyhow::bail!("discovery_advertise_ttl_sec must be in 1..=3600");
        }
        match self.role {
            NodeRole::Relay => {
                if self.peers.is_empty() {
                    anyhow::bail!("relay requires at least one peer (exit addr) in peers[]");
                }
                if self.peers.iter().any(|p| p.protocol != Protocol::Udp) {
                    anyhow::bail!("relay peers must be UDP for this stage");
                }
            }
            NodeRole::Exit => {
                // exit_target_addr must parse/resolve later; keep basic non-empty validation here
                if self.exit_target_addr.trim().is_empty() {
                    anyhow::bail!("exit_target_addr must be non-empty");
                }
                if !(8..=256).contains(&self.exit_response_window_frames) {
                    anyhow::bail!("exit_response_window_frames must be in 8..=256");
                }
            }
            NodeRole::Client => {
                if self.route_length >= 2 && self.peers.is_empty() {
                    anyhow::bail!("client route_length>=2 requires peers[] (relay/exit seed)");
                }
            }
        }
        Ok(())
    }

    pub fn load_from_file(path: &Path) -> anyhow::Result<Self> {
        let raw = std::fs::read_to_string(path)?;
        let cfg: NodeConfig = toml::from_str(&raw)?;
        cfg.validate()?;
        Ok(cfg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_basic_client_toml() {
        let s = r#"
role = "client"
bind_ip = "0.0.0.0"
bind_port = 0
"#;
        let cfg: NodeConfig = toml::from_str(s).unwrap();
        assert_eq!(cfg.role, NodeRole::Client);
        assert_eq!(cfg.bind_ip, "0.0.0.0".parse::<IpAddr>().unwrap());
        assert_eq!(cfg.client_local_listen, default_client_local_listen());
    }

    #[test]
    fn invalid_bind_port_fails_validation() {
        let cfg = NodeConfig {
            role: NodeRole::Relay,
            bind_ip: "127.0.0.1".parse().unwrap(),
            bind_port: 0,
            peers: vec![],
            ants_enabled: false,
            drain_timeout_sec: default_drain_timeout_sec(),
            health_enabled: default_health_enabled(),
            client_local_listen: default_client_local_listen(),
            route_length: 2,
            exit_target_addr: default_exit_target_addr(),
            exit_response_window_frames: default_exit_response_window_frames(),
            exit_target_allowlist: None,
            discovery_enabled: false,
            discovery_query_on_start: false,
            discovery_max_entries: default_discovery_max_entries(),
            discovery_advertise_ttl_sec: default_discovery_advertise_ttl_sec(),
        };
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn invalid_drain_timeout_fails_validation() {
        let mut cfg: NodeConfig = toml::from_str(
            r#"
role = "relay"
bind_ip = "127.0.0.1"
bind_port = 30000
drain_timeout_sec = 0
peers = [{ ip = "127.0.0.1", port = 30001, protocol = "Udp" }]
"#,
        )
        .unwrap();
        cfg.drain_timeout_sec = 0;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn invalid_discovery_settings_fail_validation() {
        let mut cfg: NodeConfig = toml::from_str(
            r#"
role = "relay"
bind_ip = "127.0.0.1"
bind_port = 30000
peers = [{ ip = "127.0.0.1", port = 30001, protocol = "Udp" }]
discovery_max_entries = 0
discovery_advertise_ttl_sec = 0
"#,
        )
        .unwrap();
        assert!(cfg.validate().is_err());

        cfg.discovery_max_entries = 1;
        cfg.discovery_advertise_ttl_sec = 4000;
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn invalid_exit_response_window_fails_validation() {
        let cfg: NodeConfig = toml::from_str(
            r#"
role = "exit"
bind_ip = "127.0.0.1"
bind_port = 30001
exit_target_addr = "127.0.0.1:8080"
exit_response_window_frames = 4
"#,
        )
        .unwrap();
        assert!(cfg.validate().is_err());
    }
}
