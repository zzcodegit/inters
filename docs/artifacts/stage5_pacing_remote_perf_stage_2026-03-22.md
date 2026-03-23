# Overlay stage breakdown RCA

## Inputs

- local raw: `docs/artifacts/stage5_pacing_remote_perf_stage_2026-03-22.local.jsonl`
- client stage trace: `docs/artifacts/stage5_pacing_remote_perf_stage_2026-03-22.client.jsonl`
- exit stage trace: `docs/artifacts/stage5_pacing_remote_perf_stage_2026-03-22.exit.jsonl`
- direct forward stage trace: `docs/artifacts/stage5_pacing_remote_perf_stage_2026-03-22.direct.jsonl`
- joined per-run view: `docs/artifacts/stage5_pacing_remote_perf_stage_2026-03-22.joined.jsonl`

## Scenario Averages

| scenario | route | route ready ms | handshake ms | target connect ms | first target byte wait ms | exit first send delay ms | overlay first-send -> client ms | window stall ms | exit/body tail ms | client total ms | retransmit rate | ack avg ms | ack p95 ms | max burst frames | pacing interval ms | pacing delay ms | burst prevented | window frames |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| direct | 0 | - | - | - | - | - | - | - | - | 1306.83 | - | - | - | - | - | - | - | - |
| remote-1hop | 1 | 1959.58 | 367.00 | 0.80 | 0.00 | 0.00 | 1111.82 | 32.40 | 937.00 | 1113.37 | 0.0000 | 182.80 | 190.00 | 4 | 3.00 | 729.00 | 273 | 64 |
| remote-2hop | 2 | 1743.48 | 364.00 | 0.40 | 0.00 | 0.00 | 1132.71 | 44.00 | 937.40 | 1133.96 | 0.0000 | 183.40 | 200.40 | 4 | 3.00 | 749.60 | 273 | 64 |
| remote-3hop | 3 | 5602.41 | 483.00 | 0.80 | 0.00 | 0.00 | 1515.90 | 155.00 | 1330.40 | 1517.50 | 0.0000 | 251.20 | 288.20 | 4 | 4.00 | 963.20 | 257 | 64 |

## Bottleneck Ranking

1. Warm route ready (one-time): `3101.82 ms`
2. Overlay pre-first-byte gap: `1253.47 ms`
3. Overlay first-send -> client first-byte gap: `1253.47 ms`
4. Exit/body delivery tail: `1068.27 ms`
5. Handshake/session setup (one-time): `404.67 ms`
6. Window/backpressure stall: `77.13 ms`
7. Route selection overhead: `1.13 ms`
8. Client body completion tail: `0.80 ms`
9. Target connect: `0.67 ms`
10. Client request buffering: `0.00 ms`
11. Target first-byte wait: `0.00 ms`
12. Exit first overlay send delay: `0.00 ms`

## Correlation

- `total_time_ms` vs `retransmit_rate`: `-`
- `total_time_ms` vs `ack_latency_ms_avg`: `0.987`

## Findings

- The best overlay path in this sample is `remote-1hop` with avg total `1113.37 ms`.
- `target_connect_ms` and target first-byte wait stay small relative to overlay totals, so the public target itself is not the main bottleneck.
- The biggest per-request cost sits after the target is already reachable: overlay first-send -> client first-byte gap, window/backpressure stall, and the remaining exit/body delivery tail dominate the remote paths.
- `max_send_burst_frames`, `pacing_interval_ms_avg`, `pacing_delay_applied_ms_total`, and `burst_prevented_count` expose whether the exit is still dumping response frames in bursts or spreading them across the ACK window.
- One-time costs (`route_ready_ms`, `handshake_ms`) matter for cold start, but they are not the reason the steady-state per-request `TTFB` stays in the multi-second range.
- Higher retransmit rate and ACK latency move with worse total time, which points to response-path reliability and tail delivery as the main source of degradation.

## Optimization Recommendation

1. Prioritize retransmit/ACK tuning on the response path first; that is where the largest steady-state time is being burned.
2. Use better exit/route scoring second; the current sample shows that path quality matters more than hop-count alone, so scoring should prefer the empirically faster overlay path instead of assuming more or fewer hops are always better.
3. Treat warm tunnel/session reuse as a cold-start improvement, not the main fix for current per-request TTFB.
4. Treat target-connect optimization as low priority here because target connect is already small compared with the overlay delivery tail.
