# Overlay stage breakdown RCA

## Inputs

- local raw: `docs/artifacts/chaos_lifecycle_remote_perf_stage_2026-03-22.local.jsonl`
- client stage trace: `docs/artifacts/chaos_lifecycle_remote_perf_stage_2026-03-22.client.jsonl`
- exit stage trace: `docs/artifacts/chaos_lifecycle_remote_perf_stage_2026-03-22.exit.jsonl`
- direct forward stage trace: `docs/artifacts/chaos_lifecycle_remote_perf_stage_2026-03-22.direct.jsonl`
- joined per-run view: `docs/artifacts/remote_perf_stage_matrix_2026-03-21.joined.jsonl`

## Scenario Averages

| scenario | route | route ready ms | handshake ms | target connect ms | first target byte wait ms | exit first send delay ms | overlay first-send -> client ms | window stall ms | exit/body tail ms | client total ms | retransmit rate | ack p95 ms | window frames |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| direct | 0 | - | - | - | - | - | - | - | - | 1229.69 | - | - | - |
| remote-1hop | 1 | 1908.81 | 357.00 | 0.00 | 1.40 | 0.00 | 1088.87 | 762.80 | 874.80 | 1091.11 | 0.0000 | 232.40 | 64 |
| remote-2hop | 2 | 1934.51 | 371.00 | 0.00 | 1.20 | 0.20 | 1738.35 | 1256.60 | 1388.40 | 1740.55 | 0.2048 | 683.40 | 64 |
| remote-3hop | 3 | 2461.94 | 844.00 | 0.20 | 1.40 | 0.40 | 1459.22 | 1115.40 | 1213.60 | 1462.08 | 0.0000 | 361.40 | 64 |

## Bottleneck Ranking

1. Warm route ready (one-time): `2101.76 ms`
2. Overlay pre-first-byte gap: `1429.01 ms`
3. Overlay first-send -> client first-byte gap: `1428.81 ms`
4. Exit/body delivery tail: `1158.93 ms`
5. Window/backpressure stall: `1044.93 ms`
6. Handshake/session setup (one-time): `524.00 ms`
7. Target first-byte wait: `1.33 ms`
8. Route selection overhead: `1.00 ms`
9. Client body completion tail: `0.83 ms`
10. Exit first overlay send delay: `0.20 ms`
11. Target connect: `0.07 ms`
12. Client request buffering: `0.00 ms`

## Correlation

- `total_time_ms` vs `retransmit_rate`: `0.722`
- `total_time_ms` vs `ack_latency_ms_avg`: `0.872`

## Findings

- The best overlay path in this sample is `remote-1hop` with avg total `1091.11 ms`.
- `target_connect_ms` and target first-byte wait stay small relative to overlay totals, so the public target itself is not the main bottleneck.
- The biggest per-request cost sits after the target is already reachable: overlay first-send -> client first-byte gap, window/backpressure stall, and the remaining exit/body delivery tail dominate the remote paths.
- One-time costs (`route_ready_ms`, `handshake_ms`) matter for cold start, but they are not the reason the steady-state per-request `TTFB` stays in the multi-second range.
- Higher retransmit rate and ACK latency move with worse total time, which points to response-path reliability and tail delivery as the main source of degradation.

## Optimization Recommendation

1. Prioritize retransmit/ACK tuning on the response path first; that is where the largest steady-state time is being burned.
2. Use better exit/route scoring second; the current sample shows that path quality matters more than hop-count alone, so scoring should prefer the empirically faster overlay path instead of assuming more or fewer hops are always better.
3. Treat warm tunnel/session reuse as a cold-start improvement, not the main fix for current per-request TTFB.
4. Treat target-connect optimization as low priority here because target connect is already small compared with the overlay delivery tail.
