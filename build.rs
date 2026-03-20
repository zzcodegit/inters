use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    // Build timestamp (ms since epoch) for logs/troubleshooting.
    let ts_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .to_string();
    println!("cargo:rustc-env=VPNNODE_BUILD_TS_MS={ts_ms}");

    // Best-effort git commit hash (works only if git + .git available).
    if let Ok(out) = Command::new("git").args(["rev-parse", "HEAD"]).output() {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !s.is_empty() {
                println!("cargo:rustc-env=VPNNODE_GIT_COMMIT={s}");
            }
        }
    }
}

