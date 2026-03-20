use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::info;

static DRAINING: AtomicBool = AtomicBool::new(false);
static DRAIN_DEADLINE_MS: AtomicU64 = AtomicU64::new(0);

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub fn is_draining() -> bool {
    DRAINING.load(Ordering::Relaxed)
}

pub fn drain_deadline_ms() -> u64 {
    DRAIN_DEADLINE_MS.load(Ordering::Relaxed)
}

pub fn deadline_reached() -> bool {
    let dl = drain_deadline_ms();
    dl != 0 && now_ms() >= dl
}

pub fn activate(timeout: Duration) {
    let already = DRAINING.swap(true, Ordering::Relaxed);
    if !already {
        let dl = now_ms().saturating_add(timeout.as_millis() as u64);
        DRAIN_DEADLINE_MS.store(dl, Ordering::Relaxed);
        info!(timeout_sec = timeout.as_secs(), deadline_ms = dl, "[drain] activated");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn activate_sets_draining_and_deadline() {
        activate(Duration::from_secs(1));
        assert!(is_draining());
        assert!(drain_deadline_ms() > 0);
    }
}

