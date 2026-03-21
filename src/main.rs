use vpnnode::build_info::BuildInfo;
use vpnnode::config;
use vpnnode::node_config::{NodeConfig, NodeRole};
use vpnnode::ops;
use vpnnode::roles;

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(name = "vpnnode")]
#[command(about = "Minimal relay-first VPN dataplane prototype", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run client node
    Client(config::ClientConfigCli),
    /// Run relay node
    Relay(roles::relay::RelayArgs),
    /// Run exit node
    Exit(config::ExitConfigCli),
    /// Run local target HTTP service
    Target(config::TargetConfigCli),
    /// Run node from TOML config
    Run {
        /// Path to TOML config file
        #[arg(long)]
        config: PathBuf,
        /// Start in drain mode immediately (no new sessions)
        #[arg(long, default_value_t = false)]
        drain: bool,
    },
    /// Validate config and basic startup feasibility
    Check {
        /// Path to TOML config file
        #[arg(long)]
        config: PathBuf,
    },
    /// Healthcheck (strict output, no long-running server)
    Health {
        /// Path to TOML config file
        #[arg(long)]
        config: PathBuf,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let bi = BuildInfo::current();
    tracing::info!(build = %bi.short(), "startup");

    let cli = Cli::parse();

    match cli.command {
        Commands::Client(args) => roles::client::run_client(args).await?,
        Commands::Relay(args) => roles::relay::run_relay(args).await?,
        Commands::Exit(args) => roles::exit::run_exit(args).await?,
        Commands::Target(args) => roles::target::run_target(args).await?,
        Commands::Run { config, drain } => run_from_config(&config, drain).await?,
        Commands::Check { config } => check_config(&config).await?,
        Commands::Health { config } => match ops::health::health_check(&config).await {
            Ok((role, addr)) => {
                let role = match role {
                    NodeRole::Client => "client",
                    NodeRole::Relay => "relay",
                    NodeRole::Exit => "exit",
                };
                println!("HEALTH OK role={} addr={}", role, addr);
            }
            Err(e) => {
                println!("HEALTH FAIL reason={}", e);
                std::process::exit(1);
            }
        },
    }

    Ok(())
}

async fn run_from_config(path: &PathBuf, drain: bool) -> anyhow::Result<()> {
    let cfg = NodeConfig::load_from_file(path)?;
    tracing::info!(role = ?cfg.role, bind = %cfg.bind_addr(), peers = cfg.peers.len(), ants = cfg.ants_enabled, "loaded config");

    if cfg.ants_enabled {
        std::env::set_var("VPNNODE_ANTS", "1");
    }
    let timeout = std::time::Duration::from_secs(cfg.drain_timeout_sec);
    ops::signal::spawn_signal_listener(timeout);
    if drain {
        ops::drain::activate(timeout);
    }

    match cfg.role {
        NodeRole::Client => {
            // Map peers into existing CLI-style args without changing flags.
            let relay = cfg.peers.get(0).and_then(|n| n.as_socket_addr());
            let exit = cfg.peers.last().and_then(|n| n.as_socket_addr());
            let discovery_bootstrap_peers: Vec<_> = cfg
                .peers
                .iter()
                .filter_map(|n| n.as_socket_addr())
                .collect();
            let args = config::ClientConfigCli {
                local_listen: cfg.client_local_listen,
                mode: "tcp".to_string(),
                relay_addr: relay
                    .map(|a| a.to_string())
                    .unwrap_or_else(|| "127.0.0.1:30000".to_string()),
                exit_addr: exit
                    .map(|a| a.to_string())
                    .unwrap_or_else(|| "127.0.0.1:30001".to_string()),
                route_length: cfg.route_length,
                relay2_addr: cfg
                    .peers
                    .get(1)
                    .and_then(|n| n.as_socket_addr())
                    .map(|a| a.to_string()),
                exact_route_only: false,
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

                discovery_enabled: cfg.discovery_enabled,
                discovery_query_on_start: cfg.discovery_query_on_start,
                discovery_max_entries: cfg.discovery_max_entries,
                discovery_advertise_ttl_sec: cfg.discovery_advertise_ttl_sec,
                discovery_bootstrap_peers: if discovery_bootstrap_peers.is_empty() {
                    None
                } else {
                    Some(discovery_bootstrap_peers)
                },
            };
            roles::client::run_client(args).await
        }
        NodeRole::Relay => {
            let exit = cfg.peers[0]
                .as_socket_addr()
                .ok_or_else(|| anyhow::anyhow!("relay peer must be UDP socket addr"))?;
            let discovery_bootstrap_peers: Vec<_> = cfg
                .peers
                .iter()
                .filter_map(|n| n.as_socket_addr())
                .collect();
            let args = config::RelayConfigCli {
                listen: cfg.bind_addr(),
                exit_addr: exit.to_string(),
                relay_key_path: "relay.key".to_string(),

                discovery_enabled: cfg.discovery_enabled,
                discovery_query_on_start: cfg.discovery_query_on_start,
                discovery_max_entries: cfg.discovery_max_entries,
                discovery_advertise_ttl_sec: cfg.discovery_advertise_ttl_sec,
                discovery_bootstrap_peers: if discovery_bootstrap_peers.is_empty() {
                    None
                } else {
                    Some(discovery_bootstrap_peers)
                },
            };
            roles::relay::run_relay(args).await
        }
        NodeRole::Exit => {
            let discovery_bootstrap_peers: Vec<_> = cfg
                .peers
                .iter()
                .filter_map(|n| n.as_socket_addr())
                .collect();
            let args = config::ExitConfigCli {
                listen: cfg.bind_addr(),
                target_addr: cfg.exit_target_addr,
                exit_key_path: "exit.key".to_string(),
                response_window_frames: cfg.exit_response_window_frames,

                discovery_enabled: cfg.discovery_enabled,
                discovery_query_on_start: cfg.discovery_query_on_start,
                discovery_max_entries: cfg.discovery_max_entries,
                discovery_advertise_ttl_sec: cfg.discovery_advertise_ttl_sec,
                discovery_bootstrap_peers: if discovery_bootstrap_peers.is_empty() {
                    None
                } else {
                    Some(discovery_bootstrap_peers)
                },
            };
            roles::exit::run_exit(args).await
        }
    }
}

async fn check_config(path: &PathBuf) -> anyhow::Result<()> {
    let cfg = NodeConfig::load_from_file(path)?;
    // Fail-fast feasibility: try binding the role UDP socket (and client TCP listener) and immediately release.
    match cfg.role {
        NodeRole::Client => {
            let _tcp = tokio::net::TcpListener::bind(cfg.client_local_listen).await?;
            let _udp = tokio::net::UdpSocket::bind(cfg.bind_addr()).await?;
        }
        NodeRole::Relay | NodeRole::Exit => {
            let _udp = tokio::net::UdpSocket::bind(cfg.bind_addr()).await?;
        }
    }
    tracing::info!(role = ?cfg.role, bind = %cfg.bind_addr(), "node ready (self-check ok)");
    Ok(())
}
