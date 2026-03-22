# Chaos Profile Report: combined

## Profile

```json
{
  "profile_name": "combined",
  "seed": 31,
  "skip_packets": 24,
  "loss_ppm": 1000,
  "duplicate_ppm": 2000,
  "reorder_ppm": 15000,
  "base_delay_ms": 8,
  "jitter_ms": 6,
  "reorder_extra_delay_ms": 30,
  "duplicate_delay_ms": 3,
  "runs": 5,
  "request_body_bytes": 32768
}
```

## Measurements

### exact-3hop

- runs: 5
- avg connect ms: 2.18
- avg TTFB ms: 1486.75
- avg total ms: 1489.06
- avg throughput Bps: 29176.10

### adaptive

- runs: 5
- avg connect ms: 3.86
- avg TTFB ms: 490.05
- avg total ms: 494.04
- avg throughput Bps: 71358.71

## Exit Response Path

### exact-3hop

- streams: 5
- sites: chaos-exact-3hop-run1, chaos-exact-3hop-run2, chaos-exact-3hop-run3, chaos-exact-3hop-run4, chaos-exact-3hop-run5
- route lens: 3x5
- http codes: 200x5
- avg ack p95 ms: n/a
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg window wait ms: 0.00
- avg stream duration ms: 271.60
- avg overlay first-send gap ms: 1.60

### adaptive

- streams: 5
- sites: chaos-adaptive-run1, chaos-adaptive-run2, chaos-adaptive-run3, chaos-adaptive-run4, chaos-adaptive-run5
- route lens: 1x5
- http codes: 200x5
- avg ack p95 ms: n/a
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg window wait ms: 0.00
- avg stream duration ms: 190.40
- avg overlay first-send gap ms: 2.40

## Route Decisions

- adaptive decision events: 5
- adaptive route changes: 0
- exact route changes: 0
- adaptive decision reasons:
  - current_still_best: 5

## Chaos Actions

- chaos_delay: 10356
- chaos_drop: 11
- chaos_duplicate: 19

## Open Message Failures

- duplicate packet detected: 10

## Client Late Events

- late_close_after_completion: 48
- late_payload_after_completion: 3

## Client Open Message Failures

- duplicate packet detected: 8
