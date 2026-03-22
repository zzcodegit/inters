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
- avg connect ms: 6.10
- avg TTFB ms: 1281.51
- avg total ms: 1287.75
- avg throughput Bps: 27635.43

### adaptive

- runs: 5
- avg connect ms: 10.44
- avg TTFB ms: 547.23
- avg total ms: 557.83
- avg throughput Bps: 62408.44

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
- avg stream duration ms: 301.40
- avg overlay first-send gap ms: 2.20

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
- avg stream duration ms: 191.00
- avg overlay first-send gap ms: 2.00

## Route Decisions

- adaptive decision events: 5
- adaptive route changes: 0
- exact route changes: 0
- adaptive decision reasons:
  - current_still_best: 5

## Chaos Actions

- chaos_delay: 9301
- chaos_drop: 7
- chaos_duplicate: 10

## Open Message Failures

- duplicate packet detected: 6

## Client Open Message Failures

- duplicate packet detected: 4

## Client Lifecycle Events

- late_ack_after_completion: 134

## Client Settlement Events

- duplicate_payload_after_local_completion: 4
- duplicate_payload_after_local_completion_repeat: 160
- duplicate_terminal_payload_after_local_completion: 1
- response_transport_local_completion: 16
- response_transport_settlement_completed: 16
- response_transport_settlement_started: 16
- response_transport_terminal_close_duplicate: 32
- response_transport_terminal_close_observed: 16
- response_transport_terminal_payload_observed: 16
- terminal_payload_after_local_completion: 3
