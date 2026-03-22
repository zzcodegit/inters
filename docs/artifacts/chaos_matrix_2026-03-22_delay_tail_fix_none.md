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
- avg connect ms: 2.14
- avg TTFB ms: 167.70
- avg total ms: 169.97
- avg throughput Bps: 197704.44

### adaptive

- runs: 5
- avg connect ms: 2.89
- avg TTFB ms: 179.96
- avg total ms: 182.99
- avg throughput Bps: 182419.00

## Exit Response Path

### exact-3hop

- streams: 5
- sites: chaos-exact-3hop-run1, chaos-exact-3hop-run2, chaos-exact-3hop-run3, chaos-exact-3hop-run4, chaos-exact-3hop-run5
- route lens: 3x5
- http codes: 200x5
- avg ack p95 ms: 65.00
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg window wait ms: 0.00
- avg stream duration ms: 154.40
- avg overlay first-send gap ms: 1.00

### adaptive

- streams: 5
- sites: chaos-adaptive-run1, chaos-adaptive-run2, chaos-adaptive-run3, chaos-adaptive-run4, chaos-adaptive-run5
- route lens: 1x5
- http codes: 200x5
- avg ack p95 ms: 75.60
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg window wait ms: 0.00
- avg stream duration ms: 163.40
- avg overlay first-send gap ms: 1.60

## Route Decisions

- adaptive decision events: 5
- adaptive route changes: 0
- exact route changes: 0
- adaptive decision reasons:
  - current_still_best: 5

## Chaos Actions

- no transport chaos actions emitted

## Client Settlement Events

- response_transport_local_completion: 16
- response_transport_settlement_completed: 16
- response_transport_settlement_started: 16
- response_transport_terminal_close_duplicate: 32
- response_transport_terminal_close_observed: 16
- response_transport_terminal_payload_observed: 16
