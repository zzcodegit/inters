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
- avg connect ms: 10.15
- avg TTFB ms: 971.92
- avg total ms: 982.29
- avg throughput Bps: 36234.98

### adaptive

- runs: 5
- avg connect ms: 9.12
- avg TTFB ms: 437.78
- avg total ms: 447.10
- avg throughput Bps: 73609.71

## Exit Response Path

### exact-3hop

- streams: 5
- sites: chaos-exact-3hop-run1-mild-delay, chaos-exact-3hop-run2-mild-delay, chaos-exact-3hop-run3-mild-delay, chaos-exact-3hop-run4-mild-delay, chaos-exact-3hop-run5-mild-delay
- route lens: 3x5
- http codes: 200x5
- pacing: enabled
- congestion states: nonex5
- avg ack avg ms: 59.40
- avg ack p95 ms: 312.00
- avg total retransmits: 10.20
- avg retransmit rate: 0.2684
- avg retransmit ppm: n/a
- avg hard window wait ms: 0.00
- avg effective cap wait ms: 0.00
- avg pacing delay ms: 271.80
- avg pacing interval ms: 4.00
- avg burst prevented: 7.00
- avg paced batches: 8.00
- avg max send burst frames: 5.60
- avg effective inflight cap: 64.00
- avg inflight cap reduced count: 0.00
- avg inflight cap restore count: 0.00
- avg ack pressure events: 0.00
- avg blocked by effective cap: 0.00
- avg congestion events: 0.00
- avg congestion duration ms: 0.00
- avg congestion cap reductions: 0.00
- avg congestion pacing increases: 0.00
- avg congestion cap limit: 64.00
- avg congestion pacing extra ms: 0.00
- avg max inflight: 36.20
- avg stream duration ms: 452.80
- avg overlay first-send gap ms: 2.00

### adaptive

- streams: 5
- sites: chaos-adaptive-run1-mild-delay, chaos-adaptive-run2-mild-delay, chaos-adaptive-run3-mild-delay, chaos-adaptive-run4-mild-delay, chaos-adaptive-run5-mild-delay
- route lens: 1x5
- http codes: 200x5
- pacing: enabled
- congestion states: nonex5
- avg ack avg ms: 138.60
- avg ack p95 ms: 150.60
- avg total retransmits: 0.00
- avg retransmit rate: 0.0000
- avg retransmit ppm: n/a
- avg hard window wait ms: 0.00
- avg effective cap wait ms: 0.00
- avg pacing delay ms: 97.80
- avg pacing interval ms: 3.20
- avg burst prevented: 7.20
- avg paced batches: 8.20
- avg max send burst frames: 6.20
- avg effective inflight cap: 64.00
- avg inflight cap reduced count: 0.00
- avg inflight cap restore count: 0.00
- avg ack pressure events: 0.00
- avg blocked by effective cap: 0.00
- avg congestion events: 0.00
- avg congestion duration ms: 0.00
- avg congestion cap reductions: 0.00
- avg congestion pacing increases: 0.00
- avg congestion cap limit: 64.00
- avg congestion pacing extra ms: 0.00
- avg max inflight: 30.40
- avg stream duration ms: 208.00
- avg overlay first-send gap ms: 2.20

## Route Decisions

- adaptive decision events: 5
- adaptive route changes: 0
- exact route changes: 0
- adaptive decision reasons:
  - current_still_best: 5

## Chaos Actions

- chaos_delay: 5980

## Exit ACK Events

- cumulative_ack_advanced: 139
- inflight_cleanup_by_ack_range: 139
- stale_ack_ignored: 439

## Client Settlement Events

- duplicate_close_stream_suppressed: 32
- duplicate_payload_after_local_completion_absorbed: 8
- response_transport_local_completion: 16
- response_transport_settlement_completed: 16
- response_transport_settlement_started: 16
- response_transport_terminal_close_observed: 16
- response_transport_terminal_payload_observed: 16
- terminal_close_before_local_completion_queued: 1
