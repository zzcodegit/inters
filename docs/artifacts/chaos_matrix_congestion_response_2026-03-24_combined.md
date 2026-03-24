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
- avg connect ms: 1.93
- avg TTFB ms: 453.71
- avg total ms: 455.79
- avg throughput Bps: 74743.74

### adaptive

- runs: 5
- avg connect ms: 3.38
- avg TTFB ms: 362.14
- avg total ms: 365.68
- avg throughput Bps: 90792.91

## Exit Response Path

### exact-3hop

- streams: 5
- sites: chaos-exact-3hop-run1-combined, chaos-exact-3hop-run2-combined, chaos-exact-3hop-run3-combined, chaos-exact-3hop-run4-combined, chaos-exact-3hop-run5-combined
- route lens: 3x5
- http codes: 200x5
- pacing: enabled
- congestion states: nonex5
- avg ack avg ms: 126.40
- avg ack p95 ms: 167.00
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg hard window wait ms: 0.00
- avg effective cap wait ms: 0.00
- avg pacing delay ms: 99.20
- avg pacing interval ms: 3.60
- avg burst prevented: 12.80
- avg paced batches: 13.80
- avg max send burst frames: 5.00
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
- avg max inflight: 32.80
- avg stream duration ms: 193.60
- avg overlay first-send gap ms: 2.00

### adaptive

- streams: 5
- sites: chaos-adaptive-run1-combined, chaos-adaptive-run2-combined, chaos-adaptive-run3-combined, chaos-adaptive-run4-combined, chaos-adaptive-run5-combined
- route lens: 1x5
- http codes: 200x5
- pacing: enabled
- congestion states: nonex5
- avg ack avg ms: 95.80
- avg ack p95 ms: 103.60
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg hard window wait ms: 0.00
- avg effective cap wait ms: 0.00
- avg pacing delay ms: 83.00
- avg pacing interval ms: 3.00
- avg burst prevented: 11.80
- avg paced batches: 12.80
- avg max send burst frames: 6.60
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
- avg max inflight: 26.20
- avg stream duration ms: 164.00
- avg overlay first-send gap ms: 1.60

## Route Decisions

- adaptive decision events: 5
- adaptive route changes: 0
- exact route changes: 0
- adaptive decision reasons:
  - current_still_best: 5

## Chaos Actions

- chaos_delay: 4180
- chaos_drop: 2
- chaos_duplicate: 4

## Exit Duplicate Drops

- duplicate_packet_dropped: 2

## Client Duplicate Drops

- duplicate_packet_dropped: 2

## Exit ACK Events

- cumulative_ack_advanced: 138
- inflight_cleanup_by_ack_range: 138
- stale_ack_ignored: 339

## Client Settlement Events

- duplicate_close_stream_suppressed: 32
- response_transport_local_completion: 16
- response_transport_settlement_completed: 16
- response_transport_settlement_started: 16
- response_transport_terminal_close_observed: 16
- response_transport_terminal_payload_observed: 16
- terminal_close_before_local_completion_queued: 1
