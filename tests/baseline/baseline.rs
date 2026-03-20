#[path = "../helpers/mod.rs"]
mod helpers;

use std::net::SocketAddr;
use std::sync::Once;

use tokio::sync::Barrier;
use tracing_subscriber::EnvFilter;

use anyhow::{anyhow, bail};
use vpnnode::config::{ClientConfigCli, ExitConfigCli, RelayConfigCli, TargetConfigCli};

static INIT_TRACING: Once = Once::new();

fn init_tracing() {
    INIT_TRACING.call_once(|| {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .try_init();
    });
}

fn http_get_request() -> &'static [u8] {
    b"GET / HTTP/1.1\r\nHost: example\r\nConnection: close\r\n\r\n"
}

#[tokio::test]
async fn baseline_handshake_challenge() {
    init_tracing();
    std::env::set_var("VPNNODE_ANTS", "0");

    let target_addr: SocketAddr = "127.0.0.1:16180".parse().unwrap();
    let exit_udp: SocketAddr = "127.0.0.1:16101".parse().unwrap();
    let relay_udp: SocketAddr = "127.0.0.1:16100".parse().unwrap();
    let client_tcp: SocketAddr = "127.0.0.1:16102".parse().unwrap();

    let target_args = TargetConfigCli { listen: target_addr };
    tokio::spawn(async move {
        let _ = vpnnode::roles::target::run_target(target_args).await;
    });

    let exit_args = ExitConfigCli {
        listen: exit_udp,
        target_addr: target_addr.to_string(),
        exit_key_path: "exit.key".to_string(),
        ..Default::default()
    };
    tokio::spawn(async move {
        let _ = vpnnode::roles::exit::run_exit(exit_args).await;
    });

    let relay_args = RelayConfigCli {
        listen: relay_udp,
        exit_addr: exit_udp.to_string(),
        relay_key_path: "relay.key".to_string(),
        ..Default::default()
    };
    tokio::spawn(async move {
        let _ = vpnnode::roles::relay::run_relay(relay_args).await;
    });

    let client_args = ClientConfigCli {
        local_listen: client_tcp,
        mode: "tcp".to_string(),
        relay_addr: relay_udp.to_string(),
        exit_addr: exit_udp.to_string(),
        route_length: 2,
        relay2_addr: None,
        client_key_path: "client.key".to_string(),
        relay_pubkey_path: "relay.pub".to_string(),
        route_cache_path: "route_cache_test_challenge.json".to_string(),
        tun_name: "tun-baseline-handshake".to_string(),
        tun_address: "10.10.0.1".to_string(),
        tun_netmask: "255.255.255.0".to_string(),
        tun_mtu: 1500,
        max_inflight_frames: 64,
        retransmit_interval: 200,
        chunk_size: 1000,
        ..Default::default()
    };
    tokio::spawn(async move {
        let _ = vpnnode::roles::client::run_client(client_args).await;
    });

    helpers::wait_http_ready(client_tcp).await.expect("client HTTP path must become ready");
    let mut stream = helpers::connect_client(client_tcp).await.expect("connect to client");
    let resp = helpers::send_and_receive(&mut stream, http_get_request())
        .await
        .expect("request/response");

    let status = helpers::extract_http_status_code(&resp)
        .expect("HTTP status code must be present");
    assert_eq!(status, 200, "handshake challenge must return 200 OK");
}

