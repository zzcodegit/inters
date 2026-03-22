use crate::crypto::{open, seal, AeadKey};
use crate::protocol::{decode, encode, TunnelMessage};
use anyhow::Result;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Simple sliding-window replay protection for a single session.
///
/// We track the highest sequence number seen so far and a bitmap of the last
/// WINDOW_SIZE sequence numbers. The window is intentionally wider than the
/// original Stage 3 version so delayed packets inside bounded chaos/reorder
/// bursts are still accepted while obviously stale traffic is rejected.
const REPLAY_WINDOW_WORDS: usize = 8;
const REPLAY_WINDOW_SIZE: u64 = (REPLAY_WINDOW_WORDS as u64) * 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReplayRejectKind {
    InvalidSeq,
    TooOld,
    Duplicate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ReplayReject {
    kind: ReplayRejectKind,
    seq: u64,
    highest: u64,
    behind: Option<u64>,
    window: u64,
}

impl ReplayReject {
    fn invalid_seq(seq: u64, highest: u64) -> Self {
        Self {
            kind: ReplayRejectKind::InvalidSeq,
            seq,
            highest,
            behind: None,
            window: REPLAY_WINDOW_SIZE,
        }
    }

    fn too_old(seq: u64, highest: u64, behind: u64) -> Self {
        Self {
            kind: ReplayRejectKind::TooOld,
            seq,
            highest,
            behind: Some(behind),
            window: REPLAY_WINDOW_SIZE,
        }
    }

    fn duplicate(seq: u64, highest: u64, behind: u64) -> Self {
        Self {
            kind: ReplayRejectKind::Duplicate,
            seq,
            highest,
            behind: Some(behind),
            window: REPLAY_WINDOW_SIZE,
        }
    }
}

impl fmt::Display for ReplayReject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            ReplayRejectKind::InvalidSeq => {
                write!(f, "seq 0 is reserved / invalid")
            }
            ReplayRejectKind::TooOld => write!(
                f,
                "packet too old for replay window: seq={} highest={} behind={} window={}",
                self.seq,
                self.highest,
                self.behind.unwrap_or_default(),
                self.window
            ),
            ReplayRejectKind::Duplicate => write!(
                f,
                "duplicate packet detected: seq={} highest={} behind={} window={}",
                self.seq,
                self.highest,
                self.behind.unwrap_or_default(),
                self.window
            ),
        }
    }
}

impl std::error::Error for ReplayReject {}

#[derive(Debug)]
struct RecvReplayState {
    highest: u64,
    bitmap: [u64; REPLAY_WINDOW_WORDS],
}

impl RecvReplayState {
    fn new() -> Self {
        Self {
            highest: 0,
            bitmap: [0; REPLAY_WINDOW_WORDS],
        }
    }

    fn clear_history(&mut self) {
        self.bitmap = [0; REPLAY_WINDOW_WORDS];
    }

    fn mark_seen(&mut self, behind: u64) {
        let word_idx = (behind / 64) as usize;
        let bit_idx = (behind % 64) as usize;
        self.bitmap[word_idx] |= 1u64 << bit_idx;
    }

    fn is_seen(&self, behind: u64) -> bool {
        let word_idx = (behind / 64) as usize;
        let bit_idx = (behind % 64) as usize;
        (self.bitmap[word_idx] & (1u64 << bit_idx)) != 0
    }

    fn shift_forward(&mut self, diff: u64) {
        if diff >= REPLAY_WINDOW_SIZE {
            self.clear_history();
            return;
        }
        let word_shift = (diff / 64) as usize;
        let bit_shift = (diff % 64) as usize;
        let previous = self.bitmap;
        let mut shifted = [0u64; REPLAY_WINDOW_WORDS];
        for dest in (0..REPLAY_WINDOW_WORDS).rev() {
            if dest < word_shift {
                continue;
            }
            let src = dest - word_shift;
            shifted[dest] |= previous[src] << bit_shift;
            if bit_shift > 0 && src > 0 {
                shifted[dest] |= previous[src - 1] >> (64 - bit_shift);
            }
        }
        self.bitmap = shifted;
    }

    /// Returns Ok(()) if the given sequence number is acceptable and updates
    /// internal state, or Err if it is considered a replay / too old.
    fn check_and_update(&mut self, seq: u64) -> std::result::Result<(), ReplayReject> {
        if seq == 0 {
            return Err(ReplayReject::invalid_seq(seq, self.highest));
        }
        if self.highest == 0 {
            // First packet.
            self.highest = seq;
            self.clear_history();
            self.mark_seen(0);
            return Ok(());
        }

        if seq > self.highest {
            let diff = seq - self.highest;
            self.shift_forward(diff);
            self.highest = seq;
            self.mark_seen(0);
            return Ok(());
        }

        // seq <= highest: check whether it is still within the window.
        let behind = self.highest - seq;
        if behind >= REPLAY_WINDOW_SIZE {
            return Err(ReplayReject::too_old(seq, self.highest, behind));
        }
        if self.is_seen(behind) {
            return Err(ReplayReject::duplicate(seq, self.highest, behind));
        }
        // Mark as seen.
        self.mark_seen(behind);
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
            let mut guard = self.recv_replay.lock().expect("recv_replay mutex poisoned");
            guard.check_and_update(seq).map_err(anyhow::Error::new)?;
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

    #[test]
    fn replay_window_accepts_reordered_packet_within_expanded_window() {
        let mut state = RecvReplayState::new();
        for seq in 1..=400 {
            if seq == 120 {
                continue;
            }
            state.check_and_update(seq).unwrap();
        }

        state.check_and_update(120).unwrap();

        let err = state.check_and_update(120).unwrap_err();
        assert_eq!(err.kind, ReplayRejectKind::Duplicate);
        assert_eq!(err.behind, Some(280));
    }

    #[test]
    fn replay_window_rejects_unseen_packet_beyond_expanded_window() {
        let mut state = RecvReplayState::new();
        for seq in 1..=700 {
            if seq == 100 {
                continue;
            }
            state.check_and_update(seq).unwrap();
        }

        let err = state.check_and_update(100).unwrap_err();
        assert_eq!(err.kind, ReplayRejectKind::TooOld);
        assert_eq!(err.behind, Some(600));
    }
}
