use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::protocol::StreamFrame;

/// ACK-only control message payload (carried inside `TunnelMessage`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AckFrame {
    pub stream_id: u32,
    /// Next expected in-order frame sequence number (cumulative ACK).
    pub ack_seq: u64,
}

/// Stage 4/5: per-stream reliable delivery state.
pub struct ReliableStream {
    pub send_next: u64,
    pub recv_next: u64,
    pub recv_buffer: BTreeMap<u64, Vec<u8>>,
    /// Unacked outbound frames with simple pacing metadata.
    pub unacked: BTreeMap<u64, UnackedEntry>,

    /// Debug/metrics counters for this reliable stream.
    pub total_frames_sent: u64,
    pub total_frames_acked: u64,
    pub total_retransmits: u64,
    pub total_ack_latency_ms_sum: u128,
    pub total_ack_latency_samples: u64,
}

pub struct UnackedEntry {
    pub payload: Vec<u8>,
    /// Marks explicit end-of-stream empty frame.
    pub is_end: bool,
    /// First time this frame was sent.
    pub first_sent: Instant,
    /// Last time this frame was sent (initial send or retransmit).
    pub last_sent: Instant,
    /// How many times this frame has been retransmitted.
    pub retransmit_count: u64,
}

impl ReliableStream {
    pub fn new() -> Self {
        Self {
            send_next: 0,
            recv_next: 0,
            recv_buffer: BTreeMap::new(),
            unacked: BTreeMap::new(),
            total_frames_sent: 0,
            total_frames_acked: 0,
            total_retransmits: 0,
            total_ack_latency_ms_sum: 0,
            total_ack_latency_samples: 0,
        }
    }

    /// Build an outgoing data frame for the given application payload.
    ///
    /// The frame carries the next send sequence number and the latest contiguous
    /// receive sequence (for ACK piggy-backing).
    pub fn build_outgoing_frame(
        &mut self,
        stream_id: u32,
        payload: Vec<u8>,
    ) -> StreamFrame {
        let seq = self.send_next;
        self.send_next = self.send_next.wrapping_add(1);
        let is_end = payload.is_empty();
        let now = Instant::now();
        self.total_frames_sent = self.total_frames_sent.saturating_add(1);
        self.unacked.insert(
            seq,
            UnackedEntry {
                payload: payload.clone(),
                is_end,
                first_sent: now,
                last_sent: now,
                retransmit_count: 0,
            },
        );
        StreamFrame {
            stream_id,
            frame_seq: seq,
            ack_seq: self.recv_next,
            payload,
        }
    }

    /// Mark that the given sequence number has just been sent (initially or retransmit).
    pub fn mark_sent(&mut self, seq: u64) {
        if let Some(entry) = self.unacked.get_mut(&seq) {
            entry.last_sent = Instant::now();
        }
    }

    /// Apply cumulative ACK information (next expected in-order sequence number from peer).
    /// Returns how many frames were removed from `unacked`.
    pub fn apply_ack(&mut self, ack_seq: u64) -> usize {
        if ack_seq == 0 {
            return 0;
        }
        let acked = self.unacked.range(..ack_seq).count();
        self.total_frames_acked = self.total_frames_acked.saturating_add(acked as u64);
        self.unacked.retain(|&seq, _| seq >= ack_seq);
        acked
    }

    /// Apply cumulative ACK information and also return an approximate ACK latency.
    ///
    /// The latency is computed as `now - first_sent` averaged over frames removed by this ACK.
    pub fn apply_ack_with_latency(&mut self, ack_seq: u64) -> (usize, Option<u64>) {
        if ack_seq == 0 {
            return (0, None);
        }

        let now = Instant::now();
        let mut acked = 0usize;
        let mut sum_latency_ms: u128 = 0;
        {
            let removed_iter = self.unacked.range(..ack_seq);
            for (_seq, entry) in removed_iter {
                acked += 1;
                sum_latency_ms += now
                    .duration_since(entry.first_sent)
                    .as_millis() as u128;
            }
        }

        self.unacked.retain(|&seq, _| seq >= ack_seq);

        if acked == 0 {
            (0, None)
        } else {
            self.total_frames_acked = self.total_frames_acked.saturating_add(acked as u64);
            self.total_ack_latency_ms_sum =
                self.total_ack_latency_ms_sum.saturating_add(sum_latency_ms);
            self.total_ack_latency_samples =
                self.total_ack_latency_samples.saturating_add(acked as u64);
            let avg = (sum_latency_ms / acked as u128) as u64;
            (acked, Some(avg))
        }
    }

    /// Process an incoming frame and update ordering / acknowledgements.
    ///
    /// Returns:
    /// - a vector of payload chunks that became in-order and should be delivered;
    /// - the latest `ack_seq` value that should be sent back to the peer;
    /// - `true` if this frame was an in-order empty (end-of-stream marker).
    pub fn process_incoming(&mut self, frame: &StreamFrame) -> (Vec<Vec<u8>>, u64, bool) {
        // First, apply ACK information for frames we have sent.
        let _ = self.apply_ack(frame.ack_seq);

        let mut deliver = Vec::new();
        let mut end_of_stream = false;

        if !frame.payload.is_empty() {
            let s = frame.frame_seq;
            if s == self.recv_next {
                deliver.push(frame.payload.clone());
                self.recv_next = self.recv_next.wrapping_add(1);
                while let Some(buf) = self.recv_buffer.remove(&self.recv_next) {
                    if buf.is_empty() {
                        end_of_stream = true;
                    }
                    deliver.push(buf);
                    self.recv_next = self.recv_next.wrapping_add(1);
                }
            } else if s > self.recv_next {
                self.recv_buffer.entry(s).or_insert_with(|| frame.payload.clone());
            }
        } else {
            // Empty payload: end-of-stream marker. Store if out-of-order.
            let s = frame.frame_seq;
            if s == self.recv_next {
                self.recv_next = self.recv_next.wrapping_add(1);
                end_of_stream = true;
                while let Some(buf) = self.recv_buffer.remove(&self.recv_next) {
                    if buf.is_empty() {
                        end_of_stream = true;
                    }
                    deliver.push(buf);
                    self.recv_next = self.recv_next.wrapping_add(1);
                }
            } else if s > self.recv_next {
                self.recv_buffer.entry(s).or_insert_with(Vec::new);
            }
        }

        (deliver, self.recv_next, end_of_stream)
    }

