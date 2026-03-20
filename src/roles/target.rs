use crate::config::TargetConfigCli as TargetArgs;
use anyhow::Result;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tracing::{info, error, debug};

pub async fn run_target(args: TargetArgs) -> Result<()> {
    let listener = TcpListener::bind(args.listen).await?;
    info!(addr = %args.listen, "target service listening");

    loop {
        let (mut socket, addr) = listener.accept().await?;
        info!(%addr, "target accepted connection");
        tokio::spawn(async move {
            // Read full request (supports large POST bodies and multi-frame tunnel).
            let mut request = Vec::new();
            match socket.read_to_end(&mut request).await {
                Ok(0) => {
                    // connection closed by peer before sending anything
                    return;
                }
                Ok(_) => {
                    debug!(remote = %addr, bytes = request.len(), "target read request");
                    // Very simple HTTP echo: always respond 200 with body "OK" or echo payload
                    let response_body = if request.is_empty() {
                        b"OK".to_vec()
                    } else {
                        request
                    };
                    let resp = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        response_body.len()
                    );
                    if let Err(e) = socket.write_all(resp.as_bytes()).await {
                        error!(%e, "failed to write http headers");
                        return;
                    }
                    if let Err(e) = socket.write_all(&response_body).await {
                        error!(%e, "failed to write body");
                    }
                    // Close connection after single response so upstream can finish reading.
                    if let Err(e) = socket.shutdown().await {
                        error!(%e, "failed to shutdown target socket");
                    }
                }
                Err(e) => {
                    error!(%e, "target read error");
                }
            }
        });
    }
}
