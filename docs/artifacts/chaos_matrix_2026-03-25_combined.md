# Chaos Profile Report: combined

## Profile

```json
{
  "profile_name": "combined",
  "seed": 31,
  "skip_packets": 24,
  "loss_ppm": 1000,
  "duplicate_ppm": 2000,
  "reorder_ppm": 15000,
  "base_delay_ms": 8,
  "jitter_ms": 6,
  "reorder_extra_delay_ms": 30,
  "duplicate_delay_ms": 3,
  "runs": 5,
  "request_body_bytes": 32768
}
```

## Measurements

### exact-3hop

- runs: 5
- avg connect ms: 19.38
- avg TTFB ms: 1025.08
- avg total ms: 1044.68
- avg throughput Bps: 32903.44

### adaptive

- runs: 5
- avg connect ms: 5.01
- avg TTFB ms: 450.79
- avg total ms: 455.98
- avg throughput Bps: 72944.53

## Exit Response Path

### exact-3hop

- streams: 5
- sites: chaos-exact-3hop-run1-combined, chaos-exact-3hop-run2-combined, chaos-exact-3hop-run3-combined, chaos-exact-3hop-run4-combined, chaos-exact-3hop-run5-combined
- route lens: 3x5
- http codes: 200x5
- pacing: enabled
- congestion states: nonex5
- avg ack avg ms: 25.60
- avg ack p95 ms: 128.00
- avg total retransmits: 8.40
- avg retransmit rate: 0.2211
- avg retransmit ppm: n/a
- avg hard window wait ms: 0.00
- avg effective cap wait ms: 0.00
- avg pacing delay ms: 248.40
- avg pacing interval ms: 3.80
- avg burst prevented: 8.20
- avg paced batches: 9.20
- avg max send burst frames: 5.80
- avg effective inflight cap: 64.00
- avg inflight cap reduced count: 0.00
- avg inflight cap restore count: 0.00
- avg ack pressure events: 0.00
- avg blocked by effective cap: 0.00
- avg congestion events: 0.00
- avg congestion duration ms: 0.00
- avg congestion cap reductions: 0.00
- avg congestion pacing increases: 0.00
- avg congestion cap limit: 64.00
- avg congestion pacing extra ms: 0.00
- avg max inflight: 37.80
- avg stream duration ms: 383.20
- avg overlay first-send gap ms: 1.80

### adaptive

- streams: 5
- sites: chaos-adaptive-run1-combined, chaos-adaptive-run2-combined, chaos-adaptive-run3-combined, chaos-adaptive-run4-combined, chaos-adaptive-run5-combined
- route lens: 1x5
- http codes: 200x5
- pacing: enabled
- congestion states: nonex5
- avg ack avg ms: 152.00
- avg ack p95 ms: 165.60
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg hard window wait ms: 0.00
- avg effective cap wait ms: 0.00
- avg pacing delay ms: 102.80
- avg pacing interval ms: 3.20
- avg burst prevented: 7.40
- avg paced batches: 8.40
- avg max send burst frames: 6.00
- avg effective inflight cap: 64.00
- avg inflight cap reduced count: 0.00
- avg inflight cap restore count: 0.00
- avg ack pressure events: 0.00
- avg blocked by effective cap: 0.00
- avg congestion events: 0.00
- avg congestion duration ms: 0.00
- avg congestion cap reductions: 0.00
- avg congestion pacing increases: 0.00
- avg congestion cap limit: 64.00
- avg congestion pacing extra ms: 0.00
- avg max inflight: 30.80
- avg stream duration ms: 217.60
- avg overlay first-send gap ms: 2.00

## Route Decisions

- adaptive decision events: 5
- adaptive route changes: 0
- exact route changes: 0
- adaptive decision reasons:
  - current_still_best: 5

## Chaos Actions

- chaos_delay: 6167
- chaos_drop: 5
- chaos_duplicate: 12

## Exit Duplicate Drops

- duplicate_packet_dropped: 4

## Client Duplicate Drops

- duplicate_packet_dropped: 8

## Exit ACK Events

- cumulative_ack_advanced: 131
- inflight_cleanup_by_ack_range: 131
- stale_ack_ignored: 449

## Client Settlement Events

- duplicate_close_stream_suppressed: 32
- duplicate_payload_after_local_completion_absorbed: 3
- response_transport_local_completion: 16
- response_transport_settlement_completed: 16
- response_transport_settlement_started: 16
- response_transport_terminal_close_observed: 16
- response_transport_terminal_payload_observed: 16
- terminal_close_before_local_completion_queued: 2
