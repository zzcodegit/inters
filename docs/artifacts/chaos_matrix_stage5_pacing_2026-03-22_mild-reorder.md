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
- avg connect ms: 0.47
- avg TTFB ms: 190.39
- avg total ms: 190.95
- avg throughput Bps: 175926.65

### adaptive

- runs: 5
- avg connect ms: 3.18
- avg TTFB ms: 204.59
- avg total ms: 207.89
- avg throughput Bps: 162959.24

## Exit Response Path

### exact-3hop

- streams: 5
- sites: chaos-exact-3hop-run1-mild-reorder, chaos-exact-3hop-run2-mild-reorder, chaos-exact-3hop-run3-mild-reorder, chaos-exact-3hop-run4-mild-reorder, chaos-exact-3hop-run5-mild-reorder
- route lens: 3x5
- http codes: 200x5
- pacing: enabled
- avg ack avg ms: 40.20
- avg ack p95 ms: 65.20
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg window wait ms: 0.00
- avg pacing delay ms: 37.00
- avg pacing interval ms: 1.00
- avg burst prevented: 9.00
- avg paced batches: 10.00
- avg max send burst frames: 6.20
- avg max inflight: 0.00
- avg stream duration ms: 155.80
- avg overlay first-send gap ms: 1.20

### adaptive

- streams: 5
- sites: chaos-adaptive-run1-mild-reorder, chaos-adaptive-run2-mild-reorder, chaos-adaptive-run3-mild-reorder, chaos-adaptive-run4-mild-reorder, chaos-adaptive-run5-mild-reorder
- route lens: 1x5
- http codes: 200x5
- pacing: enabled
- avg ack avg ms: 26.60
- avg ack p95 ms: 50.80
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg window wait ms: 0.00
- avg pacing delay ms: 44.80
- avg pacing interval ms: 1.00
- avg burst prevented: 7.00
- avg paced batches: 8.00
- avg max send burst frames: 9.80
- avg max inflight: 0.00
- avg stream duration ms: 155.80
- avg overlay first-send gap ms: 1.00

## Route Decisions

- adaptive decision events: 5
- adaptive route changes: 0
- exact route changes: 0
- adaptive decision reasons:
  - current_still_best: 5

## Chaos Actions

- chaos_delay: 75

## Exit ACK Events

- cumulative_ack_advanced: 231
- inflight_cleanup_by_ack_range: 231
- stale_ack_ignored: 193

## Client Settlement Events

- duplicate_close_stream_suppressed: 32
- response_transport_local_completion: 16
- response_transport_settlement_completed: 16
- response_transport_settlement_started: 16
- response_transport_terminal_close_observed: 16
- response_transport_terminal_payload_observed: 16
