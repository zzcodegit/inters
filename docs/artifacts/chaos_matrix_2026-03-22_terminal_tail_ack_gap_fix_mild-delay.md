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
- avg connect ms: 8.32
- avg TTFB ms: 1101.61
- avg total ms: 1110.06
- avg throughput Bps: 36995.42

### adaptive

- runs: 5
- avg connect ms: 12.37
- avg TTFB ms: 682.65
- avg total ms: 695.17
- avg throughput Bps: 52485.00

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
- avg stream duration ms: 258.40
- avg overlay first-send gap ms: 1.80

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
- avg stream duration ms: 231.80
- avg overlay first-send gap ms: 2.40

## Route Decisions

- adaptive decision events: 5
- adaptive route changes: 0
- exact route changes: 0
- adaptive decision reasons:
  - current_still_best: 5

## Chaos Actions

- chaos_delay: 9092

## Client Lifecycle Events

- late_ack_after_completion: 184

## Exit ACK Events

- cumulative_ack_advanced: 57
- inflight_cleanup_by_ack_range: 57
- stale_ack_ignored: 741

## Client Settlement Events

- duplicate_close_stream_suppressed: 32
- duplicate_payload_after_local_completion: 5
- duplicate_payload_after_local_completion_repeat: 175
- response_transport_local_completion: 16
- response_transport_settlement_completed: 16
- response_transport_settlement_started: 16
- response_transport_terminal_close_observed: 16
- response_transport_terminal_payload_observed: 16
- terminal_payload_after_local_completion: 4
