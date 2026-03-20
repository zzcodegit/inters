use std::net::SocketAddr;

use clap::Args;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommonConfig {
    pub transport: String,
    pub route_cache_path: Option<String>,
}

#[derive(Debug, Clone, Args)]
pub struct ClientConfigCli {
    /// Local TCP listener address for client (tcp-listener mode)
    #[arg(long, default_value = "127.0.0.1:10080")]
    pub local_listen: SocketAddr,
    /// Client mode: "tcp" (stable) or "tun" (experimental, Unix-only).
    /// On non-Unix platforms, "tun" is not supported and will fail fast with
    /// a clear error message; use "tcp" on Windows.
    #[arg(long, default_value = "tcp")]
    pub mode: String,
    /// Relay UDP address (may be host:port or ip:port). First hop for route-length 2+.
    #[arg(long, default_value = "127.0.0.1:30000")]
    pub relay_addr: String,
    /// Exit UDP address. For route-length 1 (direct) or full route display. Default 127.0.0.1:30001
    #[arg(long, default_value = "127.0.0.1:30001")]
    pub exit_addr: String,
    /// Stage 5: route length. 1=client->exit, 2=client->relay->exit, 3=client->relay->relay2->exit
    #[arg(long, default_value = "2")]
    pub route_length: u8,
    /// Stage 5: second relay (for route-length 3). E.g. 127.0.0.1:30002
    #[arg(long)]
    pub relay2_addr: Option<String>,
    /// Path to client static private key (x25519)
    #[arg(long, default_value = "client.key")]
    pub client_key_path: String,
    /// Path to relay static public key (x25519)
    #[arg(long, default_value = "relay.pub")]
    pub relay_pubkey_path: String,
    /// Route cache path
    #[arg(long, default_value = "route_cache.json")]
    pub route_cache_path: String,
    /// TUN interface name (tun mode)
    #[arg(long, default_value = "tun-vpnnode0")]
    pub tun_name: String,
    /// TUN IPv4 address (tun mode)
    #[arg(long, default_value = "10.10.0.1")]
    pub tun_address: String,
    /// TUN netmask (tun mode)
    #[arg(long, default_value = "255.255.255.0")]
    pub tun_netmask: String,
    /// TUN MTU (tun mode)
    #[arg(long, default_value_t = 1500)]
    pub tun_mtu: i32,
    /// Max in-flight DATA frames per stream before sender applies backpressure.
    #[arg(long, default_value_t = 64)]
    pub max_inflight_frames: usize,
    /// Base retransmit interval in milliseconds for unacked frames.
    #[arg(long, default_value_t = 200)]
    pub retransmit_interval: u64,
    /// Max application payload chunk size per DATA frame (before tunnel encapsulation).
    /// Helps keep UDP packets below MTU when combined with headers and AEAD overhead.
    #[arg(long, default_value_t = 1000)]
    pub chunk_size: usize,

    /// Stage 9.1: enable minimal discovery (control-plane only).
    #[arg(long, default_value_t = false)]
    pub discovery_enabled: bool,
    /// Stage 9.1: optionally send a discovery query on startup.
    #[arg(long, default_value_t = false)]
    pub discovery_query_on_start: bool,
    /// Stage 9.1: bounded local cache size for advertisements.
    #[arg(long, default_value_t = 256)]
    pub discovery_max_entries: usize,
    /// Stage 9.1: advertisement TTL in seconds.
    #[arg(long, default_value_t = 300)]
    pub discovery_advertise_ttl_sec: u64,
    /// Stage 9.1: optional explicit discovery bootstrap UDP peers.
    /// If not provided, roles derive candidates conservatively from configured route seeds.
    #[arg(long, value_delimiter = ',')]
    pub discovery_bootstrap_peers: Option<Vec<SocketAddr>>,
}

#[derive(Debug, Clone, Args)]
pub struct RelayConfigCli {
    /// Relay UDP listen address
    #[arg(long, default_value = "0.0.0.0:30000")]
    pub listen: SocketAddr,
    /// Exit UDP address (may be host:port or ip:port)
    #[arg(long, default_value = "127.0.0.1:30001")]
    pub exit_addr: String,
    /// Relay static private key path
    #[arg(long, default_value = "relay.key")]
    pub relay_key_path: String,

