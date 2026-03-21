#![allow(dead_code)]

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::Once;

use anyhow::{bail, Context, Result};
use tracing_subscriber::EnvFilter;
use vpnnode::config::{ClientConfigCli, ExitConfigCli, RelayConfigCli, TargetConfigCli};

static INIT_TRACING: Once = Once::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaselineMode {
    Local,
    Remote,
}

impl BaselineMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Remote => "remote",
        }
    }
}

#[derive(Debug, Clone)]
pub struct LocalBaselineSpec<'a> {
    pub target_addr: Option<SocketAddr>,
    pub exit_udp: SocketAddr,
    pub relay1_udp: Option<SocketAddr>,
    pub relay2_udp: Option<SocketAddr>,
    pub client_tcp: SocketAddr,
    pub route_length: u8,
    pub exit_target_addr: String,
    pub route_cache_path: &'a str,
    pub tun_name: &'a str,
}

#[derive(Debug, Clone)]
pub struct RemoteBaselineConfig {
    pub local_listen: SocketAddr,
    pub relay1_addr: String,
    pub relay2_addr: Option<String>,
    pub exit_addr: String,
    pub route_length: u8,
    pub exact_route_only: bool,
    pub route_cache_path: String,
    pub target_scheme: String,
    pub http_host: String,
    pub expected_ready_status: u16,
    pub ready_timeout_secs: u64,
    pub probe_attempt_timeout_secs: u64,
}

pub fn init_tracing() {
    INIT_TRACING.call_once(|| {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .try_init();
    });
}

pub fn prepare_baseline(mode: BaselineMode) {
    init_tracing();
    std::env::set_var("VPNNODE_ANTS", "0");

    match std::env::var("VPNNODE_BASELINE_MODE") {
        Ok(actual) => assert_eq!(
            actual,
            mode.as_str(),
            "expected VPNNODE_BASELINE_MODE={}, got {}",
            mode.as_str(),
            actual
        ),
        Err(_) => {
            std::env::set_var("VPNNODE_BASELINE_MODE", mode.as_str());
        }
    }
}

pub fn http_get_request(host: &str) -> Vec<u8> {
    http_get_request_path(host, "/")
}

pub fn http_get_request_path(host: &str, path: &str) -> Vec<u8> {
    format!("GET {path} HTTP/1.1\r\nHost: {host}\r\nConnection: close\r\n\r\n").into_bytes()
}

pub fn spawn_local_stack(spec: &LocalBaselineSpec<'_>) {
    if let Some(target_addr) = spec.target_addr {
        tokio::spawn(async move {
            let _ = vpnnode::roles::target::run_target(TargetConfigCli {
                listen: target_addr,
            })
            .await;
        });
    }

    let exit_udp = spec.exit_udp;
    let exit_target_addr = spec.exit_target_addr.clone();
    tokio::spawn(async move {
        let _ = vpnnode::roles::exit::run_exit(ExitConfigCli {
            listen: exit_udp,
            target_addr: exit_target_addr,
            exit_key_path: "exit.key".to_string(),
            ..Default::default()
        })
        .await;
    });

    if let Some(relay2_udp) = spec.relay2_udp {
        let downstream = spec.exit_udp.to_string();
        tokio::spawn(async move {
            let _ = vpnnode::roles::relay::run_relay(RelayConfigCli {
                listen: relay2_udp,
                exit_addr: downstream,
                relay_key_path: "relay.key".to_string(),
                ..Default::default()
            })
            .await;
        });
    }

    if let Some(relay1_udp) = spec.relay1_udp {
        let downstream = spec.relay2_udp.unwrap_or(spec.exit_udp).to_string();
        tokio::spawn(async move {
            let _ = vpnnode::roles::relay::run_relay(RelayConfigCli {
                listen: relay1_udp,
                exit_addr: downstream,
                relay_key_path: "relay.key".to_string(),
                ..Default::default()
            })
            .await;
        });
    }

    let client_args = local_client_args(spec);
    tokio::spawn(async move {
        let _ = vpnnode::roles::client::run_client(client_args).await;
    });
}

