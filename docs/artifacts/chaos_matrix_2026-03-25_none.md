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
- avg connect ms: 9.08
- avg TTFB ms: 225.97
- avg total ms: 235.26
- avg throughput Bps: 141684.54

### adaptive

- runs: 5
- avg connect ms: 13.60
- avg TTFB ms: 238.43
- avg total ms: 252.23
- avg throughput Bps: 130915.96

## Exit Response Path

### exact-3hop

- streams: 5
- sites: chaos-exact-3hop-run1-none, chaos-exact-3hop-run2-none, chaos-exact-3hop-run3-none, chaos-exact-3hop-run4-none, chaos-exact-3hop-run5-none
- route lens: 3x5
- http codes: 200x5
- pacing: enabled
- congestion states: nonex5
- avg ack avg ms: 63.80
- avg ack p95 ms: 78.60
- avg total retransmits: 0.40
- avg retransmit rate: 0.0105
- avg retransmit ppm: n/a
- avg hard window wait ms: 0.00
- avg effective cap wait ms: 0.00
- avg pacing delay ms: 30.40
- avg pacing interval ms: 1.00
- avg burst prevented: 3.60
- avg paced batches: 4.60
- avg max send burst frames: 21.80
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
- avg stream duration ms: 176.60
- avg overlay first-send gap ms: 1.20

### adaptive

- streams: 5
- sites: chaos-adaptive-run1-none, chaos-adaptive-run2-none, chaos-adaptive-run3-none, chaos-adaptive-run4-none, chaos-adaptive-run5-none
- route lens: 1x5
- http codes: 200x5
- pacing: enabled
- congestion states: nonex5
- avg ack avg ms: 69.00
- avg ack p95 ms: 98.60
- avg total retransmits: 8.20
- avg retransmit rate: 0.2158
- avg retransmit ppm: n/a
- avg hard window wait ms: 0.00
- avg effective cap wait ms: 0.00
- avg pacing delay ms: 22.40
- avg pacing interval ms: 1.00
- avg burst prevented: 2.20
- avg paced batches: 3.20
- avg max send burst frames: 26.80
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
- avg stream duration ms: 195.60
- avg overlay first-send gap ms: 1.80

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
- stale_ack_ignored: 75

## Client Settlement Events

- duplicate_close_stream_suppressed: 32
- response_transport_local_completion: 16
- response_transport_settlement_completed: 16
- response_transport_settlement_started: 16
- response_transport_terminal_close_observed: 16
- response_transport_terminal_payload_observed: 16
