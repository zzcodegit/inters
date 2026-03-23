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
- avg connect ms: 1.40
- avg TTFB ms: 333.24
- avg total ms: 334.75
- avg throughput Bps: 98298.64

### adaptive

- runs: 5
- avg connect ms: 4.08
- avg TTFB ms: 324.16
- avg total ms: 328.36
- avg throughput Bps: 100548.45

## Exit Response Path

### exact-3hop

- streams: 5
- sites: chaos-exact-3hop-run1-mild-delay, chaos-exact-3hop-run2-mild-delay, chaos-exact-3hop-run3-mild-delay, chaos-exact-3hop-run4-mild-delay, chaos-exact-3hop-run5-mild-delay
- route lens: 3x5
- http codes: 200x5
- pacing: enabled
- avg ack avg ms: 104.80
- avg ack p95 ms: 109.20
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg window wait ms: 0.00
- avg pacing delay ms: 96.40
- avg pacing interval ms: 3.00
- avg burst prevented: 13.20
- avg paced batches: 14.20
- avg max send burst frames: 4.60
- avg max inflight: 25.20
- avg stream duration ms: 155.60
- avg overlay first-send gap ms: 1.00

### adaptive

- streams: 5
- sites: chaos-adaptive-run1-mild-delay, chaos-adaptive-run2-mild-delay, chaos-adaptive-run3-mild-delay, chaos-adaptive-run4-mild-delay, chaos-adaptive-run5-mild-delay
- route lens: 1x5
- http codes: 200x5
- pacing: enabled
- avg ack avg ms: 81.40
- avg ack p95 ms: 102.60
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg window wait ms: 0.00
- avg pacing delay ms: 78.80
- avg pacing interval ms: 2.80
- avg burst prevented: 9.80
- avg paced batches: 10.80
- avg max send burst frames: 8.80
- avg max inflight: 22.80
- avg stream duration ms: 162.00
- avg overlay first-send gap ms: 1.80

## Route Decisions

- adaptive decision events: 5
- adaptive route changes: 0
- exact route changes: 0
- adaptive decision reasons:
  - current_still_best: 5

## Chaos Actions

- chaos_delay: 3772

## Exit ACK Events

- cumulative_ack_advanced: 185
- inflight_cleanup_by_ack_range: 185
- stale_ack_ignored: 239

## Client Settlement Events

- duplicate_close_stream_suppressed: 32
- response_transport_local_completion: 16
- response_transport_settlement_completed: 16
- response_transport_settlement_started: 16
- response_transport_terminal_close_observed: 16
- response_transport_terminal_payload_observed: 16
