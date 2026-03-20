use crate::crypto::{open, seal, AeadKey};
use crate::protocol::{decode, encode, TunnelMessage};
use anyhow::Result;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Simple sliding-window replay protection for a single session.
///
/// We track the highest sequence number seen so far and a bitmap of the last
/// WINDOW_SIZE sequence numbers. This is intentionally minimal but provides
/// clear, testable anti-replay behavior for Stage 3.
const REPLAY_WINDOW_SIZE: u64 = 64;

#[derive(Debug)]
struct RecvReplayState {
    highest: u64,
    bitmap: u64,
}

impl RecvReplayState {
    fn new() -> Self {
        Self {
            highest: 0,
            bitmap: 0,
        }
    }

    /// Returns Ok(()) if the given sequence number is acceptable and updates
    /// internal state, or Err if it is considered a replay / too old.
    fn check_and_update(&mut self, seq: u64) -> Result<()> {
        if seq == 0 {
            anyhow::bail!("seq 0 is reserved / invalid");
        }
        if self.highest == 0 {
            // First packet.
            self.highest = seq;
            self.bitmap = 1;
            return Ok(());
        }

        if seq > self.highest {
            let diff = seq - self.highest;
            if diff >= REPLAY_WINDOW_SIZE {
                // Jump forward beyond the window: drop all history.
                self.bitmap = 1;
            } else {
                // Shift bitmap forward and mark newest bit.
                self.bitmap <<= diff;
                self.bitmap |= 1;
            }
            self.highest = seq;
            return Ok(());
        }

        // seq <= highest: check whether it is still within the window.
        let behind = self.highest - seq;
        if behind >= REPLAY_WINDOW_SIZE {
            anyhow::bail!("packet too old for replay window");
        }
        let mask = 1u64 << behind;
        if self.bitmap & mask != 0 {
            anyhow::bail!("duplicate packet detected");
        }
        // Mark as seen.
        self.bitmap |= mask;
        Ok(())
    }
}

#[derive(Clone)]
pub struct SessionCrypto {
    pub key: AeadKey,
    send_seq: Arc<AtomicU64>,
    recv_replay: Arc<Mutex<RecvReplayState>>,
}

impl SessionCrypto {
    pub fn new(key: AeadKey) -> Self {
        Self {
            key,
            send_seq: Arc::new(AtomicU64::new(1)),
            recv_replay: Arc::new(Mutex::new(RecvReplayState::new())),
        }
    }

    pub fn next_seq(&self) -> u64 {
        self.send_seq.fetch_add(1, Ordering::Relaxed)
    }

    /// Encode and encrypt a message for sending over the wire.
    /// Wire format: [u64_be seq][ciphertext]
    pub fn seal_message(&self, mut msg: TunnelMessage) -> Result<Vec<u8>> {
        let seq = self.next_seq();
        msg.header.seq = seq;
        let encoded = encode(&msg)?;
        let ct = seal(&self.key, seq, &encoded)?;
        let mut out = Vec::with_capacity(8 + ct.len());
        out.extend_from_slice(&seq.to_be_bytes());
        out.extend_from_slice(&ct);
        Ok(out)
    }

    /// Decrypt and decode a received message from wire format, enforcing
    /// per-session replay protection.
    pub fn open_message(&self, data: &[u8]) -> Result<TunnelMessage> {
        if data.len() < 8 {
            anyhow::bail!("message too short");
        }
        let mut seq_bytes = [0u8; 8];
        seq_bytes.copy_from_slice(&data[..8]);
        let seq = u64::from_be_bytes(seq_bytes);
        {
            let mut guard = self
                .recv_replay
                .lock()
                .expect("recv_replay mutex poisoned");
            guard.check_and_update(seq)?;
        }

        let ct = &data[8..];
        let pt = open(&self.key, seq, ct)?;
        let msg = decode(&pt)?;
        Ok(msg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{derive_aead_key, generate_x25519_keypair};
    use crate::protocol::{Header, MsgType};

    #[test]
    fn session_crypto_roundtrip() {
        let kp1 = generate_x25519_keypair();
        let kp2 = generate_x25519_keypair();
        let k1 = derive_aead_key(&kp1.secret, &kp2.public);
        let k2 = derive_aead_key(&kp2.secret, &kp1.public);

        let s1 = SessionCrypto::new(k1);
        let s2 = SessionCrypto::new(k2);

        let msg = TunnelMessage {
            header: Header {
                version: 1,
                msg_type: MsgType::Data,
                session_id: 1,
                stream_id: 1,
                seq: 0,
                hop_index: 0,
            },
            payload: b"hello".to_vec(),
        };

        let ct = s1.seal_message(msg.clone()).unwrap();
        let dec = s2.open_message(&ct).unwrap();
        assert_eq!(msg.header.msg_type, dec.header.msg_type);
        assert_eq!(msg.payload, dec.payload);
    }

    #[test]
    fn replay_window_rejects_duplicates_and_old_packets() {
        let kp1 = generate_x25519_keypair();
        let kp2 = generate_x25519_keypair();
        let k1 = derive_aead_key(&kp1.secret, &kp2.public);
        let k2 = derive_aead_key(&kp2.secret, &kp1.public);

        let s1 = SessionCrypto::new(k1);
        let s2 = SessionCrypto::new(k2);

        let base_msg = TunnelMessage {
            header: Header {
                version: 1,
                msg_type: MsgType::Data,
                session_id: 1,
                stream_id: 1,
                seq: 0,
                hop_index: 0,
            },
            payload: b"hello".to_vec(),
        };

        // Send a few messages to advance the window.
        let mut cts = Vec::new();
        for _ in 0..5 {
            let ct = s1.seal_message(base_msg.clone()).unwrap();
            cts.push(ct);
        }

        // All first-time deliveries must succeed.
        for ct in &cts {
            let _ = s2.open_message(ct).unwrap();
        }

        // Replaying the last packet must fail.
        let last = cts.last().unwrap();
        let err = s2.open_message(last).unwrap_err();
        assert!(
            err.to_string().contains("duplicate") || err.to_string().contains("old"),
            "expected replay-related error, got: {err}"
        );
    }
}