#[tokio::test]
async fn baseline_route_1hop_client_to_exit() {
    init_tracing();
    std::env::set_var("VPNNODE_ANTS", "0");

    let target_addr: SocketAddr = "127.0.0.1:16280".parse().unwrap();
    let exit_udp: SocketAddr = "127.0.0.1:16201".parse().unwrap();
    let client_tcp: SocketAddr = "127.0.0.1:16202".parse().unwrap();

    let target_args = TargetConfigCli { listen: target_addr };
    tokio::spawn(async move {
        let _ = vpnnode::roles::target::run_target(target_args).await;
    });

    let exit_args = ExitConfigCli {
        listen: exit_udp,
        target_addr: target_addr.to_string(),
        exit_key_path: "exit.key".to_string(),
        ..Default::default()
    };
    tokio::spawn(async move {
        let _ = vpnnode::roles::exit::run_exit(exit_args).await;
    });

    let client_args = ClientConfigCli {
        local_listen: client_tcp,
        mode: "tcp".to_string(),
        relay_addr: "127.0.0.1:1".to_string(), // unused for route-length 1
        exit_addr: exit_udp.to_string(),
        route_length: 1,
        relay2_addr: None,
        client_key_path: "client.key".to_string(),
        relay_pubkey_path: "relay.pub".to_string(),
        route_cache_path: "route_cache_test_1hop.json".to_string(),
        tun_name: "tun-baseline-1hop".to_string(),
        tun_address: "10.10.0.1".to_string(),
        tun_netmask: "255.255.255.0".to_string(),
        tun_mtu: 1500,
        max_inflight_frames: 64,
        retransmit_interval: 200,
        chunk_size: 1000,
        ..Default::default()
    };
    tokio::spawn(async move {
        let _ = vpnnode::roles::client::run_client(client_args).await;
    });

    helpers::wait_http_ready(client_tcp).await.expect("client HTTP path must become ready");
    let mut stream = helpers::connect_client(client_tcp).await.expect("connect to client");
    let resp = helpers::send_and_receive(&mut stream, http_get_request())
        .await
        .expect("request/response");

    let status = helpers::extract_http_status_code(&resp)
        .expect("HTTP status code must be present");
    assert_eq!(status, 200, "1-hop route must return 200 OK");
}

#[tokio::test]
async fn baseline_route_2hop_client_via_relay_to_exit() {
    init_tracing();
    std::env::set_var("VPNNODE_ANTS", "0");

    let target_addr: SocketAddr = "127.0.0.1:16380".parse().unwrap();
    let exit_udp: SocketAddr = "127.0.0.1:16301".parse().unwrap();
    let relay_udp: SocketAddr = "127.0.0.1:16300".parse().unwrap();
    let client_tcp: SocketAddr = "127.0.0.1:16302".parse().unwrap();

    tokio::spawn(async move {
        let _ = vpnnode::roles::target::run_target(TargetConfigCli { listen: target_addr }).await;
    });

    tokio::spawn(async move {
        let _ = vpnnode::roles::exit::run_exit(ExitConfigCli {
            listen: exit_udp,
            target_addr: target_addr.to_string(),
            exit_key_path: "exit.key".to_string(),
            ..Default::default()
        })
        .await;
    });

    tokio::spawn(async move {
        let _ = vpnnode::roles::relay::run_relay(RelayConfigCli {
            listen: relay_udp,
            exit_addr: exit_udp.to_string(),
            relay_key_path: "relay.key".to_string(),
            ..Default::default()
        })
        .await;
    });

    let client_args = ClientConfigCli {
        local_listen: client_tcp,
        mode: "tcp".to_string(),
        relay_addr: relay_udp.to_string(),
        exit_addr: exit_udp.to_string(),
        route_length: 2,
        relay2_addr: None,
        client_key_path: "client.key".to_string(),
        relay_pubkey_path: "relay.pub".to_string(),
        route_cache_path: "route_cache_test_2hop.json".to_string(),
        tun_name: "tun-baseline-2hop".to_string(),
        tun_address: "10.10.0.1".to_string(),
        tun_netmask: "255.255.255.0".to_string(),
        tun_mtu: 1500,
        max_inflight_frames: 64,
        retransmit_interval: 200,
        chunk_size: 1000,
        ..Default::default()
    };
    tokio::spawn(async move {
        let _ = vpnnode::roles::client::run_client(client_args).await;
    });

    helpers::wait_http_ready(client_tcp).await.expect("client HTTP path must become ready");
    let mut stream = helpers::connect_client(client_tcp).await.expect("connect to client");
    let resp = helpers::send_and_receive(&mut stream, http_get_request())
        .await
        .expect("request/response");

    let status = helpers::extract_http_status_code(&resp)
        .expect("HTTP status code must be present");
    assert_eq!(status, 200, "2-hop route must return 200 OK");
}

