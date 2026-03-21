use crate::addr::{NodeAddr, Protocol};
use crate::stage_trace;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::time::sleep;
use tracing::debug;

#[async_trait]
pub trait Transport: Send + Sync {
    async fn send(&self, to: &NodeAddr, data: &[u8]) -> Result<()>;
    async fn recv(&self) -> Result<(NodeAddr, Vec<u8>)>;
    fn local_addr(&self) -> Result<NodeAddr>;
}

#[derive(Clone)]
pub struct UdpTransport {
    socket: Arc<UdpSocket>,
    chaos: Option<Arc<TransportChaos>>,
}

impl UdpTransport {
    pub async fn bind(addr: SocketAddr) -> Result<Self> {
        let socket = UdpSocket::bind(addr).await?;
        Ok(Self::from_socket(socket))
    }

    pub async fn connect(addr: SocketAddr) -> Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0").await?;
        socket.connect(addr).await?;
        Ok(Self::from_socket(socket))
    }

    pub fn from_socket(socket: UdpSocket) -> Self {
        Self {
            socket: Arc::new(socket),
            chaos: TransportChaos::from_env().map(Arc::new),
        }
    }
}

#[async_trait]
impl Transport for UdpTransport {
    async fn send(&self, to: &NodeAddr, data: &[u8]) -> Result<()> {
        let sa = to
            .as_socket_addr()
            .ok_or_else(|| anyhow::anyhow!("UdpTransport: non-UDP NodeAddr"))?;
        debug!(protocol = ?to.protocol, to = %to, bytes = data.len(), "transport send");
        if let Some(chaos) = &self.chaos {
            return chaos.send(&self.socket, sa, data).await;
        }
        self.socket.send_to(data, sa).await?;
        Ok(())
    }

    async fn recv(&self) -> Result<(NodeAddr, Vec<u8>)> {
        let mut buf = vec![0u8; 65535];
        let (len, addr) = self.socket.recv_from(&mut buf).await?;
        buf.truncate(len);
        let from = NodeAddr::from(addr);
        debug!(protocol = ?from.protocol, from = %from, bytes = len, "transport recv");
        Ok((from, buf))
    }

