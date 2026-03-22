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
- avg connect ms: 7.65
- avg TTFB ms: 1229.43
- avg total ms: 1237.23
- avg throughput Bps: 32188.68

### adaptive

- runs: 5
- avg connect ms: 3.40
- avg TTFB ms: 421.58
- avg total ms: 425.09
- avg throughput Bps: 78647.43

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
- avg stream duration ms: 270.40
- avg overlay first-send gap ms: 2.00

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
- avg stream duration ms: 203.00
- avg overlay first-send gap ms: 2.20

## Route Decisions

- adaptive decision events: 5
- adaptive route changes: 0
- exact route changes: 0
- adaptive decision reasons:
  - current_still_best: 5

## Chaos Actions

- chaos_delay: 8492

## Client Lifecycle Events

- late_ack_after_completion: 160

## Client Settlement Events

- duplicate_payload_after_local_completion: 4
- duplicate_payload_after_local_completion_repeat: 181
- duplicate_terminal_payload_after_local_completion: 2
- response_transport_local_completion: 16
- response_transport_settlement_completed: 16
- response_transport_settlement_started: 16
- response_transport_terminal_close_duplicate: 32
- response_transport_terminal_close_observed: 16
- response_transport_terminal_payload_observed: 16
- terminal_payload_after_local_completion: 3
