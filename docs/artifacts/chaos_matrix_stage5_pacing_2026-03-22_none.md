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
- avg connect ms: 8.57
- avg TTFB ms: 213.62
- avg total ms: 222.38
- avg throughput Bps: 151025.81

### adaptive

- runs: 5
- avg connect ms: 10.43
- avg TTFB ms: 289.73
- avg total ms: 300.34
- avg throughput Bps: 109757.02

## Exit Response Path

### exact-3hop

- streams: 5
- sites: chaos-exact-3hop-run1-none, chaos-exact-3hop-run2-none, chaos-exact-3hop-run3-none, chaos-exact-3hop-run4-none, chaos-exact-3hop-run5-none
- route lens: 3x5
- http codes: 200x5
- pacing: enabled
- avg ack avg ms: 49.60
- avg ack p95 ms: 64.80
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg window wait ms: 0.00
- avg pacing delay ms: 48.80
- avg pacing interval ms: 1.00
- avg burst prevented: 6.40
- avg paced batches: 7.40
- avg max send burst frames: 12.00
- avg max inflight: 0.00
- avg stream duration ms: 176.00
- avg overlay first-send gap ms: 1.40

### adaptive

- streams: 5
- sites: chaos-adaptive-run1-none, chaos-adaptive-run2-none, chaos-adaptive-run3-none, chaos-adaptive-run4-none, chaos-adaptive-run5-none
- route lens: 1x5
- http codes: 200x5
- pacing: enabled
- avg ack avg ms: 91.40
- avg ack p95 ms: 110.80
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg window wait ms: 0.00
- avg pacing delay ms: 26.60
- avg pacing interval ms: 1.00
- avg burst prevented: 2.00
- avg paced batches: 3.00
- avg max send burst frames: 28.40
- avg max inflight: 0.00
- avg stream duration ms: 226.20
- avg overlay first-send gap ms: 2.00

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
- stale_ack_ignored: 32

## Client Settlement Events

- duplicate_close_stream_suppressed: 32
- response_transport_local_completion: 16
- response_transport_settlement_completed: 16
- response_transport_settlement_started: 16
- response_transport_terminal_close_observed: 16
- response_transport_terminal_payload_observed: 16
