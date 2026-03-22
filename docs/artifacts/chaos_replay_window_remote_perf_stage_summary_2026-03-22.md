# Overlay stage breakdown RCA

## Inputs

- local raw: `docs/artifacts/remote_perf_stage_matrix_2026-03-21.local.jsonl`
- client stage trace: `docs/artifacts/remote_perf_stage_matrix_2026-03-21.client.jsonl`
- exit stage trace: `docs/artifacts/remote_perf_stage_matrix_2026-03-21.exit.jsonl`
- direct forward stage trace: `docs/artifacts/remote_perf_stage_matrix_2026-03-21.direct.jsonl`
- joined per-run view: `docs/artifacts/remote_perf_stage_matrix_2026-03-21.joined.jsonl`

## Scenario Averages

| scenario | route | route ready ms | handshake ms | target connect ms | first target byte wait ms | exit first send delay ms | overlay first-send -> client ms | window stall ms | exit/body tail ms | client total ms | retransmit rate | ack p95 ms | window frames |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| direct | 0 | - | - | - | - | - | - | - | - | 8648.46 | - | - | - |
| remote-1hop | 1 | 6351.27 | 1784.00 | 0.20 | 2.00 | 0.40 | 1180.93 | 903.40 | 1056.40 | 1184.89 | 0.0000 | 296.80 | 64 |
| remote-2hop | 2 | 7005.51 | 1387.00 | 0.00 | 2.00 | 0.20 | 1291.90 | 1012.60 | 1146.40 | 1295.31 | 0.0443 | 369.00 | 64 |
| remote-3hop | 3 | 10196.68 | 5732.00 | 0.40 | 0.60 | 0.20 | 2118.81 | 1571.60 | 1723.00 | 2121.68 | 0.1448 | 676.20 | 64 |

## Bottleneck Ranking

1. Warm route ready (one-time): `7851.16 ms`
2. Handshake/session setup (one-time): `2967.67 ms`
3. Overlay pre-first-byte gap: `1530.82 ms`
4. Overlay first-send -> client first-byte gap: `1530.55 ms`
5. Exit/body delivery tail: `1308.60 ms`
6. Window/backpressure stall: `1162.53 ms`
7. Route selection overhead: `2.33 ms`
8. Target first-byte wait: `1.53 ms`
9. Client body completion tail: `1.41 ms`
10. Exit first overlay send delay: `0.27 ms`
11. Target connect: `0.20 ms`
12. Client request buffering: `0.00 ms`

## Correlation

- `total_time_ms` vs `retransmit_rate`: `0.792`
- `total_time_ms` vs `ack_latency_ms_avg`: `0.942`

## Findings

- The best overlay path in this sample is `remote-1hop` with avg total `1184.89 ms`.
- `target_connect_ms` and target first-byte wait stay small relative to overlay totals, so the public target itself is not the main bottleneck.
- The biggest per-request cost sits after the target is already reachable: overlay first-send -> client first-byte gap, window/backpressure stall, and the remaining exit/body delivery tail dominate the remote paths.
- One-time costs (`route_ready_ms`, `handshake_ms`) matter for cold start, but they are not the reason the steady-state per-request `TTFB` stays in the multi-second range.
- Higher retransmit rate and ACK latency move with worse total time, which points to response-path reliability and tail delivery as the main source of degradation.

## Optimization Recommendation

1. Prioritize retransmit/ACK tuning on the response path first; that is where the largest steady-state time is being burned.
2. Use better exit/route scoring second; the current sample shows that path quality matters more than hop-count alone, so scoring should prefer the empirically faster overlay path instead of assuming more or fewer hops are always better.
3. Treat warm tunnel/session reuse as a cold-start improvement, not the main fix for current per-request TTFB.
4. Treat target-connect optimization as low priority here because target connect is already small compared with the overlay delivery tail.
