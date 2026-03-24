# Chaos Profile Report: none

## Profile

```json
{
  "profile_name": "none",
  "seed": 1,
  "skip_packets": 0,
  "loss_ppm": 0,
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
- avg connect ms: 8.80
- avg TTFB ms: 180.50
- avg total ms: 189.46
- avg throughput Bps: 178506.19

### adaptive

- runs: 5
- avg connect ms: 10.05
- avg TTFB ms: 173.57
- avg total ms: 183.78
- avg throughput Bps: 183420.61

## Exit Response Path

### exact-3hop

- streams: 5
- sites: chaos-exact-3hop-run1-none, chaos-exact-3hop-run2-none, chaos-exact-3hop-run3-none, chaos-exact-3hop-run4-none, chaos-exact-3hop-run5-none
- route lens: 3x5
- http codes: 200x5
- pacing: enabled
- congestion states: nonex5
- avg ack avg ms: 39.40
- avg ack p95 ms: 52.40
- avg total retransmits: 0.40
- avg retransmit rate: 0.0105
- avg retransmit ppm: n/a
- avg hard window wait ms: 0.00
- avg effective cap wait ms: 0.00
- avg pacing delay ms: 39.20
- avg pacing interval ms: 1.00
- avg burst prevented: 6.00
- avg paced batches: 7.00
- avg max send burst frames: 10.00
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
- avg stream duration ms: 166.00
- avg overlay first-send gap ms: 1.20

### adaptive

- streams: 5
- sites: chaos-adaptive-run1-none, chaos-adaptive-run2-none, chaos-adaptive-run3-none, chaos-adaptive-run4-none, chaos-adaptive-run5-none
- route lens: 1x5
- http codes: 200x5
- pacing: enabled
- congestion states: nonex5
- avg ack avg ms: 34.20
- avg ack p95 ms: 47.20
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg hard window wait ms: 0.00
- avg effective cap wait ms: 0.00
- avg pacing delay ms: 37.60
- avg pacing interval ms: 1.00
- avg burst prevented: 4.60
- avg paced batches: 5.60
- avg max send burst frames: 13.20
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
- avg overlay first-send gap ms: 1.60

## Route Decisions

- adaptive decision events: 5
- adaptive route changes: 0
- exact route changes: 0
- adaptive decision reasons:
  - current_still_best: 5

## Chaos Actions

- no transport chaos actions emitted

## Exit ACK Events

- cumulative_ack_advanced: 392
- inflight_cleanup_by_ack_range: 392
- stale_ack_ignored: 34

## Client Settlement Events

- duplicate_close_stream_suppressed: 32
- response_transport_local_completion: 16
- response_transport_settlement_completed: 16
- response_transport_settlement_started: 16
- response_transport_terminal_close_observed: 16
- response_transport_terminal_payload_observed: 16
