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
- avg connect ms: 0.61
- avg TTFB ms: 179.91
- avg total ms: 180.64
- avg throughput Bps: 184493.91

### adaptive

- runs: 5
- avg connect ms: 1.18
- avg TTFB ms: 175.17
- avg total ms: 176.46
- avg throughput Bps: 195008.77

## Exit Response Path

### exact-3hop

- streams: 5
- sites: chaos-exact-3hop-run1, chaos-exact-3hop-run2, chaos-exact-3hop-run3, chaos-exact-3hop-run4, chaos-exact-3hop-run5
- route lens: 3x5
- http codes: 200x5
- avg ack p95 ms: 77.20
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg window wait ms: 0.00
- avg stream duration ms: 156.20
- avg overlay first-send gap ms: 0.80

### adaptive

- streams: 5
- sites: chaos-adaptive-run1, chaos-adaptive-run2, chaos-adaptive-run3, chaos-adaptive-run4, chaos-adaptive-run5
- route lens: 1x5
- http codes: 200x5
- avg ack p95 ms: 55.60
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg window wait ms: 0.00
- avg stream duration ms: 161.00
- avg overlay first-send gap ms: 1.00

## Route Decisions

- adaptive decision events: 5
- adaptive route changes: 0
- exact route changes: 0
- adaptive decision reasons:
  - current_still_best: 5

## Chaos Actions

- chaos_delay: 72

## Open Message Failures

- packet too old for replay window: 6

## Client Late Events

- late_close_after_completion: 48

## Client Open Message Failures

- packet too old for replay window: 5

## Client Lifecycle Events

- late_ack_after_completion: 1
