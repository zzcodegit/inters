//! Stage 3.1: Stateless cookie for handshake anti-amplification.
//!
//! Cookie = HMAC-SHA256(server_secret, client_ip || client_pubkey || client_nonce || timestamp_bucket).
//! timestamp_bucket = current_time_secs / 30. No server state; verification recomputes and compares.

use anyhow::Result;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};

const COOKIE_LEN: usize = 32;
const BUCKET_SECS: u64 = 30;

type HmacSha256 = Hmac<Sha256>;

/// Generate a cookie for the given client. Caller must send this in HandshakeChallenge.
pub fn generate_cookie(
    secret: &[u8],
    client_addr: SocketAddr,
    client_pubkey: &[u8; 32],
    client_nonce: &[u8; 32],
) -> Result<[u8; COOKIE_LEN]> {
    let bucket = current_timestamp_bucket()?;
    compute_cookie_for_bucket(secret, client_addr, client_pubkey, client_nonce, bucket)
}

/// Verify a cookie from the client. Returns true only if cookie matches and is not expired.
pub fn verify_cookie(
    secret: &[u8],
    cookie: &[u8; COOKIE_LEN],
    client_addr: SocketAddr,
    client_pubkey: &[u8; 32],
    client_nonce: &[u8; 32],
) -> Result<bool> {
    let now_bucket = current_timestamp_bucket()?;
    for b in [now_bucket, now_bucket.saturating_sub(1)] {
        let expected =
            compute_cookie_for_bucket(secret, client_addr, client_pubkey, client_nonce, b)?;
        if expected == *cookie {
            return Ok(true);
        }
    }
    Ok(false)
}

fn current_timestamp_bucket() -> Result<u64> {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| anyhow::anyhow!("system time: {}", e))?
        .as_secs();
    Ok(secs / BUCKET_SECS)
}

fn compute_cookie_for_bucket(
    secret: &[u8],
    client_addr: SocketAddr,
    client_pubkey: &[u8; 32],
    client_nonce: &[u8; 32],
    timestamp_bucket: u64,
) -> Result<[u8; COOKIE_LEN]> {
    let mut mac =
        HmacSha256::new_from_slice(secret).map_err(|e| anyhow::anyhow!("hmac key: {}", e))?;
    mac.update(client_addr.ip().to_string().as_bytes());
    mac.update(&client_addr.port().to_be_bytes());
    mac.update(client_pubkey);
    mac.update(client_nonce);
    mac.update(&timestamp_bucket.to_be_bytes());
    let result = mac.finalize();
    let mut out = [0u8; COOKIE_LEN];
    out.copy_from_slice(result.into_bytes().as_slice());
    Ok(out)
}

/// Expose for tests: compute cookie with explicit bucket (e.g. to simulate expiry).
/// Simple per-IP rate limiter: max N handshake attempts per second.
pub struct HandshakeRateLimiter {
    max_per_sec: u32,
    /// (count, window_start_secs since epoch)
    per_ip: std::collections::HashMap<std::net::IpAddr, (u32, u64)>,
}

impl HandshakeRateLimiter {
    pub fn new(max_per_sec: u32) -> Self {
        Self {
            max_per_sec,
            per_ip: std::collections::HashMap::new(),
        }
    }

    /// Returns true if the client at this IP is allowed to proceed (and consumes one token).
    pub fn allow(&mut self, ip: std::net::IpAddr) -> bool {
        let now_secs = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(d) => d.as_secs(),
            Err(_) => return false,
        };
        let entry = self.per_ip.entry(ip).or_insert((0, now_secs));
        if now_secs.saturating_sub(entry.1) >= 1 {
            *entry = (0, now_secs);
        }
        if entry.0 >= self.max_per_sec {
            return false;
        }
        entry.0 += 1;
        true
    }
}

#[cfg(test)]
pub fn generate_cookie_for_bucket(
    secret: &[u8],
    client_addr: SocketAddr,
    client_pubkey: &[u8; 32],
    client_nonce: &[u8; 32],
    timestamp_bucket: u64,
) -> Result<[u8; COOKIE_LEN]> {
    compute_cookie_for_bucket(secret, client_addr, client_pubkey, client_nonce, timestamp_bucket)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::IpAddr;

    fn addr(ip: &str, port: u16) -> SocketAddr {
        SocketAddr::new(ip.parse::<IpAddr>().unwrap(), port)
    }

    #[test]
    fn cookie_roundtrip_generate_verify_ok() {
        let secret = b"server-secret-key-32-bytes!!!!!!";
        let client_addr = addr("192.168.1.1", 54321);
        let client_pubkey = [1u8; 32];
        let client_nonce = [2u8; 32];

        let cookie = generate_cookie(secret, client_addr, &client_pubkey, &client_nonce).unwrap();
        let ok = verify_cookie(secret, &cookie, client_addr, &client_pubkey, &client_nonce).unwrap();
        assert!(ok, "valid cookie must verify");
    }

    #[test]
    fn invalid_cookie_rejected() {
        let secret = b"server-secret-key-32-bytes!!!!!!";
        let client_addr = addr("192.168.1.1", 54321);
        let client_pubkey = [1u8; 32];
        let client_nonce = [2u8; 32];

        let wrong_cookie = [0u8; 32];
        let ok =
            verify_cookie(secret, &wrong_cookie, client_addr, &client_pubkey, &client_nonce)
                .unwrap();
        assert!(!ok, "wrong cookie must be rejected");
    }

    #[test]
    fn wrong_ip_reject() {
        let secret = b"server-secret-key-32-bytes!!!!!!";
        let addr1 = addr("192.168.1.1", 54321);
        let addr2 = addr("192.168.1.2", 54321);
        let client_pubkey = [1u8; 32];
        let client_nonce = [2u8; 32];

        let cookie = generate_cookie(secret, addr1, &client_pubkey, &client_nonce).unwrap();
        let ok = verify_cookie(secret, &cookie, addr2, &client_pubkey, &client_nonce).unwrap();
        assert!(!ok, "cookie from different IP must be rejected");
    }

    #[test]
    fn wrong_nonce_reject() {
        let secret = b"server-secret-key-32-bytes!!!!!!";
        let client_addr = addr("192.168.1.1", 54321);
        let client_pubkey = [1u8; 32];
        let nonce1 = [2u8; 32];
        let mut nonce2 = [2u8; 32];
        nonce2[0] = 3;

        let cookie =
            generate_cookie(secret, client_addr, &client_pubkey, &nonce1).unwrap();
        let ok = verify_cookie(secret, &cookie, client_addr, &client_pubkey, &nonce2).unwrap();
        assert!(!ok, "cookie with different nonce must be rejected");
    }

    #[test]
    fn expired_timestamp_reject() {
        let secret = b"server-secret-key-32-bytes!!!!!!";
        let client_addr = addr("192.168.1.1", 54321);
        let client_pubkey = [1u8; 32];
        let client_nonce = [2u8; 32];

        let bucket_old = 0u64; // very old
        let cookie = generate_cookie_for_bucket(
            secret,
            client_addr,
            &client_pubkey,
            &client_nonce,
            bucket_old,
        )
        .unwrap();

        // Verify uses current bucket and current-1; bucket_old is far in the past so verify fails
        let ok = verify_cookie(secret, &cookie, client_addr, &client_pubkey, &client_nonce).unwrap();
        assert!(!ok, "expired cookie must be rejected");
    }
}
