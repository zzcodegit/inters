use crate::ops::drain;
use std::time::Duration;

/// Spawn a background listener for SIGINT/SIGTERM.
///
/// On trigger: activates drain mode (bounded by timeout).
pub fn spawn_signal_listener(timeout: Duration) {
    tokio::spawn(async move {
        // SIGINT (Ctrl+C) works on all platforms.
        let ctrl_c = tokio::signal::ctrl_c();

        #[cfg(unix)]
        let mut term = {
            use tokio::signal::unix::{signal, SignalKind};
            signal(SignalKind::terminate()).ok()
        };

        #[cfg(unix)]
        tokio::select! {
            _ = ctrl_c => {
                drain::activate(timeout);
            }
            _ = async {
                if let Some(ref mut s) = term {
                    s.recv().await;
                } else {
                    std::future::pending::<()>().await;
                }
            } => {
                drain::activate(timeout);
            }
        }

        #[cfg(not(unix))]
        {
            let _ = ctrl_c.await;
            drain::activate(timeout);
        }
    });
}

