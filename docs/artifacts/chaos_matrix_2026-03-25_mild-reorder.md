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
- avg connect ms: 7.38
- avg TTFB ms: 265.05
- avg total ms: 272.68
- avg throughput Bps: 123749.26

### adaptive

- runs: 5
- avg connect ms: 15.36
- avg TTFB ms: 270.44
- avg total ms: 286.00
- avg throughput Bps: 116207.62

## Exit Response Path

### exact-3hop

- streams: 5
- sites: chaos-exact-3hop-run1-mild-reorder, chaos-exact-3hop-run2-mild-reorder, chaos-exact-3hop-run3-mild-reorder, chaos-exact-3hop-run4-mild-reorder, chaos-exact-3hop-run5-mild-reorder
- route lens: 3x5
- http codes: 200x5
- pacing: enabled
- congestion states: nonex5
- avg ack avg ms: 65.20
- avg ack p95 ms: 95.20
- avg total retransmits: 6.20
- avg retransmit rate: 0.1632
- avg retransmit ppm: n/a
- avg hard window wait ms: 0.00
- avg effective cap wait ms: 0.00
- avg pacing delay ms: 18.60
- avg pacing interval ms: 1.00
- avg burst prevented: 2.20
- avg paced batches: 3.20
- avg max send burst frames: 29.40
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
- avg max inflight: 3.60
- avg stream duration ms: 179.40
- avg overlay first-send gap ms: 1.40

### adaptive

- streams: 5
- sites: chaos-adaptive-run1-mild-reorder, chaos-adaptive-run2-mild-reorder, chaos-adaptive-run3-mild-reorder, chaos-adaptive-run4-mild-reorder, chaos-adaptive-run5-mild-reorder
- route lens: 1x5
- http codes: 200x5
- pacing: enabled
- congestion states: nonex5
- avg ack avg ms: 56.80
- avg ack p95 ms: 103.00
- avg total retransmits: 20.60
- avg retransmit rate: 0.5421
- avg retransmit ppm: n/a
- avg hard window wait ms: 0.00
- avg effective cap wait ms: 0.00
- avg pacing delay ms: 17.20
- avg pacing interval ms: 1.00
- avg burst prevented: 2.00
- avg paced batches: 3.00
- avg max send burst frames: 29.40
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
- avg max inflight: 9.40
- avg stream duration ms: 187.20
- avg overlay first-send gap ms: 1.60

## Route Decisions

- adaptive decision events: 5
- adaptive route changes: 0
- exact route changes: 0
- adaptive decision reasons:
  - current_still_best: 5

## Chaos Actions

- chaos_delay: 82

## Exit ACK Events

- cumulative_ack_advanced: 263
- inflight_cleanup_by_ack_range: 263
- stale_ack_ignored: 295

## Client Settlement Events

- duplicate_close_stream_suppressed: 32
- response_transport_local_completion: 16
- response_transport_settlement_completed: 16
- response_transport_settlement_started: 16
- response_transport_terminal_close_observed: 16
- response_transport_terminal_payload_observed: 16
