# Overlay stage breakdown RCA

## Inputs

- local raw: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_route_quality_stability.local.jsonl`
- client stage trace: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_route_quality_stability.client.jsonl`
- exit stage trace: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_route_quality_stability.exit.jsonl`
- direct forward stage trace: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_route_quality_stability.direct.jsonl`
- joined per-run view: `docs/artifacts/remote_perf_stage_matrix_2026-03-21.joined.jsonl`

## Scenario Averages

| scenario | route | route ready ms | handshake ms | target connect ms | first target byte wait ms | exit first send delay ms | overlay first-send -> client ms | window stall ms | exit/body tail ms | client total ms | retransmit rate | ack p95 ms | window frames |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| direct | 0 | - | - | - | - | - | - | - | - | 2334.21 | - | - | - |
| remote-1hop | 1 | 1935.85 | 355.00 | 0.20 | 1.20 | 0.00 | 960.03 | 717.80 | 846.00 | 962.20 | 0.0000 | 209.60 | 64 |
| remote-2hop | 2 | 1637.81 | 366.00 | 0.00 | 1.20 | 0.00 | 1127.43 | 862.60 | 996.40 | 1129.43 | 0.0055 | 285.60 | 64 |
| remote-3hop | 3 | 3688.16 | 1828.00 | 0.00 | 1.40 | 0.20 | 1628.78 | 1251.20 | 1389.40 | 1633.91 | 0.0444 | 404.40 | 64 |

## Bottleneck Ranking

1. Warm route ready (one-time): `2420.60 ms`
2. Overlay pre-first-byte gap: `1238.81 ms`
3. Overlay first-send -> client first-byte gap: `1238.75 ms`
4. Exit/body delivery tail: `1077.27 ms`
5. Window/backpressure stall: `943.87 ms`
6. Handshake/session setup (one-time): `849.67 ms`
7. Client body completion tail: `1.70 ms`
8. Route selection overhead: `1.60 ms`
9. Target first-byte wait: `1.27 ms`
10. Client request buffering: `0.67 ms`
11. Target connect: `0.07 ms`
12. Exit first overlay send delay: `0.07 ms`

## Correlation

- `total_time_ms` vs `retransmit_rate`: `0.661`
- `total_time_ms` vs `ack_latency_ms_avg`: `0.995`

## Findings

- The best overlay path in this sample is `remote-1hop` with avg total `962.20 ms`.
- `target_connect_ms` and target first-byte wait stay small relative to overlay totals, so the public target itself is not the main bottleneck.
- The biggest per-request cost sits after the target is already reachable: overlay first-send -> client first-byte gap, window/backpressure stall, and the remaining exit/body delivery tail dominate the remote paths.
- One-time costs (`route_ready_ms`, `handshake_ms`) matter for cold start, but they are not the reason the steady-state per-request `TTFB` stays in the multi-second range.
- Higher retransmit rate and ACK latency move with worse total time, which points to response-path reliability and tail delivery as the main source of degradation.

## Optimization Recommendation

1. Prioritize retransmit/ACK tuning on the response path first; that is where the largest steady-state time is being burned.
2. Use better exit/route scoring second; the current sample shows that path quality matters more than hop-count alone, so scoring should prefer the empirically faster overlay path instead of assuming more or fewer hops are always better.
3. Treat warm tunnel/session reuse as a cold-start improvement, not the main fix for current per-request TTFB.
4. Treat target-connect optimization as low priority here because target connect is already small compared with the overlay delivery tail.
