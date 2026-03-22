use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

use crate::protocol::StreamFrame;

const ACK_LATENCY_SAMPLE_CAP: usize = 4096;

/// ACK-only control message payload (carried inside `TunnelMessage`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AckFrame {
    pub stream_id: u32,
    /// Next expected in-order frame sequence number (cumulative ACK).
    pub ack_seq: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AckDisposition {
    Advanced,
    Stale,
    Regression,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AckApplyDetails {
    pub disposition: AckDisposition,
    pub previous_ack_seq: u64,
    pub ack_seq: u64,
    pub acked_frames: usize,
    pub avg_latency_ms: Option<u64>,
    pub inflight_before: usize,
    pub inflight_after: usize,
    pub first_acked_seq: Option<u64>,
    pub last_acked_seq_exclusive: Option<u64>,
    pub gap_detected: bool,
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
    pub ack_latency_samples_ms: Vec<u64>,
    pub highest_ack_seq_seen: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AckLatencySummary {
    pub avg_ms: Option<u64>,
    pub min_ms: Option<u64>,
    pub p50_ms: Option<u64>,
    pub p95_ms: Option<u64>,
    pub max_ms: Option<u64>,
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
            ack_latency_samples_ms: Vec::new(),
            highest_ack_seq_seen: 0,
        }
    }

    /// Build an outgoing data frame for the given application payload.
    ///
    /// The frame carries the next send sequence number and the latest contiguous
    /// receive sequence (for ACK piggy-backing).
    pub fn build_outgoing_frame(&mut self, stream_id: u32, payload: Vec<u8>) -> StreamFrame {
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
        self.apply_ack_with_latency_details(ack_seq).acked_frames
    }

    /// Apply cumulative ACK information and also return an approximate ACK latency.
    ///
    /// The latency is computed as `now - first_sent` averaged over frames removed by this ACK.
    pub fn apply_ack_with_latency(&mut self, ack_seq: u64) -> (usize, Option<u64>) {
        let details = self.apply_ack_with_latency_details(ack_seq);
        (details.acked_frames, details.avg_latency_ms)
    }

    pub fn apply_ack_with_latency_details(&mut self, ack_seq: u64) -> AckApplyDetails {
        let previous_ack_seq = self.highest_ack_seq_seen;
        let inflight_before = self.unacked.len();
        let first_acked_seq = self
            .unacked
            .keys()
            .next()
            .copied()
            .filter(|seq| *seq < ack_seq);

        let disposition = if ack_seq > previous_ack_seq {
            self.highest_ack_seq_seen = ack_seq;
            AckDisposition::Advanced
        } else {
            AckDisposition::Stale
        };

        if ack_seq == 0 {
            return AckApplyDetails {
                disposition,
                previous_ack_seq,
                ack_seq,
                acked_frames: 0,
                avg_latency_ms: None,
                inflight_before,
                inflight_after: inflight_before,
                first_acked_seq: None,
                last_acked_seq_exclusive: None,
                gap_detected: false,
            };
        }

        let now = Instant::now();
        let mut acked = 0usize;
        let mut sum_latency_ms: u128 = 0;
        let mut observed_latencies_ms = Vec::new();
        {
            let removed_iter = self.unacked.range(..ack_seq);
            for (_seq, entry) in removed_iter {
                acked += 1;
                let latency_ms = now.duration_since(entry.first_sent).as_millis() as u64;
                sum_latency_ms += latency_ms as u128;
                observed_latencies_ms.push(latency_ms);
            }
        }

        self.unacked.retain(|&seq, _| seq >= ack_seq);
        let inflight_after = self.unacked.len();
        let avg_latency_ms = if acked == 0 {
            None
        } else {
            self.total_frames_acked = self.total_frames_acked.saturating_add(acked as u64);
            self.total_ack_latency_ms_sum =
                self.total_ack_latency_ms_sum.saturating_add(sum_latency_ms);
            self.total_ack_latency_samples =
                self.total_ack_latency_samples.saturating_add(acked as u64);
            let remaining_capacity =
                ACK_LATENCY_SAMPLE_CAP.saturating_sub(self.ack_latency_samples_ms.len());
            self.ack_latency_samples_ms
                .extend(observed_latencies_ms.into_iter().take(remaining_capacity));
            Some((sum_latency_ms / acked as u128) as u64)
        };
        let gap_detected = first_acked_seq
            .map(|first| first.saturating_add(acked as u64) != ack_seq)
            .unwrap_or(false);

        AckApplyDetails {
            disposition,
            previous_ack_seq,
            ack_seq,
            acked_frames: acked,
            avg_latency_ms,
            inflight_before,
            inflight_after,
            first_acked_seq,
            last_acked_seq_exclusive: if acked > 0 { Some(ack_seq) } else { None },
            gap_detected,
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
                self.recv_buffer
                    .entry(s)
                    .or_insert_with(|| frame.payload.clone());
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
        Some((self.total_ack_latency_ms_sum / self.total_ack_latency_samples as u128) as u64)
    }

    pub fn ack_latency_summary(&self) -> AckLatencySummary {
        let avg_ms = self.ack_latency_avg_ms();
        if self.ack_latency_samples_ms.is_empty() {
            return AckLatencySummary {
                avg_ms,
                ..AckLatencySummary::default()
            };
        }

        let mut samples = self.ack_latency_samples_ms.clone();
        samples.sort_unstable();
        AckLatencySummary {
            avg_ms,
            min_ms: samples.first().copied(),
            p50_ms: quantile_from_sorted_samples(&samples, 0.50),
            p95_ms: quantile_from_sorted_samples(&samples, 0.95),
            max_ms: samples.last().copied(),
        }
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

fn quantile_from_sorted_samples(samples: &[u64], quantile: f64) -> Option<u64> {
    if samples.is_empty() {
        return None;
    }
    let clamped = quantile.clamp(0.0, 1.0);
    let index = ((samples.len().saturating_sub(1)) as f64 * clamped).round() as usize;
    samples.get(index).copied()
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

    #[test]
    fn ack_latency_summary_reports_ordered_quantiles() {
        let mut stream = ReliableStream::new();
        let base = Instant::now();

        let f0 = stream.build_outgoing_frame(7, b"a".to_vec());
        let f1 = stream.build_outgoing_frame(7, b"b".to_vec());
        let f2 = stream.build_outgoing_frame(7, b"c".to_vec());
        let f3 = stream.build_outgoing_frame(7, b"d".to_vec());

        stream.unacked.get_mut(&f0.frame_seq).unwrap().first_sent =
            base - Duration::from_millis(40);
        stream.unacked.get_mut(&f1.frame_seq).unwrap().first_sent =
            base - Duration::from_millis(80);
        stream.unacked.get_mut(&f2.frame_seq).unwrap().first_sent =
            base - Duration::from_millis(120);
        stream.unacked.get_mut(&f3.frame_seq).unwrap().first_sent =
            base - Duration::from_millis(160);

        let (acked, avg_ms) = stream.apply_ack_with_latency(4);
        assert_eq!(acked, 4);
        assert!(avg_ms.unwrap() >= 40);

        let summary = stream.ack_latency_summary();
        assert!(summary.min_ms.unwrap() >= 40);
        assert!(summary.p50_ms.unwrap() >= summary.min_ms.unwrap());
        assert!(summary.p95_ms.unwrap() >= summary.p50_ms.unwrap());
        assert!(summary.max_ms.unwrap() >= summary.p95_ms.unwrap());
    }

    #[test]
    fn ack_details_classify_advance_and_treat_older_acks_as_stale() {
        let mut stream = ReliableStream::new();
        let _ = stream.build_outgoing_frame(7, b"a".to_vec());
        let _ = stream.build_outgoing_frame(7, b"b".to_vec());

        let advanced = stream.apply_ack_with_latency_details(1);
        assert_eq!(advanced.disposition, AckDisposition::Advanced);
        assert_eq!(advanced.previous_ack_seq, 0);
        assert_eq!(advanced.ack_seq, 1);
        assert_eq!(advanced.acked_frames, 1);
        assert_eq!(advanced.first_acked_seq, Some(0));
        assert_eq!(advanced.last_acked_seq_exclusive, Some(1));
        assert!(!advanced.gap_detected);

        let stale = stream.apply_ack_with_latency_details(1);
        assert_eq!(stale.disposition, AckDisposition::Stale);
        assert_eq!(stale.acked_frames, 0);

        let older_ack = stream.apply_ack_with_latency_details(0);
        assert_eq!(older_ack.disposition, AckDisposition::Stale);
        assert_eq!(older_ack.acked_frames, 0);
    }
}
