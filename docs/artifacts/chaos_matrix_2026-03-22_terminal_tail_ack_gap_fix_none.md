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
- avg connect ms: 3.33
- avg TTFB ms: 222.87
- avg total ms: 226.36
- avg throughput Bps: 145620.12

### adaptive

- runs: 5
- avg connect ms: 2.47
- avg TTFB ms: 247.77
- avg total ms: 250.40
- avg throughput Bps: 131486.09

## Exit Response Path

### exact-3hop

- streams: 5
- sites: chaos-exact-3hop-run1, chaos-exact-3hop-run2, chaos-exact-3hop-run3, chaos-exact-3hop-run4, chaos-exact-3hop-run5
- route lens: 3x5
- http codes: 200x5
- avg ack p95 ms: 111.80
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg window wait ms: 0.00
- avg stream duration ms: 176.20
- avg overlay first-send gap ms: 1.20

### adaptive

- streams: 5
- sites: chaos-adaptive-run1, chaos-adaptive-run2, chaos-adaptive-run3, chaos-adaptive-run4, chaos-adaptive-run5
- route lens: 1x5
- http codes: 200x5
- avg ack p95 ms: 115.60
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg window wait ms: 0.00
- avg stream duration ms: 187.80
- avg overlay first-send gap ms: 2.00

## Route Decisions

- adaptive decision events: 5
- adaptive route changes: 0
- exact route changes: 0
- adaptive decision reasons:
  - current_still_best: 5

## Chaos Actions

- no transport chaos actions emitted

## Exit ACK Events

- cumulative_ack_advanced: 392
- inflight_cleanup_by_ack_range: 392
- stale_ack_ignored: 32

## Client Settlement Events

- duplicate_close_stream_suppressed: 32
- response_transport_local_completion: 16
- response_transport_settlement_completed: 16
- response_transport_settlement_started: 16
- response_transport_terminal_close_observed: 16
- response_transport_terminal_payload_observed: 16
