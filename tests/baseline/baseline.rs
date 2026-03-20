#[path = "../helpers/mod.rs"]
mod helpers;
mod support;

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{anyhow, bail};
use tokio::sync::Barrier;

use support::{BaselineMode, LocalBaselineSpec};

fn local_spec(
    target_addr: Option<SocketAddr>,
    exit_udp: SocketAddr,
    relay1_udp: Option<SocketAddr>,
    relay2_udp: Option<SocketAddr>,
    client_tcp: SocketAddr,
    route_length: u8,
    exit_target_addr: String,
    route_cache_path: &'static str,
    tun_name: &'static str,
) -> LocalBaselineSpec<'static> {
    LocalBaselineSpec {
        target_addr,
        exit_udp,
        relay1_udp,
        relay2_udp,
        client_tcp,
        route_length,
        exit_target_addr,
        route_cache_path,
        tun_name,
    }
}

async fn request_response(client_tcp: SocketAddr, request: &[u8]) -> Vec<u8> {
    let mut stream = helpers::connect_client(client_tcp)
        .await
        .expect("connect to client");
    helpers::send_and_receive(&mut stream, request)
        .await
        .expect("request/response")
}

fn assert_status(resp: &[u8], expected: u16, message: &str) {
    let status = helpers::extract_http_status_code(resp).expect("HTTP status code must be present");
    assert_eq!(status, expected, "{message}");
}

#[tokio::test]
async fn baseline_handshake_challenge() {
    support::prepare_baseline(BaselineMode::Local);

    let spec = local_spec(
        Some("127.0.0.1:16180".parse().unwrap()),
        "127.0.0.1:16101".parse().unwrap(),
        Some("127.0.0.1:16100".parse().unwrap()),
        None,
        "127.0.0.1:16102".parse().unwrap(),
        2,
        "127.0.0.1:16180".to_string(),
        "route_cache_test_challenge.json",
        "tun-baseline-handshake",
    );
    support::spawn_local_stack(&spec);

    helpers::wait_http_ready(spec.client_tcp)
        .await
        .expect("client HTTP path must become ready");
    let resp = request_response(spec.client_tcp, &support::http_get_request("example")).await;
    assert_status(&resp, 200, "handshake challenge must return 200 OK");
}

#[tokio::test]
async fn baseline_route_1hop_client_to_exit() {
    support::prepare_baseline(BaselineMode::Local);

    let spec = local_spec(
        Some("127.0.0.1:16280".parse().unwrap()),
        "127.0.0.1:16201".parse().unwrap(),
        None,
        None,
        "127.0.0.1:16202".parse().unwrap(),
        1,
        "127.0.0.1:16280".to_string(),
        "route_cache_test_1hop.json",
        "tun-baseline-1hop",
    );
    support::spawn_local_stack(&spec);

    helpers::wait_http_ready(spec.client_tcp)
        .await
        .expect("client HTTP path must become ready");
    let resp = request_response(spec.client_tcp, &support::http_get_request("example")).await;
    assert_status(&resp, 200, "1-hop route must return 200 OK");
}

#[tokio::test]
async fn baseline_route_2hop_client_via_relay_to_exit() {
    support::prepare_baseline(BaselineMode::Local);

    let spec = local_spec(
        Some("127.0.0.1:16380".parse().unwrap()),
        "127.0.0.1:16301".parse().unwrap(),
        Some("127.0.0.1:16300".parse().unwrap()),
        None,
        "127.0.0.1:16302".parse().unwrap(),
        2,
        "127.0.0.1:16380".to_string(),
        "route_cache_test_2hop.json",
        "tun-baseline-2hop",
    );
    support::spawn_local_stack(&spec);

    helpers::wait_http_ready(spec.client_tcp)
        .await
        .expect("client HTTP path must become ready");
    let resp = request_response(spec.client_tcp, &support::http_get_request("example")).await;
    assert_status(&resp, 200, "2-hop route must return 200 OK");
}

#[tokio::test]
async fn baseline_route_3hop_via_two_relays_to_exit() {
    support::prepare_baseline(BaselineMode::Local);

    let spec = local_spec(
        Some("127.0.0.1:16480".parse().unwrap()),
        "127.0.0.1:16402".parse().unwrap(),
        Some("127.0.0.1:16400".parse().unwrap()),
        Some("127.0.0.1:16401".parse().unwrap()),
        "127.0.0.1:16403".parse().unwrap(),
        3,
        "127.0.0.1:16480".to_string(),
        "route_cache_test_3hop.json",
        "tun-baseline-3hop",
    );
    support::spawn_local_stack(&spec);

    helpers::wait_http_ready(spec.client_tcp)
        .await
        .expect("client HTTP path must become ready");
    let resp = request_response(spec.client_tcp, &support::http_get_request("example")).await;
    assert_status(&resp, 200, "3-hop route must return 200 OK");
}

