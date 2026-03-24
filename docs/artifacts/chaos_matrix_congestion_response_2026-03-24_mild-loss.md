# Chaos Profile Report: mild-loss

## Profile

```json
{
  "profile_name": "mild-loss",
  "seed": 17,
  "skip_packets": 24,
  "loss_ppm": 1000,
  "duplicate_ppm": 0,
  "reorder_ppm": 0,
  "base_delay_ms": 0,
  "jitter_ms": 0,
  "reorder_extra_delay_ms": 0,
  "duplicate_delay_ms": 2,
  "runs": 5,
  "request_body_bytes": 32768
}
```

## Measurements

### exact-3hop

- runs: 5
- avg connect ms: 10.07
- avg TTFB ms: 225.18
- avg total ms: 235.44
- avg throughput Bps: 154094.49

### adaptive

- runs: 5
- avg connect ms: 9.91
- avg TTFB ms: 173.68
- avg total ms: 183.74
- avg throughput Bps: 183024.22

## Exit Response Path

### exact-3hop

- streams: 5
- sites: chaos-exact-3hop-run1-mild-loss, chaos-exact-3hop-run2-mild-loss, chaos-exact-3hop-run3-mild-loss, chaos-exact-3hop-run4-mild-loss, chaos-exact-3hop-run5-mild-loss
- route lens: 3x5
- http codes: 200x5
- pacing: enabled
- congestion states: nonex5
- avg ack avg ms: 41.40
- avg ack p95 ms: 54.20
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg hard window wait ms: 0.00
- avg effective cap wait ms: 0.00
- avg pacing delay ms: 35.80
- avg pacing interval ms: 1.00
- avg burst prevented: 5.60
- avg paced batches: 6.60
- avg max send burst frames: 10.60
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
- avg max inflight: 0.00
- avg stream duration ms: 163.00
- avg overlay first-send gap ms: 1.40

### adaptive

- streams: 5
- sites: chaos-adaptive-run1-mild-loss, chaos-adaptive-run2-mild-loss, chaos-adaptive-run3-mild-loss, chaos-adaptive-run4-mild-loss, chaos-adaptive-run5-mild-loss
- route lens: 1x5
- http codes: 200x5
- pacing: enabled
- congestion states: nonex5
- avg ack avg ms: 31.20
- avg ack p95 ms: 43.80
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg hard window wait ms: 0.00
- avg effective cap wait ms: 0.00
- avg pacing delay ms: 45.60
- avg pacing interval ms: 1.00
- avg burst prevented: 5.00
- avg paced batches: 6.00
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
- avg max inflight: 0.00
- avg stream duration ms: 160.80
- avg overlay first-send gap ms: 1.40

## Route Decisions

- adaptive decision events: 5
- adaptive route changes: 0
- exact route changes: 0
- adaptive decision reasons:
  - current_still_best: 5

## Chaos Actions

- chaos_drop: 4

## Exit ACK Events

- cumulative_ack_advanced: 389
- inflight_cleanup_by_ack_range: 389
- stale_ack_ignored: 32

## Client Settlement Events

- duplicate_close_stream_suppressed: 32
- response_transport_local_completion: 16
- response_transport_settlement_completed: 16
- response_transport_settlement_started: 16
- response_transport_terminal_close_observed: 16
- response_transport_terminal_payload_observed: 16