pub fn local_client_args(spec: &LocalBaselineSpec<'_>) -> ClientConfigCli {
    let relay_addr = if spec.route_length == 1 {
        "127.0.0.1:1".to_string()
    } else {
        spec.relay1_udp
            .expect("relay1_udp must be set for route_length >= 2")
            .to_string()
    };
    let relay2_addr = if spec.route_length >= 3 {
        Some(
            spec.relay2_udp
                .expect("relay2_udp must be set for route_length >= 3")
                .to_string(),
        )
    } else {
        None
    };

    ClientConfigCli {
        local_listen: spec.client_tcp,
        mode: "tcp".to_string(),
        relay_addr,
        exit_addr: spec.exit_udp.to_string(),
        route_length: spec.route_length,
        relay2_addr,
        client_key_path: "client.key".to_string(),
        relay_pubkey_path: "relay.pub".to_string(),
        route_cache_path: spec.route_cache_path.to_string(),
        tun_name: spec.tun_name.to_string(),
        tun_address: "10.10.0.1".to_string(),
        tun_netmask: "255.255.255.0".to_string(),
        tun_mtu: 1500,
        max_inflight_frames: 64,
        retransmit_interval: 200,
        chunk_size: 1000,
        ..Default::default()
    }
}

pub struct ManagedRemoteClient {
    child: Option<Child>,
}

impl ManagedRemoteClient {
    pub fn stop(&mut self) {
        let Some(mut child) = self.child.take() else {
            return;
        };
        let _ = child.kill();
        let _ = child.wait();
    }
}

impl Drop for ManagedRemoteClient {
    fn drop(&mut self) {
        self.stop();
    }
}

pub fn spawn_remote_client(config: &RemoteBaselineConfig) -> Result<ManagedRemoteClient> {
    spawn_remote_client_process(config)
}

pub fn spawn_remote_client_process(config: &RemoteBaselineConfig) -> Result<ManagedRemoteClient> {
    let client_args = config.client_args();
    let binary = vpnnode_binary_path();
    let cli_args = client_cli_args(&client_args);
    let child = Command::new(&binary)
        .args(&cli_args)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .with_context(|| {
            format!(
                "failed to spawn remote client process {} {}",
                binary.display(),
                cli_args.join(" ")
            )
        })?;
    Ok(ManagedRemoteClient { child: Some(child) })
}

impl RemoteBaselineConfig {
    pub fn from_env() -> Result<Self> {
        let route_length = parse_env_or_default::<u8>("VPNNODE_BASELINE_REMOTE_ROUTE_LENGTH", 3)?;
        if !(1..=3).contains(&route_length) {
            bail!("VPNNODE_BASELINE_REMOTE_ROUTE_LENGTH must be 1, 2, or 3; got {route_length}");
        }

        let exit_addr = required_env("VPNNODE_BASELINE_REMOTE_EXIT_ADDR")?;
        reject_loopback("VPNNODE_BASELINE_REMOTE_EXIT_ADDR", &exit_addr)?;

        let relay1_addr = if route_length >= 2 {
            let value = required_env("VPNNODE_BASELINE_REMOTE_RELAY1_ADDR")?;
            reject_loopback("VPNNODE_BASELINE_REMOTE_RELAY1_ADDR", &value)?;
            value
        } else {
            "127.0.0.1:1".to_string()
        };

        let relay2_addr = if route_length >= 3 {
            let value = required_env("VPNNODE_BASELINE_REMOTE_RELAY2_ADDR")?;
            reject_loopback("VPNNODE_BASELINE_REMOTE_RELAY2_ADDR", &value)?;
            Some(value)
        } else {
            None
        };

        Ok(Self {
            local_listen: parse_env_or_default(
                "VPNNODE_BASELINE_REMOTE_LOCAL_LISTEN",
                "127.0.0.1:19080"
                    .parse()
                    .expect("default remote local listen"),
            )?,
            relay1_addr,
            relay2_addr,
            exit_addr,
            route_length,
            exact_route_only: parse_env_or_default(
                "VPNNODE_BASELINE_REMOTE_EXACT_ROUTE_ONLY",
                true,
            )?,
            route_cache_path: env_or_default(
                "VPNNODE_BASELINE_REMOTE_ROUTE_CACHE_PATH",
                "route_cache_remote_baseline.json",
            ),
            target_scheme: parse_target_scheme()?,
            http_host: env_or_default("VPNNODE_BASELINE_REMOTE_HTTP_HOST", "example"),
            expected_ready_status: parse_env_or_default(
                "VPNNODE_BASELINE_REMOTE_EXPECT_READY_STATUS",
                200_u16,
            )?,
            ready_timeout_secs: parse_env_or_default(
                "VPNNODE_BASELINE_REMOTE_READY_TIMEOUT_SECS",
                90_u64,
            )?,
            probe_attempt_timeout_secs: parse_env_or_default(
                "VPNNODE_BASELINE_REMOTE_PROBE_ATTEMPT_TIMEOUT_SECS",
                45_u64,
            )?,
        })
    }