#[tokio::test]
async fn baseline_route_3hop_via_two_relays_to_exit() {
    init_tracing();
    std::env::set_var("VPNNODE_ANTS", "0");

    let target_addr: SocketAddr = "127.0.0.1:16480".parse().unwrap();
    let exit_udp: SocketAddr = "127.0.0.1:16402".parse().unwrap();
    let relay2_udp: SocketAddr = "127.0.0.1:16401".parse().unwrap();
    let relay1_udp: SocketAddr = "127.0.0.1:16400".parse().unwrap();
    let client_tcp: SocketAddr = "127.0.0.1:16403".parse().unwrap();

    tokio::spawn(async move {
        let _ = vpnnode::roles::target::run_target(TargetConfigCli { listen: target_addr }).await;
    });

    tokio::spawn(async move {
        let _ = vpnnode::roles::exit::run_exit(ExitConfigCli {
            listen: exit_udp,
            target_addr: target_addr.to_string(),
            exit_key_path: "exit.key".to_string(),
            ..Default::default()
        })
        .await;
    });

    tokio::spawn(async move {
        let _ = vpnnode::roles::relay::run_relay(RelayConfigCli {
            listen: relay2_udp,
            exit_addr: exit_udp.to_string(),
            relay_key_path: "relay.key".to_string(),
            ..Default::default()
        })
        .await;
    });

    tokio::spawn(async move {
        let _ = vpnnode::roles::relay::run_relay(RelayConfigCli {
            listen: relay1_udp,
            exit_addr: relay2_udp.to_string(),
            relay_key_path: "relay.key".to_string(),
            ..Default::default()
        })
        .await;
    });

    let client_args = ClientConfigCli {
        local_listen: client_tcp,
        mode: "tcp".to_string(),
        relay_addr: relay1_udp.to_string(),
        exit_addr: exit_udp.to_string(),
        route_length: 3,
        relay2_addr: Some(relay2_udp.to_string()),
        client_key_path: "client.key".to_string(),
        relay_pubkey_path: "relay.pub".to_string(),
        route_cache_path: "route_cache_test_3hop.json".to_string(),
        tun_name: "tun-baseline-3hop".to_string(),
        tun_address: "10.10.0.1".to_string(),
        tun_netmask: "255.255.255.0".to_string(),
        tun_mtu: 1500,
        max_inflight_frames: 64,
        retransmit_interval: 200,
        chunk_size: 1000,
        ..Default::default()
    };
    tokio::spawn(async move {
        let _ = vpnnode::roles::client::run_client(client_args).await;
    });

    helpers::wait_http_ready(client_tcp).await.expect("client HTTP path must become ready");
    let mut stream = helpers::connect_client(client_tcp).await.expect("connect to client");
    let resp = helpers::send_and_receive(&mut stream, http_get_request())
        .await
        .expect("request/response");

    let status = helpers::extract_http_status_code(&resp)
        .expect("HTTP status code must be present");
    assert_eq!(status, 200, "3-hop route must return 200 OK");
}

#[tokio::test]
async fn baseline_large_transfer_ge_5mb() {
    init_tracing();
    std::env::set_var("VPNNODE_ANTS", "0");

    let target_addr: SocketAddr = "127.0.0.1:16580".parse().unwrap();
    let exit_udp: SocketAddr = "127.0.0.1:16501".parse().unwrap();
    let relay_udp: SocketAddr = "127.0.0.1:16500".parse().unwrap();
    let client_tcp: SocketAddr = "127.0.0.1:16502".parse().unwrap();

    tokio::spawn(async move {
        let _ = vpnnode::roles::target::run_target(TargetConfigCli { listen: target_addr }).await;
    });

    tokio::spawn(async move {
        let _ = vpnnode::roles::exit::run_exit(ExitConfigCli {
            listen: exit_udp,
            target_addr: target_addr.to_string(),
            exit_key_path: "exit.key".to_string(),
            ..Default::default()
        })
        .await;
    });

    tokio::spawn(async move {
        let _ = vpnnode::roles::relay::run_relay(RelayConfigCli {
            listen: relay_udp,
            exit_addr: exit_udp.to_string(),
            relay_key_path: "relay.key".to_string(),
            ..Default::default()
        })
        .await;
    });

    let client_args = ClientConfigCli {
        local_listen: client_tcp,
        mode: "tcp".to_string(),
        relay_addr: relay_udp.to_string(),
        exit_addr: exit_udp.to_string(),
        route_length: 2,
        relay2_addr: None,
        client_key_path: "client.key".to_string(),
        relay_pubkey_path: "relay.pub".to_string(),
        route_cache_path: "route_cache_test_large.json".to_string(),
        tun_name: "tun-baseline-large".to_string(),
        tun_address: "10.10.0.1".to_string(),
        tun_netmask: "255.255.255.0".to_string(),
        tun_mtu: 1500,
        max_inflight_frames: 64,
        retransmit_interval: 200,
        chunk_size: 1000,
        ..Default::default()
    };
    tokio::spawn(async move {
        let _ = vpnnode::roles::client::run_client(client_args).await;
    });

    helpers::wait_http_ready(client_tcp).await.expect("client HTTP path must become ready");
    let mut stream = helpers::connect_client(client_tcp).await.expect("connect to client");

    let body_size: usize = 5 * 1024 * 1024; // 5 MiB
    let body = vec![b'X'; body_size];
    let header = format!(
        "POST /echo HTTP/1.1\r\nHost: example\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body_size
    );
    let mut request = Vec::with_capacity(header.as_bytes().len() + body.len());
    request.extend_from_slice(header.as_bytes());
    request.extend_from_slice(&body);

    let resp = helpers::send_and_receive(&mut stream, &request)
        .await
        .expect("request/response");

    let status = helpers::extract_http_status_code(&resp)
        .expect("HTTP status code must be present");
    assert_eq!(status, 200, "large transfer must return 200 OK");

    // Response body should include the echoed body segment.
    let header_end = resp
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|p| p + 4)
        .expect("response must contain header terminator");
    let response_body = &resp[header_end..];
    assert!(
        response_body.ends_with(&body),
        "echoed body must be present at the end of response body (expected {}, got {})",
        body.len(),
        response_body.len()
    );
}