    /// Current number of unacknowledged frames in flight.
    pub fn inflight(&self) -> usize {
        self.unacked.len()
    }

    /// Current number of queued out-of-order frames awaiting reassembly.
    pub fn recv_buffer_len(&self) -> usize {
        self.recv_buffer.len()
    }

    /// Average ACK latency in milliseconds across frames that were acked.
    pub fn ack_latency_avg_ms(&self) -> Option<u64> {
        if self.total_ack_latency_samples == 0 {
            return None;
        }
        Some(
            (self.total_ack_latency_ms_sum
                / self.total_ack_latency_samples as u128) as u64,
        )
    }

    pub fn counters_snapshot(&self) -> (u64, u64, u64) {
        (
            self.total_frames_sent,
            self.total_frames_acked,
            self.total_retransmits,
        )
    }

    /// Whether we can send a new frame without exceeding the window.
    pub fn can_send(&self, window_frames: usize) -> bool {
        self.inflight() < window_frames
    }

    /// Select a small, paced subset of unacked frames for retransmission.
    ///
    /// - `interval`: how long a frame must be idle before we retransmit.
    /// - `max_frames`: cap per tick to avoid packet storms.
    /// - Returns frames in order from oldest seq.
    pub fn frames_for_retransmit(
        &mut self,
        stream_id: u32,
        now: Instant,
        interval: Duration,
        max_frames: usize,
    ) -> Vec<StreamFrame> {
        let mut out = Vec::new();
        for (&seq, entry) in self.unacked.iter_mut() {
            if out.len() >= max_frames {
                break;
            }
            if now.duration_since(entry.last_sent) < interval {
                continue;
            }
            entry.last_sent = now;
            entry.retransmit_count = entry.retransmit_count.saturating_add(1);
            self.total_retransmits = self.total_retransmits.saturating_add(1);
            out.push(StreamFrame {
                stream_id,
                frame_seq: seq,
                ack_seq: self.recv_next,
                payload: entry.payload.clone(),
            });
        }
        out
    }

    /// Return all unacked frames (used only in unit tests).
    pub fn unacked_frames(&self, stream_id: u32) -> Vec<StreamFrame> {
        self.unacked
            .iter()
            .map(|(&seq, entry)| StreamFrame {
                stream_id,
                frame_seq: seq,
                ack_seq: self.recv_next,
                payload: entry.payload.clone(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn out_of_order_is_reordered() {
        let mut a = ReliableStream::new();
        let mut b = ReliableStream::new();

        // a -> b: send three frames
        let f1 = a.build_outgoing_frame(1, b"one".to_vec());
        let f2 = a.build_outgoing_frame(1, b"two".to_vec());
        let f3 = a.build_outgoing_frame(1, b"three".to_vec());

        // deliver to b in order 1,3,2 and ensure application sees 1,2,3.
        let (d1, _, _) = b.process_incoming(&f1);
        assert_eq!(d1, vec![b"one".to_vec()]);

        let (d3, _, _) = b.process_incoming(&f3);
        assert!(d3.is_empty());

        let (d2, _, _) = b.process_incoming(&f2);
        assert_eq!(d2, vec![b"two".to_vec(), b"three".to_vec()]);
    }

    #[test]
    fn ack_clears_unacked() {
        let mut a = ReliableStream::new();
        let mut b = ReliableStream::new();

        let f1 = a.build_outgoing_frame(1, b"one".to_vec());
        let f2 = a.build_outgoing_frame(1, b"two".to_vec());
        assert_eq!(a.inflight(), 2);

        // b receives f1 and f2 and sends back an ACK via ack_seq.
        let (_d1, _ack1, _) = b.process_incoming(&f1);
        let (_d2, ack2, _) = b.process_incoming(&f2);

        // a learns about ack2 via an incoming frame that carries that ack_seq.
        let ack_only = StreamFrame {
            stream_id: 1,
            frame_seq: 0,
            ack_seq: ack2,
            payload: Vec::new(),
        };
        let (_d, _ack_back, _) = a.process_incoming(&ack_only);
        assert_eq!(a.inflight(), 0);
    }

    #[test]
    fn out_of_order_empty_frame_stored_and_delivers_end_of_stream() {
        let mut b = ReliableStream::new();
        // Receive empty (end) before data - UDP reordering.
        let empty_end = StreamFrame {
            stream_id: 1,
            frame_seq: 1,
            ack_seq: 0,
            payload: Vec::new(),
        };
        let (d1, _, eos1) = b.process_incoming(&empty_end);
        assert!(d1.is_empty());
        assert!(!eos1, "empty out of order should not signal end yet");

        // Now receive data frame 0.
        let data0 = StreamFrame {
            stream_id: 1,
            frame_seq: 0,
            ack_seq: 0,
            payload: b"hello".to_vec(),
        };
        let (d2, _, eos2) = b.process_incoming(&data0);
        assert_eq!(d2, vec![b"hello".to_vec(), vec![]]);
        assert!(eos2, "after draining buffered empty, should signal end");
    }
}

