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
- avg connect ms: 3.71
- avg TTFB ms: 289.77
- avg total ms: 293.67
- avg throughput Bps: 135293.36

### adaptive

- runs: 5
- avg connect ms: 3.34
- avg TTFB ms: 180.39
- avg total ms: 183.89
- avg throughput Bps: 182581.61

## Exit Response Path

### exact-3hop

- streams: 5
- sites: chaos-exact-3hop-run1, chaos-exact-3hop-run2, chaos-exact-3hop-run3, chaos-exact-3hop-run4, chaos-exact-3hop-run5
- route lens: 3x5
- http codes: 200x5
- avg ack p95 ms: 89.00
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg window wait ms: 0.00
- avg stream duration ms: 164.60
- avg overlay first-send gap ms: 2.00

### adaptive

- streams: 5
- sites: chaos-adaptive-run1, chaos-adaptive-run2, chaos-adaptive-run3, chaos-adaptive-run4, chaos-adaptive-run5
- route lens: 1x5
- http codes: 200x5
- avg ack p95 ms: 77.80
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg window wait ms: 0.00
- avg stream duration ms: 163.20
- avg overlay first-send gap ms: 1.20

## Route Decisions

- adaptive decision events: 5
- adaptive route changes: 0
- exact route changes: 0
- adaptive decision reasons:
  - current_still_best: 5

## Chaos Actions

- chaos_drop: 7

## Client Settlement Events

- response_transport_local_completion: 16
- response_transport_settlement_completed: 16
- response_transport_settlement_started: 16
- response_transport_terminal_close_duplicate: 30
- response_transport_terminal_close_observed: 15
- response_transport_terminal_payload_observed: 16