#[tokio::test]
async fn baseline_parallel_streams_ge_5() {
    init_tracing();
    std::env::set_var("VPNNODE_ANTS", "0");

    let target_addr: SocketAddr = "127.0.0.1:16680".parse().unwrap();
    let exit_udp: SocketAddr = "127.0.0.1:16601".parse().unwrap();
    let relay_udp: SocketAddr = "127.0.0.1:16600".parse().unwrap();
    let client_tcp: SocketAddr = "127.0.0.1:16602".parse().unwrap();

    tokio::spawn(async move {
        let _ = vpnnode::roles::target::run_target(TargetConfigCli { listen: target_addr }).await;
    });

    tokio::spawn(async move {
        let _ = vpnnode::roles::exit::run_exit(ExitConfigCli {
            listen: exit_udp,
            target_addr: target_addr.to_string(),
            exit_key_path: "exit.key".to_string(),
            ..Default::default()
        })
        .await;
    });

    tokio::spawn(async move {
        let _ = vpnnode::roles::relay::run_relay(RelayConfigCli {
            listen: relay_udp,
            exit_addr: exit_udp.to_string(),
            relay_key_path: "relay.key".to_string(),
            ..Default::default()
        })
        .await;
    });

    let client_args = ClientConfigCli {
        local_listen: client_tcp,
        mode: "tcp".to_string(),
        relay_addr: relay_udp.to_string(),
        exit_addr: exit_udp.to_string(),
        route_length: 2,
        relay2_addr: None,
        client_key_path: "client.key".to_string(),
        relay_pubkey_path: "relay.pub".to_string(),
        route_cache_path: "route_cache_test_parallel.json".to_string(),
        tun_name: "tun-baseline-parallel".to_string(),
        tun_address: "10.10.0.1".to_string(),
        tun_netmask: "255.255.255.0".to_string(),
        tun_mtu: 1500,
        max_inflight_frames: 64,
        retransmit_interval: 200,
        chunk_size: 1000,
        ..Default::default()
    };
    tokio::spawn(async move {
        let _ = vpnnode::roles::client::run_client(client_args).await;
    });

    helpers::wait_http_ready(client_tcp).await.expect("client HTTP path must become ready");

    let n = 5usize;
    let barrier = std::sync::Arc::new(Barrier::new(n));
    let mut handles = Vec::with_capacity(n);
    for _ in 0..n {
        let barrier = barrier.clone();
        handles.push(tokio::spawn(async move {
            barrier.wait().await;
            let mut stream = helpers::connect_client(client_tcp).await?;
            let resp = helpers::send_and_receive(&mut stream, http_get_request()).await?;
            let status = helpers::extract_http_status_code(&resp)
                .ok_or_else(|| anyhow!("missing http status code in response"))?;
            if status != 200 {
                bail!("expected 200 OK, got {status}");
            }
            Ok::<(), anyhow::Error>(())
        }));
    }

    for h in handles {
        let r = h.await.expect("parallel stream task should not panic");
        r.expect("parallel stream task must return Ok");
    }
}