#[tokio::test]
async fn baseline_large_transfer_ge_5mb() {
    support::prepare_baseline(BaselineMode::Local);

    let spec = local_spec(
        Some("127.0.0.1:16580".parse().unwrap()),
        "127.0.0.1:16501".parse().unwrap(),
        Some("127.0.0.1:16500".parse().unwrap()),
        None,
        "127.0.0.1:16502".parse().unwrap(),
        2,
        "127.0.0.1:16580".to_string(),
        "route_cache_test_large.json",
        "tun-baseline-large",
    );
    support::spawn_local_stack(&spec);

    helpers::wait_http_ready(spec.client_tcp)
        .await
        .expect("client HTTP path must become ready");

    let body_size: usize = 5 * 1024 * 1024;
    let body = vec![b'X'; body_size];
    let header = format!(
        "POST /echo HTTP/1.1\r\nHost: example\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body_size
    );
    let mut request = Vec::with_capacity(header.len() + body.len());
    request.extend_from_slice(header.as_bytes());
    request.extend_from_slice(&body);

    let resp = request_response(spec.client_tcp, &request).await;
    assert_status(&resp, 200, "large transfer must return 200 OK");

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
    support::prepare_baseline(BaselineMode::Local);

    let spec = local_spec(
        Some("127.0.0.1:16680".parse().unwrap()),
        "127.0.0.1:16601".parse().unwrap(),
        Some("127.0.0.1:16600".parse().unwrap()),
        None,
        "127.0.0.1:16602".parse().unwrap(),
        2,
        "127.0.0.1:16680".to_string(),
        "route_cache_test_parallel.json",
        "tun-baseline-parallel",
    );
    support::spawn_local_stack(&spec);

    helpers::wait_http_ready(spec.client_tcp)
        .await
        .expect("client HTTP path must become ready");

    let n = 5usize;
    let barrier = Arc::new(Barrier::new(n));
    let mut handles = Vec::with_capacity(n);
    for _ in 0..n {
        let barrier = barrier.clone();
        let client_tcp = spec.client_tcp;
        let request = support::http_get_request("example");
        handles.push(tokio::spawn(async move {
            barrier.wait().await;
            let resp = request_response(client_tcp, &request).await;
            let status = helpers::extract_http_status_code(&resp)
                .ok_or_else(|| anyhow!("missing http status code in response"))?;
            if status != 200 {
                bail!("expected 200 OK, got {status}");
            }
            Ok::<(), anyhow::Error>(())
        }));
    }

    for handle in handles {
        let result = handle.await.expect("parallel stream task should not panic");
        result.expect("parallel stream task must return Ok");
    }
}

#[tokio::test]
async fn baseline_target_unavailable_returns_502() {
    support::prepare_baseline(BaselineMode::Local);

    let spec = local_spec(
        None,
        "127.0.0.1:16701".parse().unwrap(),
        Some("127.0.0.1:16700".parse().unwrap()),
        None,
        "127.0.0.1:16702".parse().unwrap(),
        2,
        "127.0.0.1:28999".to_string(),
        "route_cache_test_unavail.json",
        "tun-baseline-unavail",
    );
    support::spawn_local_stack(&spec);

    helpers::wait_http_status(spec.client_tcp, &[502])
        .await
        .expect("client must expose deterministic 502 path");
    let resp = request_response(spec.client_tcp, &support::http_get_request("example")).await;
    assert_status(&resp, 502, "target unavailable must return 502 Bad Gateway");
}

#[tokio::test]
async fn baseline_small_response_returns_200_and_completes() {
    support::prepare_baseline(BaselineMode::Local);

    let spec = local_spec(
        Some("127.0.0.1:16880".parse().unwrap()),
        "127.0.0.1:16801".parse().unwrap(),
        Some("127.0.0.1:16800".parse().unwrap()),
        None,
        "127.0.0.1:16802".parse().unwrap(),
        2,
        "127.0.0.1:16880".to_string(),
        "route_cache_test_small_resp.json",
        "tun-baseline-small",
    );
    support::spawn_local_stack(&spec);

    helpers::wait_http_ready(spec.client_tcp)
        .await
        .expect("client HTTP path must become ready");
    let resp = request_response(spec.client_tcp, &support::http_get_request("example")).await;

    assert_status(&resp, 200, "small response must return 200 OK");
    assert!(!resp.is_empty(), "small response must complete with non-empty payload");
}

#[tokio::test]
async fn baseline_response_completion_respects_non_pathological_growth() {
    support::prepare_baseline(BaselineMode::Local);

    let spec = local_spec(
        Some("127.0.0.1:16980".parse().unwrap()),
        "127.0.0.1:16901".parse().unwrap(),
        Some("127.0.0.1:16900".parse().unwrap()),
        None,
        "127.0.0.1:16902".parse().unwrap(),
        2,
        "127.0.0.1:16980".to_string(),
        "route_cache_test_growth_guard.json",
        "tun-baseline-growth",
    );
    support::spawn_local_stack(&spec);

    helpers::wait_http_ready(spec.client_tcp)
        .await
        .expect("client HTTP path must become ready");

    let body = vec![b'Z'; 32 * 1024];
    let header = format!(
        "POST /echo HTTP/1.1\r\nHost: example\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    );
    let mut req = Vec::with_capacity(header.len() + body.len());
    req.extend_from_slice(header.as_bytes());
    req.extend_from_slice(&body);

    let resp = request_response(spec.client_tcp, &req).await;
    assert_status(&resp, 200, "non-pathological response must return 200 OK");

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
