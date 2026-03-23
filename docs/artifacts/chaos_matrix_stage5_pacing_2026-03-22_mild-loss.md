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
- avg connect ms: 13.16
- avg TTFB ms: 262.83
- avg total ms: 276.16
- avg throughput Bps: 130398.73

### adaptive

- runs: 5
- avg connect ms: 15.56
- avg TTFB ms: 265.46
- avg total ms: 281.19
- avg throughput Bps: 118709.99

## Exit Response Path

### exact-3hop

- streams: 5
- sites: chaos-exact-3hop-run1-mild-loss, chaos-exact-3hop-run2-mild-loss, chaos-exact-3hop-run3-mild-loss, chaos-exact-3hop-run4-mild-loss, chaos-exact-3hop-run5-mild-loss
- route lens: 3x5
- http codes: 200x5
- pacing: enabled
- avg ack avg ms: 60.00
- avg ack p95 ms: 81.60
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg window wait ms: 0.00
- avg pacing delay ms: 41.60
- avg pacing interval ms: 1.00
- avg burst prevented: 5.40
- avg paced batches: 6.40
- avg max send burst frames: 14.20
- avg max inflight: 3.80
- avg stream duration ms: 179.80
- avg overlay first-send gap ms: 2.20

### adaptive

- streams: 5
- sites: chaos-adaptive-run1-mild-loss, chaos-adaptive-run2-mild-loss, chaos-adaptive-run3-mild-loss, chaos-adaptive-run4-mild-loss, chaos-adaptive-run5-mild-loss
- route lens: 1x5
- http codes: 200x5
- pacing: enabled
- avg ack avg ms: 74.00
- avg ack p95 ms: 91.20
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg window wait ms: 0.00
- avg pacing delay ms: 44.00
- avg pacing interval ms: 1.00
- avg burst prevented: 3.20
- avg paced batches: 4.20
- avg max send burst frames: 24.20
- avg max inflight: 0.40
- avg stream duration ms: 206.60
- avg overlay first-send gap ms: 2.60

## Route Decisions

- adaptive decision events: 5
- adaptive route changes: 0
- exact route changes: 0
- adaptive decision reasons:
  - current_still_best: 5

## Chaos Actions

- chaos_drop: 5

## Exit ACK Events

- cumulative_ack_advanced: 389
- inflight_cleanup_by_ack_range: 389
- stale_ack_ignored: 31

## Client Settlement Events

- duplicate_close_stream_suppressed: 32
- response_transport_local_completion: 16
- response_transport_settlement_completed: 16
- response_transport_settlement_started: 16
- response_transport_terminal_close_observed: 16
- response_transport_terminal_payload_observed: 16