#[tokio::test]
async fn baseline_target_unavailable_returns_502() {
    init_tracing();
    std::env::set_var("VPNNODE_ANTS", "0");

    let exit_udp: SocketAddr = "127.0.0.1:16701".parse().unwrap();
    let relay_udp: SocketAddr = "127.0.0.1:16700".parse().unwrap();
    let client_tcp: SocketAddr = "127.0.0.1:16702".parse().unwrap();

    // Exit points to a non-listening target port -> deterministic error path.
    let exit_args = ExitConfigCli {
        listen: exit_udp,
        target_addr: "127.0.0.1:28999".to_string(),
        exit_key_path: "exit.key".to_string(),
        ..Default::default()
    };
    tokio::spawn(async move {
        let _ = vpnnode::roles::exit::run_exit(exit_args).await;
    });

    let relay_args = RelayConfigCli {
        listen: relay_udp,
        exit_addr: exit_udp.to_string(),
        relay_key_path: "relay.key".to_string(),
        ..Default::default()
    };
    tokio::spawn(async move {
        let _ = vpnnode::roles::relay::run_relay(relay_args).await;
    });

    let client_args = ClientConfigCli {
        local_listen: client_tcp,
        mode: "tcp".to_string(),
        relay_addr: relay_udp.to_string(),
        exit_addr: exit_udp.to_string(),
        route_length: 2,
        relay2_addr: None,
        client_key_path: "client.key".to_string(),
        relay_pubkey_path: "relay.pub".to_string(),
        route_cache_path: "route_cache_test_unavail.json".to_string(),
        tun_name: "tun-baseline-unavail".to_string(),
        tun_address: "10.10.0.1".to_string(),
        tun_netmask: "255.255.255.0".to_string(),
        tun_mtu: 1500,
        max_inflight_frames: 64,
        retransmit_interval: 200,
        chunk_size: 1000,
        ..Default::default()
    };
    tokio::spawn(async move {
        let _ = vpnnode::roles::client::run_client(client_args).await;
    });

    helpers::wait_http_ready(client_tcp).await.expect("client HTTP path must become ready");
    let mut stream = helpers::connect_client(client_tcp).await.expect("connect to client");
    let resp = helpers::send_and_receive(&mut stream, http_get_request())
        .await
        .expect("request/response");

    let status = helpers::extract_http_status_code(&resp)
        .expect("HTTP status code must be present");
    assert_eq!(status, 502, "target unavailable must return 502 Bad Gateway");
}

#[tokio::test]
async fn baseline_small_response_returns_200_and_completes() {
    init_tracing();
    std::env::set_var("VPNNODE_ANTS", "0");

    let target_addr: SocketAddr = "127.0.0.1:16880".parse().unwrap();
    let exit_udp: SocketAddr = "127.0.0.1:16801".parse().unwrap();
    let relay_udp: SocketAddr = "127.0.0.1:16800".parse().unwrap();
    let client_tcp: SocketAddr = "127.0.0.1:16802".parse().unwrap();

    tokio::spawn(async move {
        let _ = vpnnode::roles::target::run_target(TargetConfigCli { listen: target_addr }).await;
    });
    tokio::spawn(async move {
        let _ = vpnnode::roles::exit::run_exit(ExitConfigCli {
            listen: exit_udp,
            target_addr: target_addr.to_string(),
            exit_key_path: "exit.key".to_string(),
            ..Default::default()
        })
        .await;
    });
    tokio::spawn(async move {
        let _ = vpnnode::roles::relay::run_relay(RelayConfigCli {
            listen: relay_udp,
            exit_addr: exit_udp.to_string(),
            relay_key_path: "relay.key".to_string(),
            ..Default::default()
        })
        .await;
    });
    tokio::spawn(async move {
        let _ = vpnnode::roles::client::run_client(ClientConfigCli {
            local_listen: client_tcp,
            mode: "tcp".to_string(),
            relay_addr: relay_udp.to_string(),
            exit_addr: exit_udp.to_string(),
            route_length: 2,
            relay2_addr: None,
            client_key_path: "client.key".to_string(),
            relay_pubkey_path: "relay.pub".to_string(),
            route_cache_path: "route_cache_test_small_resp.json".to_string(),
            tun_name: "tun-baseline-small".to_string(),
            tun_address: "10.10.0.1".to_string(),
            tun_netmask: "255.255.255.0".to_string(),
            tun_mtu: 1500,
            max_inflight_frames: 64,
            retransmit_interval: 200,
            chunk_size: 1000,
            ..Default::default()
        })
        .await;
    });

    helpers::wait_http_ready(client_tcp).await.expect("client HTTP path must become ready");
    let mut stream = helpers::connect_client(client_tcp).await.expect("connect to client");
    let resp = helpers::send_and_receive(&mut stream, http_get_request())
        .await
        .expect("request/response");

    let status = helpers::extract_http_status_code(&resp)
        .expect("HTTP status code must be present");
    assert_eq!(status, 200, "small response must return 200 OK");
    assert!(
        !resp.is_empty(),
        "small response must complete with non-empty payload"
    );
}

