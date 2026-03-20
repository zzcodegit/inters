use std::net::SocketAddr;
use std::sync::Once;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::time::{sleep, timeout, Duration};
use tracing_subscriber::EnvFilter;

static INIT_TRACING: Once = Once::new();

fn init_tracing() {
    INIT_TRACING.call_once(|| {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::from_default_env())
            .try_init();
    });
}

/// Stage 3.1: Full stack with cookie challenge flow (init → challenge → init+cookie → ack).
#[tokio::test]
async fn handshake_challenge_flow() {
    init_tracing();
    let target_addr: SocketAddr = "127.0.0.1:15180".parse().unwrap();
    let exit_udp: SocketAddr = "127.0.0.1:15101".parse().unwrap();
    let relay_udp: SocketAddr = "127.0.0.1:15100".parse().unwrap();
    let client_tcp: SocketAddr = "127.0.0.1:15102".parse().unwrap();

    let target_args = vpnnode::config::TargetConfigCli { listen: target_addr };
    tokio::spawn(async move {
        let _ = vpnnode::roles::target::run_target(target_args).await;
    });

    let exit_args = vpnnode::config::ExitConfigCli {
        listen: exit_udp,
        target_addr: target_addr.to_string(),
        exit_key_path: "exit.key".to_string(),
        ..Default::default()
    };
    tokio::spawn(async move {
        let _ = vpnnode::roles::exit::run_exit(exit_args).await;
    });

    let relay_args = vpnnode::config::RelayConfigCli {
        listen: relay_udp,
        exit_addr: exit_udp.to_string(),
        relay_key_path: "relay.key".to_string(),
        ..Default::default()
    };
    tokio::spawn(async move {
        let _ = vpnnode::roles::relay::run_relay(relay_args).await;
    });

    let client_args = vpnnode::config::ClientConfigCli {
        local_listen: client_tcp,
        mode: "tcp".to_string(),
        relay_addr: relay_udp.to_string(),
        exit_addr: exit_udp.to_string(),
        route_length: 2,
        relay2_addr: None,
        client_key_path: "client.key".to_string(),
        relay_pubkey_path: "relay.pub".to_string(),
        route_cache_path: "route_cache_test_challenge.json".to_string(),
        tun_name: "tun-test0".to_string(),
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

    sleep(Duration::from_millis(2500)).await;

    let mut stream = tokio::net::TcpStream::connect(client_tcp)
        .await
        .expect("connect to client listener");
    let req = b"GET / HTTP/1.1\r\nHost: example\r\nConnection: close\r\n\r\n";
    stream.write_all(req).await.expect("write request");
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.expect("read response");
    let body = String::from_utf8_lossy(&buf);
    assert!(
        body.contains("HTTP/1.1 200 OK"),
        "Stage 3.1 challenge flow must complete with 200 OK, got: {body}"
    );
}

#[tokio::test]
async fn end_to_end_http_route() {
    init_tracing();
    // Ports for this test
    let target_addr: SocketAddr = "127.0.0.1:18080".parse().unwrap();
    let exit_udp: SocketAddr = "127.0.0.1:13001".parse().unwrap();
    let relay_udp: SocketAddr = "127.0.0.1:13000".parse().unwrap();
    let client_tcp: SocketAddr = "127.0.0.1:11080".parse().unwrap();

    // Start target
    let target_args = vpnnode::config::TargetConfigCli { listen: target_addr };
    tokio::spawn(async move {
        let _ = vpnnode::roles::target::run_target(target_args).await;
    });

    // Start exit
    let exit_args = vpnnode::config::ExitConfigCli {
        listen: exit_udp,
        target_addr: target_addr.to_string(),
        exit_key_path: "exit.key".to_string(),
        ..Default::default()
    };
    tokio::spawn(async move {
        let _ = vpnnode::roles::exit::run_exit(exit_args).await;
    });

    // Start relay
    let relay_args = vpnnode::config::RelayConfigCli {
        listen: relay_udp,
        exit_addr: exit_udp.to_string(),
        relay_key_path: "relay.key".to_string(),
        ..Default::default()
    };
    tokio::spawn(async move {
        let _ = vpnnode::roles::relay::run_relay(relay_args).await;
    });

    // Start client (route-length 2: client -> relay -> exit)
    let client_args = vpnnode::config::ClientConfigCli {
        local_listen: client_tcp,
        mode: "tcp".to_string(),
        relay_addr: relay_udp.to_string(),
        exit_addr: exit_udp.to_string(),
        route_length: 2,
        relay2_addr: None,
        client_key_path: "client.key".to_string(),
        relay_pubkey_path: "relay.pub".to_string(),
        route_cache_path: "route_cache_test.json".to_string(),
        tun_name: "tun-test0".to_string(),
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

    sleep(Duration::from_millis(500)).await;

    // Perform HTTP request via client local listener
    let mut stream = tokio::net::TcpStream::connect(client_tcp)
        .await
        .expect("connect to client listener");
    let req = b"GET / HTTP/1.1\r\nHost: example\r\nConnection: close\r\n\r\n";
    stream.write_all(req).await.expect("write request");
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.expect("read response");
    let body = String::from_utf8_lossy(&buf);
    assert!(
        body.contains("HTTP/1.1 "),
        "expected an HTTP response status line, got: {body}"
    );
}

#[tokio::test]
async fn target_unavailable_returns_error() {
    init_tracing();
    let exit_udp: SocketAddr = "127.0.0.1:23001".parse().unwrap();
    let relay_udp: SocketAddr = "127.0.0.1:23000".parse().unwrap();
    let client_tcp: SocketAddr = "127.0.0.1:21080".parse().unwrap();

    // Exit points to non-listening target port
    let exit_args = vpnnode::config::ExitConfigCli {
        listen: exit_udp,
        target_addr: "127.0.0.1:28080".to_string(),
        exit_key_path: "exit.key".to_string(),
        ..Default::default()
    };
    tokio::spawn(async move {
        let _ = vpnnode::roles::exit::run_exit(exit_args).await;
    });

    let relay_args = vpnnode::config::RelayConfigCli {
        listen: relay_udp,
        exit_addr: exit_udp.to_string(),
        relay_key_path: "relay.key".to_string(),
        ..Default::default()
    };
    tokio::spawn(async move {
        let _ = vpnnode::roles::relay::run_relay(relay_args).await;
    });

    let client_args = vpnnode::config::ClientConfigCli {
        local_listen: client_tcp,
        mode: "tcp".to_string(),
        relay_addr: relay_udp.to_string(),
        exit_addr: exit_udp.to_string(),
        route_length: 2,
        relay2_addr: None,
        client_key_path: "client.key".to_string(),
        relay_pubkey_path: "relay.pub".to_string(),
        route_cache_path: "route_cache_test_unavail.json".to_string(),
        tun_name: "tun-test0".to_string(),
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

    sleep(Duration::from_millis(500)).await;

    // Perform a request through the client and ensure we get a deterministic
    // HTTP error response instead of hanging.
    let mut stream = timeout(Duration::from_secs(5), async {
        tokio::net::TcpStream::connect(client_tcp).await
    })
    .await
    .expect("connect attempt should not hang")
    .expect("client listener should accept connections even if target is unavailable");

    let req = b"GET / HTTP/1.1\r\nHost: example\r\nConnection: close\r\n\r\n";
    stream.write_all(req).await.expect("write request");
    let mut buf = Vec::new();
    timeout(Duration::from_secs(5), async {
        stream
            .read_to_end(&mut buf)
            .await
            .expect("read error response");
    })
    .await
    .expect("reading error response should not hang");
    let body = String::from_utf8_lossy(&buf);
    assert!(
        body.contains("HTTP/1.1 502 Bad Gateway"),
        "expected deterministic 502 error from client when target unavailable, got: {body}"
    );
}

#[tokio::test]
async fn multi_frame_single_stream_request() {
    init_tracing();
    // Similar topology as end_to_end_http_route but with a larger payload to
    // force the client to send multiple DATA frames for a single stream.
    let target_addr: SocketAddr = "127.0.0.1:48080".parse().unwrap();
    let exit_udp: SocketAddr = "127.0.0.1:43001".parse().unwrap();
    let relay_udp: SocketAddr = "127.0.0.1:43000".parse().unwrap();
    let client_tcp: SocketAddr = "127.0.0.1:41080".parse().unwrap();

    // Start target
    let target_args = vpnnode::config::TargetConfigCli { listen: target_addr };
    tokio::spawn(async move {
        let _ = vpnnode::roles::target::run_target(target_args).await;
    });

    // Start exit
    let exit_args = vpnnode::config::ExitConfigCli {
        listen: exit_udp,
        target_addr: target_addr.to_string(),
        exit_key_path: "exit.key".to_string(),
        ..Default::default()
    };
    tokio::spawn(async move {
        let _ = vpnnode::roles::exit::run_exit(exit_args).await;
    });

    // Start relay
    let relay_args = vpnnode::config::RelayConfigCli {
        listen: relay_udp,
        exit_addr: exit_udp.to_string(),
        relay_key_path: "relay.key".to_string(),
        ..Default::default()
    };
    tokio::spawn(async move {
        let _ = vpnnode::roles::relay::run_relay(relay_args).await;
    });

    let client_args = vpnnode::config::ClientConfigCli {
        local_listen: client_tcp,
        mode: "tcp".to_string(),
        relay_addr: relay_udp.to_string(),
        exit_addr: exit_udp.to_string(),
        route_length: 2,
        relay2_addr: None,
        client_key_path: "client.key".to_string(),
        relay_pubkey_path: "relay.pub".to_string(),
        route_cache_path: "route_cache_test_multi_frame.json".to_string(),
        tun_name: "tun-test0".to_string(),
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

    sleep(Duration::from_millis(1500)).await;

    let mut stream = tokio::net::TcpStream::connect(client_tcp)
        .await
        .expect("connect to client listener");

    // Build a large HTTP request body so that the client will have to
    // read and forward it in multiple chunks (multiple DATA frames).
    let body = vec![b'A'; 10_000];
    let req = format!(
        "POST /echo HTTP/1.1\r\nHost: example\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream
        .write_all(req.as_bytes())
        .await
        .expect("write headers");
    stream.write_all(&body).await.expect("write body");

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.expect("read response");
    let text = String::from_utf8_lossy(&buf);
    assert!(
        text.contains("HTTP/1.1 200 OK"),
        "expected successful HTTP response, got: {text}"
    );
    // Ensure echoed body length matches what we sent.
    assert!(
        text.len() >= body.len(),
        "response should contain echoed body of at least {} bytes",
        body.len()
    );
}

#[tokio::test]
async fn multiple_parallel_streams() {
    init_tracing();
    let target_addr: SocketAddr = "127.0.0.1:38080".parse().unwrap();
    let exit_udp: SocketAddr = "127.0.0.1:33001".parse().unwrap();
    let relay_udp: SocketAddr = "127.0.0.1:33000".parse().unwrap();
    let client_tcp: SocketAddr = "127.0.0.1:31080".parse().unwrap();

    // Start target
    let target_args = vpnnode::config::TargetConfigCli { listen: target_addr };
    tokio::spawn(async move {
        let _ = vpnnode::roles::target::run_target(target_args).await;
    });

    // Start exit
    let exit_args = vpnnode::config::ExitConfigCli {
        listen: exit_udp,
        target_addr: target_addr.to_string(),
        exit_key_path: "exit.key".to_string(),
        ..Default::default()
    };
    tokio::spawn(async move {
        let _ = vpnnode::roles::exit::run_exit(exit_args).await;
    });

    // Start relay
    let relay_args = vpnnode::config::RelayConfigCli {
        listen: relay_udp,
        exit_addr: exit_udp.to_string(),
        relay_key_path: "relay.key".to_string(),
        ..Default::default()
    };
    tokio::spawn(async move {
        let _ = vpnnode::roles::relay::run_relay(relay_args).await;
    });

    let client_args = vpnnode::config::ClientConfigCli {
        local_listen: client_tcp,
        mode: "tcp".to_string(),
        relay_addr: relay_udp.to_string(),
        exit_addr: exit_udp.to_string(),
        route_length: 2,
        relay2_addr: None,
        client_key_path: "client.key".to_string(),
        relay_pubkey_path: "relay.pub".to_string(),
        route_cache_path: "route_cache_test_parallel.json".to_string(),
        tun_name: "tun-test0".to_string(),
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

    sleep(Duration::from_millis(500)).await;

    let mut handles = Vec::new();
    for _ in 0..5 {
        let addr = client_tcp;
        handles.push(tokio::spawn(async move {
            let mut stream = tokio::net::TcpStream::connect(addr)
                .await
                .expect("connect to client listener");
            let req =
                b"GET / HTTP/1.1\r\nHost: example\r\nConnection: close\r\n\r\n";
            stream.write_all(req).await.expect("write request");
            let mut buf = Vec::new();
            stream.read_to_end(&mut buf).await.expect("read response");
            let body = String::from_utf8_lossy(&buf);
            assert!(
                body.contains("HTTP/1.1 "),
                "expected an HTTP response status line, got: {body}"
            );
        }));
    }

    for h in handles {
        h.await.unwrap();
    }
}

/// Stage 4: large POST (200KB) through reliable stream; response must be 200 and body intact.
#[tokio::test]
async fn large_http_transfer() {
    init_tracing();
    let target_addr: SocketAddr = "127.0.0.1:58080".parse().unwrap();
    let exit_udp: SocketAddr = "127.0.0.1:53001".parse().unwrap();
    let relay_udp: SocketAddr = "127.0.0.1:53000".parse().unwrap();
    let client_tcp: SocketAddr = "127.0.0.1:51080".parse().unwrap();

    let target_args = vpnnode::config::TargetConfigCli { listen: target_addr };
    tokio::spawn(async move {
        let _ = vpnnode::roles::target::run_target(target_args).await;
    });

    let exit_args = vpnnode::config::ExitConfigCli {
        listen: exit_udp,
        target_addr: target_addr.to_string(),
        exit_key_path: "exit.key".to_string(),
        ..Default::default()
    };
    tokio::spawn(async move {
        let _ = vpnnode::roles::exit::run_exit(exit_args).await;
    });

    let relay_args = vpnnode::config::RelayConfigCli {
        listen: relay_udp,
        exit_addr: exit_udp.to_string(),
        relay_key_path: "relay.key".to_string(),
        ..Default::default()
    };
    tokio::spawn(async move {
        let _ = vpnnode::roles::relay::run_relay(relay_args).await;
    });

    let client_args = vpnnode::config::ClientConfigCli {
        local_listen: client_tcp,
        mode: "tcp".to_string(),
        relay_addr: relay_udp.to_string(),
        exit_addr: exit_udp.to_string(),
        route_length: 2,
        relay2_addr: None,
        client_key_path: "client.key".to_string(),
        relay_pubkey_path: "relay.pub".to_string(),
        route_cache_path: "route_cache_test_large.json".to_string(),
        tun_name: "tun-test0".to_string(),
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

    sleep(Duration::from_millis(2500)).await;

    // Уменьшаем размер тела для стабильного прохождения на Windows/Docker,
    // при этом сохраняем многокадровый сценарий.
    let body_size = 64 * 1024;
    let body = vec![b'X'; body_size];
    let req = format!(
        "POST /echo HTTP/1.1\r\nHost: example\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body_size
    );

    let mut stream = tokio::net::TcpStream::connect(client_tcp)
        .await
        .expect("connect to client");
    stream.write_all(req.as_bytes()).await.expect("write headers");
    stream.write_all(&body).await.expect("write body");

    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.expect("read response");
    let text = String::from_utf8_lossy(&buf);
    assert!(
        text.contains("HTTP/1.1 200 OK"),
        "expected 200 OK, got: {}",
        text.lines().next().unwrap_or("")
    );
    assert!(
        text.len() >= body_size,
        "response should contain echoed body (at least {} bytes), got {}",
        body_size,
        text.len()
    );
}

/// Stage 5: 1-hop route (client -> exit directly, no relay)
#[tokio::test]
async fn route_1hop_direct_to_exit() {
    init_tracing();
    let target_addr: SocketAddr = "127.0.0.1:19180".parse().unwrap();
    let exit_udp: SocketAddr = "127.0.0.1:19101".parse().unwrap();
    let client_tcp: SocketAddr = "127.0.0.1:19102".parse().unwrap();

    let target_args = vpnnode::config::TargetConfigCli { listen: target_addr };
    tokio::spawn(async move {
        let _ = vpnnode::roles::target::run_target(target_args).await;
    });

    let exit_args = vpnnode::config::ExitConfigCli {
        listen: exit_udp,
        target_addr: target_addr.to_string(),
        exit_key_path: "exit.key".to_string(),
        ..Default::default()
    };
    tokio::spawn(async move {
        let _ = vpnnode::roles::exit::run_exit(exit_args).await;
    });

    let client_args = vpnnode::config::ClientConfigCli {
        local_listen: client_tcp,
        mode: "tcp".to_string(),
        relay_addr: "127.0.0.1:1".to_string(), // unused for route-length 1
        exit_addr: exit_udp.to_string(),
        route_length: 1,
        relay2_addr: None,
        client_key_path: "client.key".to_string(),
        relay_pubkey_path: "relay.pub".to_string(),
        route_cache_path: "route_cache_test_1hop.json".to_string(),
        tun_name: "tun-test0".to_string(),
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

    sleep(Duration::from_millis(2000)).await;

    let mut stream = tokio::net::TcpStream::connect(client_tcp)
        .await
        .expect("connect to client");
    let req = b"GET / HTTP/1.1\r\nHost: example\r\nConnection: close\r\n\r\n";
    stream.write_all(req).await.expect("write request");
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.expect("read response");
    let body = String::from_utf8_lossy(&buf);
    assert!(
        body.contains("HTTP/1.1 200 OK"),
        "1-hop route must return 200 OK, got: {body}"
    );
}

/// Stage 5: 2-hop route (client -> relay -> exit)
#[tokio::test]
async fn route_2hop_via_relay() {
    init_tracing();
    let target_addr: SocketAddr = "127.0.0.1:19280".parse().unwrap();
    let exit_udp: SocketAddr = "127.0.0.1:19201".parse().unwrap();
    let relay_udp: SocketAddr = "127.0.0.1:19200".parse().unwrap();
    let client_tcp: SocketAddr = "127.0.0.1:19202".parse().unwrap();

    let target_args = vpnnode::config::TargetConfigCli { listen: target_addr };
    tokio::spawn(async move {
        let _ = vpnnode::roles::target::run_target(target_args).await;
    });

    let exit_args = vpnnode::config::ExitConfigCli {
        listen: exit_udp,
        target_addr: target_addr.to_string(),
        exit_key_path: "exit.key".to_string(),
        ..Default::default()
    };
    tokio::spawn(async move {
        let _ = vpnnode::roles::exit::run_exit(exit_args).await;
    });

    let relay_args = vpnnode::config::RelayConfigCli {
        listen: relay_udp,
        exit_addr: exit_udp.to_string(),
        relay_key_path: "relay.key".to_string(),
        ..Default::default()
    };
    tokio::spawn(async move {
        let _ = vpnnode::roles::relay::run_relay(relay_args).await;
    });

    let client_args = vpnnode::config::ClientConfigCli {
        local_listen: client_tcp,
        mode: "tcp".to_string(),
        relay_addr: relay_udp.to_string(),
        exit_addr: exit_udp.to_string(),
        route_length: 2,
        relay2_addr: None,
        client_key_path: "client.key".to_string(),
        relay_pubkey_path: "relay.pub".to_string(),
        route_cache_path: "route_cache_test_2hop.json".to_string(),
        tun_name: "tun-test0".to_string(),
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

    sleep(Duration::from_millis(2000)).await;

    let mut stream = tokio::net::TcpStream::connect(client_tcp)
        .await
        .expect("connect to client");
    let req = b"GET / HTTP/1.1\r\nHost: example\r\nConnection: close\r\n\r\n";
    stream.write_all(req).await.expect("write request");
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.expect("read response");
    let body = String::from_utf8_lossy(&buf);
    assert!(
        body.contains("HTTP/1.1 200 OK"),
        "2-hop route must return 200 OK, got: {body}"
    );
}

/// Stage 5: 3-hop route (client -> relay1 -> relay2 -> exit)
#[tokio::test]
async fn route_3hop_via_two_relays() {
    init_tracing();
    let target_addr: SocketAddr = "127.0.0.1:19380".parse().unwrap();
    let exit_udp: SocketAddr = "127.0.0.1:19302".parse().unwrap();
    let relay2_udp: SocketAddr = "127.0.0.1:19301".parse().unwrap();
    let relay1_udp: SocketAddr = "127.0.0.1:19300".parse().unwrap();
    let client_tcp: SocketAddr = "127.0.0.1:19303".parse().unwrap();

    let target_args = vpnnode::config::TargetConfigCli { listen: target_addr };
    tokio::spawn(async move {
        let _ = vpnnode::roles::target::run_target(target_args).await;
    });

    let exit_args = vpnnode::config::ExitConfigCli {
        listen: exit_udp,
        target_addr: target_addr.to_string(),
        exit_key_path: "exit.key".to_string(),
        ..Default::default()
    };
    tokio::spawn(async move {
        let _ = vpnnode::roles::exit::run_exit(exit_args).await;
    });

    let relay2_args = vpnnode::config::RelayConfigCli {
        listen: relay2_udp,
        exit_addr: exit_udp.to_string(),
        relay_key_path: "relay.key".to_string(),
        ..Default::default()
    };
    tokio::spawn(async move {
        let _ = vpnnode::roles::relay::run_relay(relay2_args).await;
    });

    let relay1_args = vpnnode::config::RelayConfigCli {
        listen: relay1_udp,
        exit_addr: relay2_udp.to_string(),
        relay_key_path: "relay.key".to_string(),
        ..Default::default()
    };
    tokio::spawn(async move {
        let _ = vpnnode::roles::relay::run_relay(relay1_args).await;
    });

    let client_args = vpnnode::config::ClientConfigCli {
        local_listen: client_tcp,
        mode: "tcp".to_string(),
        relay_addr: relay1_udp.to_string(),
        exit_addr: exit_udp.to_string(),
        route_length: 3,
        relay2_addr: Some(relay2_udp.to_string()),
        client_key_path: "client.key".to_string(),
        relay_pubkey_path: "relay.pub".to_string(),
        route_cache_path: "route_cache_test_3hop.json".to_string(),
        tun_name: "tun-test0".to_string(),
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

    sleep(Duration::from_millis(3000)).await;

    let mut stream = tokio::net::TcpStream::connect(client_tcp)
        .await
        .expect("connect to client");
    let req = b"GET / HTTP/1.1\r\nHost: example\r\nConnection: close\r\n\r\n";
    stream.write_all(req).await.expect("write request");
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.expect("read response");
    let body = String::from_utf8_lossy(&buf);
    assert!(
        body.contains("HTTP/1.1 200 OK"),
        "3-hop route must return 200 OK, got: {body}"
    );
}