    fn local_addr(&self) -> Result<NodeAddr> {
        let sa = self.socket.local_addr()?;
        Ok(NodeAddr {
            ip: sa.ip(),
            port: sa.port(),
            protocol: Protocol::Udp,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ChaosProfile {
    label: String,
    seed: u64,
    skip_packets: u64,
    loss_ppm: u32,
    duplicate_ppm: u32,
    reorder_ppm: u32,
    base_delay_ms: u64,
    jitter_ms: u64,
    reorder_extra_delay_ms: u64,
    duplicate_delay_ms: u64,
}

impl ChaosProfile {
    fn from_env() -> Option<Self> {
        let profile = Self {
            label: env_string("VPNNODE_TRANSPORT_CHAOS_LABEL")
                .unwrap_or_else(|| "env".to_string()),
            seed: env_u64("VPNNODE_TRANSPORT_CHAOS_SEED").unwrap_or(1),
            skip_packets: env_u64("VPNNODE_TRANSPORT_CHAOS_SKIP_PACKETS").unwrap_or(0),
            loss_ppm: env_u32("VPNNODE_TRANSPORT_CHAOS_LOSS_PPM").unwrap_or(0),
            duplicate_ppm: env_u32("VPNNODE_TRANSPORT_CHAOS_DUPLICATE_PPM").unwrap_or(0),
            reorder_ppm: env_u32("VPNNODE_TRANSPORT_CHAOS_REORDER_PPM").unwrap_or(0),
            base_delay_ms: env_u64("VPNNODE_TRANSPORT_CHAOS_BASE_DELAY_MS").unwrap_or(0),
            jitter_ms: env_u64("VPNNODE_TRANSPORT_CHAOS_JITTER_MS").unwrap_or(0),
            reorder_extra_delay_ms: env_u64("VPNNODE_TRANSPORT_CHAOS_REORDER_EXTRA_DELAY_MS")
                .unwrap_or(0),
            duplicate_delay_ms: env_u64("VPNNODE_TRANSPORT_CHAOS_DUPLICATE_DELAY_MS")
                .unwrap_or(2),
        };
        if profile.is_active() {
            Some(profile)
        } else {
            None
        }
    }

    fn is_active(&self) -> bool {
        self.loss_ppm > 0
            || self.duplicate_ppm > 0
            || self.reorder_ppm > 0
            || self.base_delay_ms > 0
            || self.jitter_ms > 0
            || self.reorder_extra_delay_ms > 0
    }

    fn plan(&self, packet_index: u64, local_port: u16, to_port: u16, bytes: usize) -> ChaosDecision {
        if packet_index <= self.skip_packets {
            return ChaosDecision {
                packet_index,
                local_port,
                to_port,
                bytes,
                primary_delay_ms: Some(0),
                duplicate_delay_ms: None,
                dropped: false,
                reordered: false,
                jitter_ms: 0,
            };
        }

        let jitter_ms = if self.jitter_ms == 0 {
            0
        } else {
            sample_range(
                self.seed,
                packet_index,
                local_port,
                to_port,
                bytes,
                3,
                self.jitter_ms.saturating_add(1),
            )
        };
        let reordered = sample_ppm(
            self.seed,
            packet_index,
            local_port,
            to_port,
            bytes,
            1,
            self.reorder_ppm,
        );
        let dropped = sample_ppm(
            self.seed,
            packet_index,
            local_port,
            to_port,
            bytes,
            2,
            self.loss_ppm,
        );
        let duplicate = sample_ppm(
            self.seed,
            packet_index,
            local_port,
            to_port,
            bytes,
            4,
            self.duplicate_ppm,
        );
        let base_delay_ms = self
            .base_delay_ms
            .saturating_add(jitter_ms)
            .saturating_add(if reordered {
                self.reorder_extra_delay_ms.max(1)
            } else {
                0
            });
        let primary_delay_ms = if dropped { None } else { Some(base_delay_ms) };
        let duplicate_delay_ms = if duplicate {
            Some(
                base_delay_ms
                    .saturating_add(self.duplicate_delay_ms.max(1))
                    .max(1),
            )
        } else {
            None
        };

        ChaosDecision {
            packet_index,
            local_port,
            to_port,
            bytes,
            primary_delay_ms,
            duplicate_delay_ms,
            dropped,
            reordered,
            jitter_ms,
        }
    }
}

struct TransportChaos {
    profile: ChaosProfile,
    send_counter: AtomicU64,
}

impl TransportChaos {
    fn from_env() -> Option<Self> {
        ChaosProfile::from_env().map(|profile| Self {
            profile,
            send_counter: AtomicU64::new(0),
        })
    }

    async fn send(&self, socket: &Arc<UdpSocket>, to: SocketAddr, data: &[u8]) -> Result<()> {
        let local_port = socket.local_addr().map(|addr| addr.port()).unwrap_or_default();
        let packet_index = self.send_counter.fetch_add(1, Ordering::Relaxed) + 1;
        let decision = self
            .profile
            .plan(packet_index, local_port, to.port(), data.len());

        if let Some(delay_ms) = decision.primary_delay_ms {
            if delay_ms == 0 {
                socket.send_to(data, to).await?;
            } else {
                spawn_delayed_send(socket.clone(), to, data.to_vec(), delay_ms);
                emit_transport_chaos(
                    "chaos_delay",
                    &self.profile,
                    &decision,
                    json!({
                        "kind": "primary",
                        "delay_ms": delay_ms,
                    }),
                );
            }
        } else {
            emit_transport_chaos(
                "chaos_drop",
                &self.profile,
                &decision,
                json!({
                    "kind": "primary",
                }),
            );
        }

        if let Some(delay_ms) = decision.duplicate_delay_ms {
            spawn_delayed_send(socket.clone(), to, data.to_vec(), delay_ms);
            emit_transport_chaos(
                "chaos_duplicate",
                &self.profile,
                &decision,
                json!({
                    "kind": "duplicate",
                    "delay_ms": delay_ms,
                }),
            );
        }

        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ChaosDecision {
    packet_index: u64,
    local_port: u16,
    to_port: u16,
    bytes: usize,
    primary_delay_ms: Option<u64>,
    duplicate_delay_ms: Option<u64>,
    dropped: bool,
    reordered: bool,
    jitter_ms: u64,
}

fn spawn_delayed_send(socket: Arc<UdpSocket>, to: SocketAddr, data: Vec<u8>, delay_ms: u64) {
    tokio::spawn(async move {
        sleep(Duration::from_millis(delay_ms)).await;
        let _ = socket.send_to(&data, to).await;
    });
}

fn emit_transport_chaos(
    stage: &str,
    profile: &ChaosProfile,
    decision: &ChaosDecision,
    extra: serde_json::Value,
) {
    if !stage_trace::enabled() {
        return;
    }
    let mut object = stage_trace::event("transport", stage);
    object.insert("profile".to_string(), json!(profile.label));
    object.insert("packet_index".to_string(), json!(decision.packet_index));
    object.insert("local_port".to_string(), json!(decision.local_port));
    object.insert("to_port".to_string(), json!(decision.to_port));
    object.insert("bytes".to_string(), json!(decision.bytes));
    object.insert("dropped".to_string(), json!(decision.dropped));
    object.insert("reordered".to_string(), json!(decision.reordered));
    object.insert("jitter_ms".to_string(), json!(decision.jitter_ms));
    if let serde_json::Value::Object(extra_fields) = extra {
        object.extend(extra_fields);
    }
    stage_trace::emit(serde_json::Value::Object(object));
}

fn env_string(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.trim().is_empty())
}

fn env_u64(name: &str) -> Option<u64> {
    env_string(name).and_then(|value| value.parse().ok())
}

fn env_u32(name: &str) -> Option<u32> {
    env_string(name).and_then(|value| value.parse().ok())
}

fn sample_ppm(
    seed: u64,
    packet_index: u64,
    local_port: u16,
    to_port: u16,
    bytes: usize,
    channel: u64,
    threshold_ppm: u32,
) -> bool {
    if threshold_ppm == 0 {
        return false;
    }
    let threshold_ppm = threshold_ppm.min(1_000_000);
    let sample = mixed_sample(seed, packet_index, local_port, to_port, bytes, channel) % 1_000_000;
    sample < threshold_ppm as u64
}

fn sample_range(
    seed: u64,
    packet_index: u64,
    local_port: u16,
    to_port: u16,
    bytes: usize,
    channel: u64,
    upper_exclusive: u64,
) -> u64 {
    if upper_exclusive <= 1 {
        return 0;
    }
    mixed_sample(seed, packet_index, local_port, to_port, bytes, channel) % upper_exclusive
}

fn mixed_sample(
    seed: u64,
    packet_index: u64,
    local_port: u16,
    to_port: u16,
    bytes: usize,
    channel: u64,
) -> u64 {
    let mut value = seed
        ^ packet_index.wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ ((local_port as u64) << 16)
        ^ (to_port as u64)
        ^ ((bytes as u64) << 32)
        ^ channel.wrapping_mul(0xD1B5_4A32_D192_ED03);
    value ^= value >> 30;
    value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn udp_transport_roundtrip() {
        let addr1: SocketAddr = "127.0.0.1:40001".parse().unwrap();
        let addr2: SocketAddr = "127.0.0.1:40002".parse().unwrap();

        let t1 = UdpTransport::bind(addr1).await.unwrap();
        let t2 = UdpTransport::bind(addr2).await.unwrap();

        let msg = b"hello";

        let h1 = tokio::spawn(async move {
            let (_from, data) = t1.recv().await.unwrap();
            data
        });

        t2.send(&NodeAddr::from(addr1), msg).await.unwrap();
        let received = h1.await.unwrap();
        assert_eq!(&received, msg);
    }

    #[test]
    fn chaos_plan_is_deterministic() {
        let profile = ChaosProfile {
            label: "test".to_string(),
            seed: 7,
            skip_packets: 0,
            loss_ppm: 12_000,
            duplicate_ppm: 7_000,
            reorder_ppm: 9_000,
            base_delay_ms: 4,
            jitter_ms: 3,
            reorder_extra_delay_ms: 11,
            duplicate_delay_ms: 5,
        };
        let left = profile.plan(42, 30001, 30002, 1200);
        let right = profile.plan(42, 30001, 30002, 1200);
        assert_eq!(left, right);
    }

    #[test]
    fn chaos_plan_respects_skip_packets() {
        let profile = ChaosProfile {
            label: "test".to_string(),
            seed: 7,
            skip_packets: 8,
            loss_ppm: 1_000_000,
            duplicate_ppm: 1_000_000,
            reorder_ppm: 1_000_000,
            base_delay_ms: 50,
            jitter_ms: 25,
            reorder_extra_delay_ms: 25,
            duplicate_delay_ms: 5,
        };
        let decision = profile.plan(4, 30001, 30002, 1200);
        assert_eq!(decision.primary_delay_ms, Some(0));
        assert_eq!(decision.duplicate_delay_ms, None);
        assert!(!decision.dropped);
        assert!(!decision.reordered);
    }

    #[test]
    fn chaos_profile_only_activates_when_values_are_nonzero() {
        let inactive = ChaosProfile {
            label: "inactive".to_string(),
            seed: 1,
            skip_packets: 0,
            loss_ppm: 0,
            duplicate_ppm: 0,
            reorder_ppm: 0,
            base_delay_ms: 0,
            jitter_ms: 0,
            reorder_extra_delay_ms: 0,
            duplicate_delay_ms: 2,
        };
        assert!(!inactive.is_active());

        let mut active = inactive.clone();
        active.loss_ppm = 1;
        assert!(active.is_active());
    }
}
