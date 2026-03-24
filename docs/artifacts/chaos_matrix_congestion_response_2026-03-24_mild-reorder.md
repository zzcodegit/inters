# Chaos Profile Report: mild-reorder

## Profile

```json
{
  "profile_name": "mild-reorder",
  "seed": 23,
  "skip_packets": 24,
  "loss_ppm": 0,
  "duplicate_ppm": 0,
  "reorder_ppm": 20000,
  "base_delay_ms": 0,
  "jitter_ms": 0,
  "reorder_extra_delay_ms": 40,
  "duplicate_delay_ms": 2,
  "runs": 5,
  "request_body_bytes": 32768
}
```

## Measurements

### exact-3hop

- runs: 5
- avg connect ms: 1.18
- avg TTFB ms: 197.41
- avg total ms: 198.77
- avg throughput Bps: 169615.48

### adaptive

- runs: 5
- avg connect ms: 5.79
- avg TTFB ms: 192.57
- avg total ms: 198.53
- avg throughput Bps: 168889.44

## Exit Response Path

### exact-3hop

- streams: 5
- sites: chaos-exact-3hop-run1-mild-reorder, chaos-exact-3hop-run2-mild-reorder, chaos-exact-3hop-run3-mild-reorder, chaos-exact-3hop-run4-mild-reorder, chaos-exact-3hop-run5-mild-reorder
- route lens: 3x5
- http codes: 200x5
- pacing: enabled
- congestion states: nonex5
- avg ack avg ms: 48.60
- avg ack p95 ms: 71.40
- avg total retransmits: 0.40
- avg retransmit rate: 0.0105
- avg retransmit ppm: n/a
- avg hard window wait ms: 0.00
- avg effective cap wait ms: 0.00
- avg pacing delay ms: 36.80
- avg pacing interval ms: 1.00
- avg burst prevented: 5.80
- avg paced batches: 6.80
- avg max send burst frames: 10.80
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
- avg stream duration ms: 163.60
- avg overlay first-send gap ms: 1.20

### adaptive

- streams: 5
- sites: chaos-adaptive-run1-mild-reorder, chaos-adaptive-run2-mild-reorder, chaos-adaptive-run3-mild-reorder, chaos-adaptive-run4-mild-reorder, chaos-adaptive-run5-mild-reorder
- route lens: 1x5
- http codes: 200x5
- pacing: enabled
- congestion states: nonex5
- avg ack avg ms: 38.80
- avg ack p95 ms: 62.80
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg hard window wait ms: 0.00
- avg effective cap wait ms: 0.00
- avg pacing delay ms: 36.80
- avg pacing interval ms: 1.00
- avg burst prevented: 4.80
- avg paced batches: 5.80
- avg max send burst frames: 12.40
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
- avg stream duration ms: 154.40
- avg overlay first-send gap ms: 1.20

## Route Decisions

- adaptive decision events: 5
- adaptive route changes: 0
- exact route changes: 0
- adaptive decision reasons:
  - current_still_best: 5

## Chaos Actions

- chaos_delay: 92

## Exit ACK Events

- cumulative_ack_advanced: 162
- inflight_cleanup_by_ack_range: 162
- stale_ack_ignored: 264

## Client Settlement Events

- duplicate_close_stream_suppressed: 32
- response_transport_local_completion: 16
- response_transport_settlement_completed: 16
- response_transport_settlement_started: 16
- response_transport_terminal_close_observed: 16
- response_transport_terminal_payload_observed: 16