    pub fn for_scenario(
        &self,
        route_length: u8,
        local_listen: SocketAddr,
        route_cache_path: impl Into<String>,
        http_host: impl Into<String>,
    ) -> Result<Self> {
        if !(1..=3).contains(&route_length) {
            bail!("remote scenario route length must be 1, 2, or 3; got {route_length}");
        }
        if route_length >= 2 && self.relay1_addr == "127.0.0.1:1" {
            bail!("relay1 is required for remote route length {route_length}");
        }
        if route_length >= 3 && self.relay2_addr.is_none() {
            bail!("relay2 is required for remote route length 3");
        }

        Ok(Self {
            local_listen,
            relay1_addr: if route_length >= 2 {
                self.relay1_addr.clone()
            } else {
                "127.0.0.1:1".to_string()
            },
            relay2_addr: if route_length >= 3 {
                self.relay2_addr.clone()
            } else {
                None
            },
            exit_addr: self.exit_addr.clone(),
            route_length,
            exact_route_only: self.exact_route_only,
            route_cache_path: route_cache_path.into(),
            target_scheme: self.target_scheme.clone(),
            http_host: http_host.into(),
            expected_ready_status: self.expected_ready_status,
            ready_timeout_secs: self.ready_timeout_secs,
            probe_attempt_timeout_secs: self.probe_attempt_timeout_secs,
        })
    }

    pub fn client_args(&self) -> ClientConfigCli {
        ClientConfigCli {
            local_listen: self.local_listen,
            mode: "tcp".to_string(),
            relay_addr: if self.route_length == 1 {
                "127.0.0.1:1".to_string()
            } else {
                self.relay1_addr.clone()
            },
            exit_addr: self.exit_addr.clone(),
            route_length: self.route_length,
            relay2_addr: if self.route_length >= 3 {
                self.relay2_addr.clone()
            } else {
                None
            },
            exact_route_only: self.exact_route_only,
            client_key_path: "client.key".to_string(),
            relay_pubkey_path: "relay.pub".to_string(),
            route_cache_path: self.route_cache_path.clone(),
            tun_name: "tun-baseline-remote".to_string(),
            tun_address: "10.10.0.1".to_string(),
            tun_netmask: "255.255.255.0".to_string(),
            tun_mtu: 1500,
            max_inflight_frames: 64,
            retransmit_interval: 200,
            chunk_size: 1000,
            ..Default::default()
        }
    }

    pub fn route_chain(&self) -> String {
        let mut hops = vec!["client".to_string()];
        if self.route_length >= 2 {
            hops.push(self.relay1_addr.clone());
        }
        if let Some(relay2_addr) = &self.relay2_addr {
            hops.push(relay2_addr.clone());
        }
        hops.push(self.exit_addr.clone());
        hops.push("target".to_string());
        hops.join(" -> ")
    }
}

