# Overlay stage breakdown RCA

## Inputs

- local raw: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_residual_fix.local.jsonl`
- client stage trace: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_residual_fix.client.jsonl`
- exit stage trace: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_residual_fix.exit.jsonl`
- direct forward stage trace: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_residual_fix.direct.jsonl`
- joined per-run view: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_residual_fix.joined.jsonl`

## Scenario Averages

| scenario | route | route ready ms | handshake ms | target connect ms | first target byte wait ms | exit first send delay ms | overlay first-send -> client ms | window stall ms | exit/body tail ms | client total ms | retransmit rate | ack p95 ms | window frames |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| direct | 0 | - | - | - | - | - | - | - | - | 1975.46 | - | - | - |
| remote-1hop | 1 | 2797.02 | 692.00 | 0.20 | 1.20 | 0.40 | 1448.96 | 1101.40 | 1208.80 | 1451.97 | 0.0554 | 415.20 | 64 |
| remote-2hop | 2 | 2578.81 | 519.00 | 0.00 | 1.40 | 0.00 | 1467.09 | 933.60 | 1056.20 | 1469.85 | 0.0000 | 371.20 | 64 |
| remote-3hop | 3 | 4065.68 | 1539.00 | 0.40 | 1.20 | 0.20 | 3951.80 | 2697.40 | 2811.40 | 3955.00 | 0.3388 | 1164.80 | 64 |

## Bottleneck Ranking

1. Warm route ready (one-time): `3147.17 ms`
2. Overlay pre-first-byte gap: `2289.48 ms`
3. Overlay first-send -> client first-byte gap: `2289.28 ms`
4. Exit/body delivery tail: `1692.13 ms`
5. Window/backpressure stall: `1577.47 ms`
6. Handshake/session setup (one-time): `916.67 ms`
7. Client body completion tail: `1.32 ms`
8. Target first-byte wait: `1.27 ms`
9. Route selection overhead: `0.73 ms`
10. Target connect: `0.20 ms`
11. Exit first overlay send delay: `0.20 ms`
12. Client request buffering: `0.00 ms`

## Correlation

- `total_time_ms` vs `retransmit_rate`: `0.894`
- `total_time_ms` vs `ack_latency_ms_avg`: `0.941`

## Findings

- The best overlay path in this sample is `remote-1hop` with avg total `1451.97 ms`.
- `target_connect_ms` and target first-byte wait stay small relative to overlay totals, so the public target itself is not the main bottleneck.
- The biggest per-request cost sits after the target is already reachable: overlay first-send -> client first-byte gap, window/backpressure stall, and the remaining exit/body delivery tail dominate the remote paths.
- One-time costs (`route_ready_ms`, `handshake_ms`) matter for cold start, but they are not the reason the steady-state per-request `TTFB` stays in the multi-second range.
- Higher retransmit rate and ACK latency move with worse total time, which points to response-path reliability and tail delivery as the main source of degradation.

## Optimization Recommendation

1. Prioritize retransmit/ACK tuning on the response path first; that is where the largest steady-state time is being burned.
2. Use better exit/route scoring second; the current sample shows that path quality matters more than hop-count alone, so scoring should prefer the empirically faster overlay path instead of assuming more or fewer hops are always better.
3. Treat warm tunnel/session reuse as a cold-start improvement, not the main fix for current per-request TTFB.
4. Treat target-connect optimization as low priority here because target connect is already small compared with the overlay delivery tail.
