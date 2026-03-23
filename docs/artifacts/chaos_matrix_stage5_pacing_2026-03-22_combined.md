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
- avg connect ms: 1.38
- avg TTFB ms: 464.18
- avg total ms: 465.67
- avg throughput Bps: 74647.59

### adaptive

- runs: 5
- avg connect ms: 8.23
- avg TTFB ms: 644.33
- avg total ms: 652.77
- avg throughput Bps: 51285.75

## Exit Response Path

### exact-3hop

- streams: 5
- sites: chaos-exact-3hop-run1-combined, chaos-exact-3hop-run2-combined, chaos-exact-3hop-run3-combined, chaos-exact-3hop-run4-combined, chaos-exact-3hop-run5-combined
- route lens: 3x5
- http codes: 200x5
- pacing: enabled
- avg ack avg ms: 90.40
- avg ack p95 ms: 164.33
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg window wait ms: 0.00
- avg pacing delay ms: 109.00
- avg pacing interval ms: 3.80
- avg burst prevented: 12.80
- avg paced batches: 13.80
- avg max send burst frames: 4.40
- avg max inflight: 35.20
- avg stream duration ms: 192.00
- avg overlay first-send gap ms: 1.20

### adaptive

- streams: 5
- sites: chaos-adaptive-run1-combined, chaos-adaptive-run2-combined, chaos-adaptive-run3-combined, chaos-adaptive-run4-combined, chaos-adaptive-run5-combined
- route lens: 1x5
- http codes: 200x5
- pacing: enabled
- avg ack avg ms: 252.00
- avg ack p95 ms: 265.40
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg window wait ms: 0.00
- avg pacing delay ms: 170.00
- avg pacing interval ms: 3.80
- avg burst prevented: 6.40
- avg paced batches: 7.40
- avg max send burst frames: 7.20
- avg max inflight: 30.40
- avg stream duration ms: 368.60
- avg overlay first-send gap ms: 3.40

## Route Decisions

- adaptive decision events: 5
- adaptive route changes: 0
- exact route changes: 0
- adaptive decision reasons:
  - current_still_best: 5

## Chaos Actions

- chaos_delay: 4552
- chaos_drop: 3
- chaos_duplicate: 10

## Exit Duplicate Drops

- duplicate_packet_dropped: 5

## Client Duplicate Drops

- duplicate_packet_dropped: 5

## Exit ACK Events

- cumulative_ack_advanced: 136
- inflight_cleanup_by_ack_range: 136
- stale_ack_ignored: 287

## Client Settlement Events

- duplicate_close_stream_suppressed: 32
- response_transport_local_completion: 16
- response_transport_settlement_completed: 16
- response_transport_settlement_started: 16
- response_transport_terminal_close_observed: 16
- response_transport_terminal_payload_observed: 16
- terminal_close_before_local_completion_queued: 3
