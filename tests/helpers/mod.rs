#![allow(dead_code)]

use std::net::SocketAddr;

use anyhow::{anyhow, bail, Context, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};

const DEFAULT_READY_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const DEFAULT_READ_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_PROBE_IO_TIMEOUT: Duration = Duration::from_secs(5);
const DEFAULT_PROBE_ATTEMPT_TIMEOUT: Duration = Duration::from_secs(6);
const DEFAULT_HTTP_READY_STATUSES: &[u16] = &[200];
const DEFAULT_READY_RETRY_DELAY: Duration = Duration::from_millis(100);
const DEFAULT_HTTP_PROBE_RETRY_DELAY: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Copy)]
pub struct HttpProbeOptions {
    pub ready_timeout: Duration,
    pub connect_timeout: Duration,
    pub probe_io_timeout: Duration,
    pub probe_attempt_timeout: Duration,
    pub retry_delay: Duration,
}

impl Default for HttpProbeOptions {
    fn default() -> Self {
        Self {
            ready_timeout: DEFAULT_READY_TIMEOUT,
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            probe_io_timeout: DEFAULT_PROBE_IO_TIMEOUT,
            probe_attempt_timeout: DEFAULT_PROBE_ATTEMPT_TIMEOUT,
            retry_delay: DEFAULT_HTTP_PROBE_RETRY_DELAY,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HttpProbeResult {
    pub response: Vec<u8>,
    pub status_code: u16,
}

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
                tokio::time::sleep(DEFAULT_READY_RETRY_DELAY).await;
            }
        }
    }
}

/// Wait until the client can complete an HTTP request end-to-end.
///
/// This helper sends a deterministic `GET /` probe and requires a usable
/// response status. By default, "usable" means `200 OK`.
pub async fn wait_http_ready(addr: SocketAddr) -> Result<()> {
    wait_http_status(addr, DEFAULT_HTTP_READY_STATUSES).await?;
    Ok(())
}

/// Wait until the client can complete an HTTP request end-to-end and returns
/// one of the explicitly accepted statuses.
pub async fn wait_http_status(addr: SocketAddr, accepted_statuses: &[u16]) -> Result<HttpProbeResult> {
    let probe = b"GET / HTTP/1.1\r\nHost: example\r\nConnection: close\r\n\r\n";
    wait_http_status_with_request(addr, probe, accepted_statuses).await
}

/// Wait until the client can complete a specific HTTP probe request end-to-end
/// and returns one of the explicitly accepted statuses.
pub async fn wait_http_status_with_request(
    addr: SocketAddr,
    request: &[u8],
    accepted_statuses: &[u16],
) -> Result<HttpProbeResult> {
    wait_http_status_with_request_options(
        addr,
        request,
        accepted_statuses,
        HttpProbeOptions::default(),
    )
    .await
}

pub async fn wait_http_status_with_request_options(
    addr: SocketAddr,
    request: &[u8],
    accepted_statuses: &[u16],
    options: HttpProbeOptions,
) -> Result<HttpProbeResult> {
    let start = tokio::time::Instant::now();
    let mut last_status = None;

    loop {
        if start.elapsed() >= options.ready_timeout {
            let accepted = accepted_statuses
                .iter()
                .map(u16::to_string)
                .collect::<Vec<_>>()
                .join(", ");
            match last_status {
                Some(status) => {
                    bail!("timeout waiting for HTTP-ready path at {addr}; accepted statuses=[{accepted}], last seen status={status}");
                }
                None => {
                    bail!("timeout waiting for HTTP-ready path at {addr}; accepted statuses=[{accepted}], no parseable HTTP status observed");
                }
            }
        }

        match probe_http_with_options(addr, request, options).await {
            Ok(result) => {
                last_status = Some(result.status_code);
                if accepted_statuses.contains(&result.status_code) {
                    return Ok(result);
                }
                tokio::time::sleep(options.retry_delay).await;
            }
            _ => {
                // ignore and retry until the global deadline
                tokio::time::sleep(options.retry_delay).await;
            }
        }
    }
}

/// Perform a single HTTP probe and return the raw response plus parsed status.
pub async fn probe_http(addr: SocketAddr, request: &[u8]) -> Result<HttpProbeResult> {
    probe_http_with_options(addr, request, HttpProbeOptions::default()).await
}

pub async fn probe_http_with_options(
    addr: SocketAddr,
    request: &[u8],
    options: HttpProbeOptions,
) -> Result<HttpProbeResult> {
    let attempt = async {
        let stream = timeout(options.connect_timeout, TcpStream::connect(addr))
            .await
            .map_err(|e| anyhow!("connect timeout/err: {e}"))??;

        let mut stream = stream;
        let resp = timeout(
            options.probe_io_timeout,
            async {
                stream.write_all(request).await?;
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

    let resp = timeout(options.probe_attempt_timeout, attempt)
        .await
        .map_err(|e| anyhow!("probe attempt timeout/err: {e}"))??;
    let status_code = extract_http_status_code(&resp)
        .ok_or_else(|| anyhow!("probe response did not contain a parseable HTTP status line"))?;
    Ok(HttpProbeResult {
        response: resp,
        status_code,
    })
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