fn vpnnode_binary_path() -> PathBuf {
    if let Ok(path) = std::env::var("CARGO_BIN_EXE_vpnnode") {
        return PathBuf::from(path);
    }
    if let Some(path) = option_env!("CARGO_BIN_EXE_vpnnode") {
        return PathBuf::from(path);
    }

    let exe_name = if cfg!(windows) {
        "target\\debug\\vpnnode.exe"
    } else {
        "target/debug/vpnnode"
    };
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(exe_name)
}

fn client_cli_args(args: &ClientConfigCli) -> Vec<String> {
    let mut cli_args = vec![
        "client".to_string(),
        "--local-listen".to_string(),
        args.local_listen.to_string(),
        "--mode".to_string(),
        args.mode.clone(),
        "--relay-addr".to_string(),
        args.relay_addr.clone(),
        "--exit-addr".to_string(),
        args.exit_addr.clone(),
        "--route-length".to_string(),
        args.route_length.to_string(),
        "--client-key-path".to_string(),
        args.client_key_path.clone(),
        "--relay-pubkey-path".to_string(),
        args.relay_pubkey_path.clone(),
        "--route-cache-path".to_string(),
        args.route_cache_path.clone(),
        "--tun-name".to_string(),
        args.tun_name.clone(),
        "--tun-address".to_string(),
        args.tun_address.clone(),
        "--tun-netmask".to_string(),
        args.tun_netmask.clone(),
        "--tun-mtu".to_string(),
        args.tun_mtu.to_string(),
        "--max-inflight-frames".to_string(),
        args.max_inflight_frames.to_string(),
        "--retransmit-interval".to_string(),
        args.retransmit_interval.to_string(),
        "--chunk-size".to_string(),
        args.chunk_size.to_string(),
    ];

    if let Some(relay2_addr) = &args.relay2_addr {
        cli_args.push("--relay2-addr".to_string());
        cli_args.push(relay2_addr.clone());
    }
    if args.exact_route_only {
        cli_args.push("--exact-route-only".to_string());
    }
    if args.discovery_enabled {
        cli_args.push("--discovery-enabled".to_string());
    }
    if args.discovery_query_on_start {
        cli_args.push("--discovery-query-on-start".to_string());
    }
    if let Some(peers) = &args.discovery_bootstrap_peers {
        let joined = peers
            .iter()
            .map(|peer| peer.to_string())
            .collect::<Vec<_>>()
            .join(",");
        if !joined.is_empty() {
            cli_args.push("--discovery-bootstrap-peers".to_string());
            cli_args.push(joined);
        }
    }

    cli_args
}

fn required_env(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("{name} must be set for remote baseline mode"))
}

fn env_or_default(name: &str, default: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| default.to_string())
}

fn parse_env_or_default<T>(name: &str, default: T) -> Result<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match std::env::var(name) {
        Ok(raw) => raw
            .parse::<T>()
            .map_err(|e| anyhow::anyhow!("failed to parse {name}={raw:?}: {e}")),
        Err(_) => Ok(default),
    }
}

fn reject_loopback(name: &str, value: &str) -> Result<()> {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.starts_with("127.")
        || normalized.starts_with("localhost:")
        || normalized.starts_with("[::1]")
        || normalized.starts_with("::1:")
    {
        bail!("{name} must point to a real remote node, not loopback: {value}");
    }
    Ok(())
}

fn parse_target_scheme() -> Result<String> {
    let scheme = env_or_default("VPNNODE_BASELINE_REMOTE_TARGET_SCHEME", "http");
    if scheme.eq_ignore_ascii_case("http") {
        Ok("http".to_string())
    } else {
        bail!(
            "VPNNODE_BASELINE_REMOTE_TARGET_SCHEME={scheme:?} is not supported in remote baseline mode; use a plain HTTP target for the current smoke harness"
        );
    }
}
