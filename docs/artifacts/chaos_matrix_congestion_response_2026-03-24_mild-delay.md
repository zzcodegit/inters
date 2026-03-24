# Chaos Profile Report: mild-delay

## Profile

```json
{
  "profile_name": "mild-delay",
  "seed": 29,
  "skip_packets": 24,
  "loss_ppm": 0,
  "duplicate_ppm": 0,
  "reorder_ppm": 0,
  "base_delay_ms": 4,
  "jitter_ms": 2,
  "reorder_extra_delay_ms": 0,
  "duplicate_delay_ms": 2,
  "runs": 5,
  "request_body_bytes": 32768
}
```

## Measurements

### exact-3hop

- runs: 5
- avg connect ms: 1.82
- avg TTFB ms: 445.70
- avg total ms: 447.69
- avg throughput Bps: 75120.07

### adaptive

- runs: 5
- avg connect ms: 2.52
- avg TTFB ms: 393.82
- avg total ms: 396.49
- avg throughput Bps: 88425.01

## Exit Response Path

### exact-3hop

- streams: 5
- sites: chaos-exact-3hop-run1-mild-delay, chaos-exact-3hop-run2-mild-delay, chaos-exact-3hop-run3-mild-delay, chaos-exact-3hop-run4-mild-delay, chaos-exact-3hop-run5-mild-delay
- route lens: 3x5
- http codes: 200x5
- pacing: enabled
- congestion states: nonex5
- avg ack avg ms: 149.60
- avg ack p95 ms: 194.50
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg hard window wait ms: 0.00
- avg effective cap wait ms: 0.00
- avg pacing delay ms: 123.80
- avg pacing interval ms: 3.80
- avg burst prevented: 9.20
- avg paced batches: 10.20
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
- avg max inflight: 34.00
- avg stream duration ms: 230.60
- avg overlay first-send gap ms: 1.20

### adaptive

- streams: 5
- sites: chaos-adaptive-run1-mild-delay, chaos-adaptive-run2-mild-delay, chaos-adaptive-run3-mild-delay, chaos-adaptive-run4-mild-delay, chaos-adaptive-run5-mild-delay
- route lens: 1x5
- http codes: 200x5
- pacing: enabled
- congestion states: nonex5
- avg ack avg ms: 95.80
- avg ack p95 ms: 129.80
- avg total retransmits: 5.20
- avg retransmit rate: 0.1368
- avg retransmit ppm: n/a
- avg hard window wait ms: 0.00
- avg effective cap wait ms: 0.00
- avg pacing delay ms: 87.40
- avg pacing interval ms: 2.60
- avg burst prevented: 7.80
- avg paced batches: 8.80
- avg max send burst frames: 11.80
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
- avg max inflight: 25.60
- avg stream duration ms: 246.60
- avg overlay first-send gap ms: 1.60

## Route Decisions

- adaptive decision events: 5
- adaptive route changes: 0
- exact route changes: 0
- adaptive decision reasons:
  - current_still_best: 5

## Chaos Actions

- chaos_delay: 4440

## Exit ACK Events

- cumulative_ack_advanced: 172
- inflight_cleanup_by_ack_range: 172
- stale_ack_ignored: 313

## Client Settlement Events

- duplicate_close_stream_suppressed: 32
- response_transport_local_completion: 16
- response_transport_settlement_completed: 16
- response_transport_settlement_started: 16
- response_transport_terminal_close_observed: 16
- response_transport_terminal_payload_observed: 16
- terminal_close_before_local_completion_queued: 2
