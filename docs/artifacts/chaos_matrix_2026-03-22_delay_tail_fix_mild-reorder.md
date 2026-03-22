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
- avg connect ms: 1.03
- avg TTFB ms: 199.93
- avg total ms: 201.09
- avg throughput Bps: 166177.16

### adaptive

- runs: 5
- avg connect ms: 3.13
- avg TTFB ms: 198.83
- avg total ms: 202.09
- avg throughput Bps: 165515.83

## Exit Response Path

### exact-3hop

- streams: 5
- sites: chaos-exact-3hop-run1, chaos-exact-3hop-run2, chaos-exact-3hop-run3, chaos-exact-3hop-run4, chaos-exact-3hop-run5
- route lens: 3x5
- http codes: 200x5
- avg ack p95 ms: 83.40
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg window wait ms: 0.00
- avg stream duration ms: 165.80
- avg overlay first-send gap ms: 1.40

### adaptive

- streams: 5
- sites: chaos-adaptive-run1, chaos-adaptive-run2, chaos-adaptive-run3, chaos-adaptive-run4, chaos-adaptive-run5
- route lens: 1x5
- http codes: 200x5
- avg ack p95 ms: 75.80
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg window wait ms: 0.00
- avg stream duration ms: 159.60
- avg overlay first-send gap ms: 1.60

## Route Decisions

- adaptive decision events: 5
- adaptive route changes: 0
- exact route changes: 0
- adaptive decision reasons:
  - current_still_best: 5

## Chaos Actions

- chaos_delay: 82

## Client Settlement Events

- response_transport_local_completion: 16
- response_transport_settlement_completed: 16
- response_transport_settlement_started: 16
- response_transport_terminal_close_duplicate: 32
- response_transport_terminal_close_observed: 16
- response_transport_terminal_payload_observed: 16