    /// Stage 9.1: enable minimal discovery (control-plane only).
    #[arg(long, default_value_t = false)]
    pub discovery_enabled: bool,
    #[arg(long, default_value_t = false)]
    pub discovery_query_on_start: bool,
    #[arg(long, default_value_t = 256)]
    pub discovery_max_entries: usize,
    #[arg(long, default_value_t = 300)]
    pub discovery_advertise_ttl_sec: u64,
    #[arg(long, value_delimiter = ',')]
    pub discovery_bootstrap_peers: Option<Vec<SocketAddr>>,
}

#[derive(Debug, Clone, Args)]
pub struct ExitConfigCli {
    /// Exit UDP listen address
    #[arg(long, default_value = "0.0.0.0:30001")]
    pub listen: SocketAddr,
    /// Target TCP service address (may be host:port or ip:port)
    #[arg(long, default_value = "127.0.0.1:8080")]
    pub target_addr: String,
    /// Exit static private key path
    #[arg(long, default_value = "exit.key")]
    pub exit_key_path: String,

    /// Stage 9.1: enable minimal discovery (control-plane only).
    #[arg(long, default_value_t = false)]
    pub discovery_enabled: bool,
    #[arg(long, default_value_t = false)]
    pub discovery_query_on_start: bool,
    #[arg(long, default_value_t = 256)]
    pub discovery_max_entries: usize,
    #[arg(long, default_value_t = 300)]
    pub discovery_advertise_ttl_sec: u64,
    #[arg(long, value_delimiter = ',')]
    pub discovery_bootstrap_peers: Option<Vec<SocketAddr>>,
}

impl Default for ClientConfigCli {
    fn default() -> Self {
        Self {
            local_listen: "127.0.0.1:10080".parse().expect("default local_listen"),
            mode: "tcp".to_string(),
            relay_addr: "127.0.0.1:30000".to_string(),
            exit_addr: "127.0.0.1:30001".to_string(),
            route_length: 2,
            relay2_addr: None,
            client_key_path: "client.key".to_string(),
            relay_pubkey_path: "relay.pub".to_string(),
            route_cache_path: "route_cache.json".to_string(),
            tun_name: "tun-vpnnode0".to_string(),
            tun_address: "10.10.0.1".to_string(),
            tun_netmask: "255.255.255.0".to_string(),
            tun_mtu: 1500,
            max_inflight_frames: 64,
            retransmit_interval: 200,
            chunk_size: 1000,
            discovery_enabled: false,
            discovery_query_on_start: false,
            discovery_max_entries: 256,
            discovery_advertise_ttl_sec: 300,
            discovery_bootstrap_peers: None,
        }
    }
}

impl Default for RelayConfigCli {
    fn default() -> Self {
        Self {
            listen: "0.0.0.0:30000".parse().expect("default relay listen"),
            exit_addr: "127.0.0.1:30001".to_string(),
            relay_key_path: "relay.key".to_string(),
            discovery_enabled: false,
            discovery_query_on_start: false,
            discovery_max_entries: 256,
            discovery_advertise_ttl_sec: 300,
            discovery_bootstrap_peers: None,
        }
    }
}

impl Default for ExitConfigCli {
    fn default() -> Self {
        Self {
            listen: "0.0.0.0:30001".parse().expect("default exit listen"),
            target_addr: "127.0.0.1:8080".to_string(),
            exit_key_path: "exit.key".to_string(),
            discovery_enabled: false,
            discovery_query_on_start: false,
            discovery_max_entries: 256,
            discovery_advertise_ttl_sec: 300,
            discovery_bootstrap_peers: None,
        }
    }
}

#[derive(Debug, Clone, Args)]
pub struct TargetConfigCli {
    /// HTTP target listen address
    #[arg(long, default_value = "127.0.0.1:8080")]
    pub listen: SocketAddr,
}

