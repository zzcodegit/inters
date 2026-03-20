use std::net::SocketAddr;

use anyhow::{anyhow, bail, Context, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

const DEFAULT_READY_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(30);

/// Wait until the client *local TCP listener* accepts connections.
///
/// This helper is intentionally "transport-only": it does NOT validate that
/// the VPN path is fully established, it only verifies that `TcpStream::connect`
/// succeeds.
#[allow(dead_code)]
pub async fn wait_tcp_listener(addr: SocketAddr) -> Result<()> {
    let start = tokio::time::Instant::now();
    loop {
        if start.elapsed() >= DEFAULT_READY_TIMEOUT {
            bail!("timeout waiting for client TCP listener at {addr}");
        }

        let connect = timeout(DEFAULT_CONNECT_TIMEOUT, TcpStream::connect(addr)).await;
        match connect {
            Ok(Ok(stream)) => {
                drop(stream);
                return Ok(());
            }
            _ => {
                // Listener may be up later; keep retrying within the global deadline.
                tokio::task::yield_now().await;
            }
        }
    }
}

/// Wait until the client can complete an HTTP request end-to-end.
///
/// This helper sends a deterministic `GET /` probe and requires the response
/// to contain a parseable `HTTP/<ver> <status>` status line.
pub async fn wait_http_ready(addr: SocketAddr) -> Result<()> {
    let start = tokio::time::Instant::now();
    let probe = b"GET / HTTP/1.1\r\nHost: example\r\nConnection: close\r\n\r\n";

    loop {
        if start.elapsed() >= DEFAULT_READY_TIMEOUT {
            bail!("timeout waiting for HTTP-ready path at {addr}");
        }

        let attempt = async {
            let stream = timeout(DEFAULT_CONNECT_TIMEOUT, TcpStream::connect(addr))
                .await
                .map_err(|e| anyhow!("connect timeout/err: {e}"))??;

            let mut stream = stream;
            let resp = timeout(
                Duration::from_secs(5),
                async {
                    stream.write_all(probe).await?;
                    stream.flush().await?;
                    let mut buf = Vec::new();
                    stream.read_to_end(&mut buf).await?;
                    Ok::<Vec<u8>, anyhow::Error>(buf)
                },
            )
            .await
            .map_err(|e| anyhow!("probe read/write timeout/err: {e}"))??;

            Ok::<Vec<u8>, anyhow::Error>(resp)
        };

        match timeout(Duration::from_secs(6), attempt).await {
            Ok(Ok(resp)) => {
                if extract_http_status_code(&resp).is_some() {
                    return Ok(());
                }
            }
            _ => {
                // ignore and retry until the global deadline
            }
        }
    }
}

/// Connect to a client TCP listener with a bounded timeout.
pub async fn connect_client(addr: SocketAddr) -> Result<TcpStream> {
    timeout(DEFAULT_CONNECT_TIMEOUT, TcpStream::connect(addr))
        .await
        .context("connect_client: TCP connect timeout")?
        .map_err(|e| anyhow!("connect_client: connect failed: {e}"))
}

/// Send a request over TCP and read the full response until EOF.
pub async fn send_and_receive(stream: &mut TcpStream, request: &[u8]) -> Result<Vec<u8>> {
    // For large transfers allow longer reads.
    let read_timeout = if request.len() >= 1_000_000 {
        Duration::from_secs(120)
    } else {
        DEFAULT_READ_TIMEOUT
    };

    timeout(Duration::from_secs(10), stream.write_all(request))
        .await
        .context("send_and_receive: write_all timeout")?
        .map_err(|e| anyhow!("send_and_receive: write_all failed: {e}"))?;

    let mut buf = Vec::new();
    timeout(read_timeout, stream.read_to_end(&mut buf))
        .await
        .context("send_and_receive: read_to_end timeout")?
        .map_err(|e| anyhow!("send_and_receive: read_to_end failed: {e}"))?;

    Ok(buf)
}

/// Convenience: parse `HTTP/... <code> ...` status code from a response.
pub fn extract_http_status_code(resp: &[u8]) -> Option<u16> {
    let marker = b"HTTP/";
    let pos = resp.windows(marker.len()).position(|w| w == marker)?;
    let mut i = pos + marker.len();
    while i < resp.len() && resp[i] != b' ' {
        i += 1;
    }
    if i + 3 > resp.len() {
        return None;
    }
    while i < resp.len() && resp[i].is_ascii_whitespace() {
        i += 1;
    }
    if i + 3 > resp.len() {
        return None;
    }
    let d0 = resp[i];
    let d1 = resp[i + 1];
    let d2 = resp[i + 2];
    if !d0.is_ascii_digit() || !d1.is_ascii_digit() || !d2.is_ascii_digit() {
        return None;
    }
    Some(((d0 - b'0') as u16) * 100 + ((d1 - b'0') as u16) * 10 + ((d2 - b'0') as u16))
}

