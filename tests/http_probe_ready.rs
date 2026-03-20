#[path = "helpers/mod.rs"]
mod helpers;

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use anyhow::Context;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio::time::Duration;

async fn spawn_status_server(statuses: Vec<u16>) -> anyhow::Result<std::net::SocketAddr> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .context("bind status server")?;
    let addr = listener.local_addr().context("local addr")?;
    let statuses = Arc::new(statuses);
    let counter = Arc::new(AtomicUsize::new(0));

    tokio::spawn({
        let statuses = Arc::clone(&statuses);
        let counter = Arc::clone(&counter);
        async move {
            loop {
                let (mut socket, _) = match listener.accept().await {
                    Ok(pair) => pair,
                    Err(_) => return,
                };
                let statuses = Arc::clone(&statuses);
                let counter = Arc::clone(&counter);
                tokio::spawn(async move {
                    let mut buf = [0_u8; 1024];
                    let _ = socket.read(&mut buf).await;
                    let idx = counter.fetch_add(1, Ordering::SeqCst);
                    let status = statuses
                        .get(idx)
                        .copied()
                        .or_else(|| statuses.last().copied())
                        .unwrap_or(200);
                    let reason = match status {
                        200 => "OK",
                        504 => "Gateway Timeout",
                        _ => "Test",
                    };
                    let body = format!("status={status}\n");
                    let response = format!(
                        "HTTP/1.1 {status} {reason}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                        body.len(),
                        body
                    );
                    let _ = socket.write_all(response.as_bytes()).await;
                    let _ = socket.shutdown().await;
                });
            }
        }
    });

    Ok(addr)
}

#[tokio::test]
async fn wait_http_ready_rejects_504_until_200_is_observed() -> anyhow::Result<()> {
    let addr = spawn_status_server(vec![504, 200]).await?;
    let request = b"GET / HTTP/1.1\r\nHost: example\r\nConnection: close\r\n\r\n";
    let probe = helpers::wait_http_status_with_request_options(
        addr,
        request,
        &[200],
        helpers::HttpProbeOptions {
            ready_timeout: Duration::from_secs(2),
            connect_timeout: Duration::from_millis(250),
            probe_io_timeout: Duration::from_millis(250),
            probe_attempt_timeout: Duration::from_millis(250),
            retry_delay: Duration::from_millis(50),
        },
    )
    .await?;

    assert_eq!(probe.status_code, 200);
    Ok(())
}
