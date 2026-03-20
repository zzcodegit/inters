use crate::node_config::{NodeConfig, NodeRole};
use std::path::Path;

pub async fn health_check(path: &Path) -> anyhow::Result<(NodeRole, std::net::SocketAddr)> {
    let cfg = NodeConfig::load_from_file(path)?;
    if !cfg.health_enabled {
        anyhow::bail!("health disabled by config");
    }

    // Validate role init does not fail (foundation-only: bind feasibility).
    match cfg.role {
        NodeRole::Client => {
            let _tcp = tokio::net::TcpListener::bind(cfg.client_local_listen).await?;
            let _udp = tokio::net::UdpSocket::bind(cfg.bind_addr()).await?;
        }
        NodeRole::Relay | NodeRole::Exit => {
            let _udp = tokio::net::UdpSocket::bind(cfg.bind_addr()).await?;
        }
    }

    // Exit-only: validate target allowlist shape (foundation only).
    if cfg.role == NodeRole::Exit {
        if let Some(list) = &cfg.exit_target_allowlist {
            if list.iter().any(|s| s.trim().is_empty()) {
                anyhow::bail!("exit_target_allowlist contains empty entry");
            }
        }
        if cfg.exit_target_addr.trim().is_empty() {
            anyhow::bail!("exit_target_addr must be non-empty");
        }
    }

    Ok((cfg.role, cfg.bind_addr()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::SocketAddr;
    use rand_core::{OsRng, RngCore};

    #[tokio::test]
    async fn health_bind_conflict_fails() {
        // Bind a UDP socket, then attempt health_check on same port via temp config.
        let sa: SocketAddr = "127.0.0.1:0".parse().unwrap();
        let sock = tokio::net::UdpSocket::bind(sa).await.unwrap();
        let bound = sock.local_addr().unwrap();

        let mut rnd = [0u8; 8];
        OsRng.fill_bytes(&mut rnd);
        let name = format!("vpnnode-health-test-{:x}.toml", u64::from_be_bytes(rnd));
        let tmp = std::env::temp_dir().join(name);
        std::fs::write(
            &tmp,
            format!(
                r#"
role = "relay"
bind_ip = "127.0.0.1"
bind_port = {}
drain_timeout_sec = 20
peers = [{{ ip = "127.0.0.1", port = 30001, protocol = "Udp" }}]
"#,
                bound.port()
            ),
        )
        .unwrap();

        let r = health_check(&tmp).await;
        let _ = std::fs::remove_file(&tmp);
        assert!(r.is_err());
    }
}