#[tokio::test]
async fn baseline_response_completion_respects_non_pathological_growth() {
    init_tracing();
    std::env::set_var("VPNNODE_ANTS", "0");

    let target_addr: SocketAddr = "127.0.0.1:16980".parse().unwrap();
    let exit_udp: SocketAddr = "127.0.0.1:16901".parse().unwrap();
    let relay_udp: SocketAddr = "127.0.0.1:16900".parse().unwrap();
    let client_tcp: SocketAddr = "127.0.0.1:16902".parse().unwrap();

    tokio::spawn(async move {
        let _ = vpnnode::roles::target::run_target(TargetConfigCli { listen: target_addr }).await;
    });
    tokio::spawn(async move {
        let _ = vpnnode::roles::exit::run_exit(ExitConfigCli {
            listen: exit_udp,
            target_addr: target_addr.to_string(),
            exit_key_path: "exit.key".to_string(),
            ..Default::default()
        })
        .await;
    });
    tokio::spawn(async move {
        let _ = vpnnode::roles::relay::run_relay(RelayConfigCli {
            listen: relay_udp,
            exit_addr: exit_udp.to_string(),
            relay_key_path: "relay.key".to_string(),
            ..Default::default()
        })
        .await;
    });
    tokio::spawn(async move {
        let _ = vpnnode::roles::client::run_client(ClientConfigCli {
            local_listen: client_tcp,
            mode: "tcp".to_string(),
            relay_addr: relay_udp.to_string(),
            exit_addr: exit_udp.to_string(),
            route_length: 2,
            relay2_addr: None,
            client_key_path: "client.key".to_string(),
            relay_pubkey_path: "relay.pub".to_string(),
            route_cache_path: "route_cache_test_growth_guard.json".to_string(),
            tun_name: "tun-baseline-growth".to_string(),
            tun_address: "10.10.0.1".to_string(),
            tun_netmask: "255.255.255.0".to_string(),
            tun_mtu: 1500,
            max_inflight_frames: 64,
            retransmit_interval: 200,
            chunk_size: 1000,
            ..Default::default()
        })
        .await;
    });

    helpers::wait_http_ready(client_tcp).await.expect("client HTTP path must become ready");
    let mut stream = helpers::connect_client(client_tcp).await.expect("connect to client");

    let body = vec![b'Z'; 32 * 1024];
    let header = format!(
        "POST /echo HTTP/1.1\r\nHost: example\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let mut req = Vec::with_capacity(header.len() + body.len());
    req.extend_from_slice(header.as_bytes());
    req.extend_from_slice(&body);

    let resp = helpers::send_and_receive(&mut stream, &req)
        .await
        .expect("request/response");
    let status = helpers::extract_http_status_code(&resp)
        .expect("HTTP status code must be present");
    assert_eq!(status, 200, "non-pathological response must return 200 OK");

    let hdr_end = resp
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .map(|p| p + 4)
        .expect("response must contain header terminator");
    let response_body = &resp[hdr_end..];
    assert!(
        response_body.ends_with(&body),
        "response body must complete and include echoed payload"
    );
    assert!(
        response_body.len() <= req.len() + 64 * 1024,
        "response should not exhibit pathological growth"
    );
}

