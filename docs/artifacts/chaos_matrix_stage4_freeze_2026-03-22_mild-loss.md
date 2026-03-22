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
- avg connect ms: 3.59
- avg TTFB ms: 270.86
- avg total ms: 274.64
- avg throughput Bps: 130621.94

### adaptive

- runs: 5
- avg connect ms: 2.71
- avg TTFB ms: 265.68
- avg total ms: 268.53
- avg throughput Bps: 132239.69

## Exit Response Path

### exact-3hop

- streams: 5
- sites: chaos-exact-3hop-run1, chaos-exact-3hop-run2, chaos-exact-3hop-run3, chaos-exact-3hop-run4, chaos-exact-3hop-run5
- route lens: 3x5
- http codes: 200x5
- avg ack p95 ms: 111.60
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg window wait ms: 0.00
- avg stream duration ms: 179.00
- avg overlay first-send gap ms: 1.40

### adaptive

- streams: 5
- sites: chaos-adaptive-run1, chaos-adaptive-run2, chaos-adaptive-run3, chaos-adaptive-run4, chaos-adaptive-run5
- route lens: 1x5
- http codes: 200x5
- avg ack p95 ms: 100.00
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg window wait ms: 0.00
- avg stream duration ms: 168.00
- avg overlay first-send gap ms: 1.80

## Route Decisions

- adaptive decision events: 5
- adaptive route changes: 0
- exact route changes: 0
- adaptive decision reasons:
  - current_still_best: 5

## Chaos Actions

- chaos_drop: 8

## Exit ACK Events

- cumulative_ack_advanced: 388
- inflight_cleanup_by_ack_range: 388
- stale_ack_ignored: 31

## Client Settlement Events

- duplicate_close_stream_suppressed: 32
- response_transport_local_completion: 16
- response_transport_settlement_completed: 16
- response_transport_settlement_started: 16
- response_transport_terminal_close_observed: 16
- response_transport_terminal_payload_observed: 16
