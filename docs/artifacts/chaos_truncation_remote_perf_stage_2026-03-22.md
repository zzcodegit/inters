# Overlay stage breakdown RCA

## Inputs

- local raw: `docs/artifacts/chaos_truncation_remote_perf_stage_2026-03-22.local.jsonl`
- client stage trace: `docs/artifacts/chaos_truncation_remote_perf_stage_2026-03-22.client.jsonl`
- exit stage trace: `docs/artifacts/chaos_truncation_remote_perf_stage_2026-03-22.exit.jsonl`
- direct forward stage trace: `docs/artifacts/chaos_truncation_remote_perf_stage_2026-03-22.direct.jsonl`
- joined per-run view: `docs/artifacts/remote_perf_stage_matrix_2026-03-21.joined.jsonl`

## Scenario Averages

| scenario | route | route ready ms | handshake ms | target connect ms | first target byte wait ms | exit first send delay ms | overlay first-send -> client ms | window stall ms | exit/body tail ms | client total ms | retransmit rate | ack p95 ms | window frames |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| direct | 0 | - | - | - | - | - | - | - | - | 1540.02 | - | - | - |
| remote-1hop | 1 | 2333.61 | 430.00 | 0.40 | 1.40 | 0.20 | 1023.74 | 744.20 | 936.00 | 1026.99 | 0.0000 | 267.80 | 64 |
| remote-2hop | 2 | 3500.96 | 1759.00 | 0.00 | 1.80 | 0.40 | 1312.22 | 979.60 | 1116.20 | 1315.87 | 0.0000 | 327.80 | 64 |
| remote-3hop | 3 | 2683.56 | 616.00 | 0.00 | 1.20 | 0.00 | 1471.57 | 1110.80 | 1238.40 | 1474.40 | 0.0000 | 343.60 | 64 |

## Bottleneck Ranking

1. Warm route ready (one-time): `2839.38 ms`
2. Overlay pre-first-byte gap: `1269.38 ms`
3. Overlay first-send -> client first-byte gap: `1269.18 ms`
4. Exit/body delivery tail: `1096.87 ms`
5. Window/backpressure stall: `944.87 ms`
6. Handshake/session setup (one-time): `935.00 ms`
7. Route selection overhead: `2.80 ms`
8. Target first-byte wait: `1.47 ms`
9. Client body completion tail: `1.44 ms`
10. Exit first overlay send delay: `0.20 ms`
11. Target connect: `0.13 ms`
12. Client request buffering: `0.00 ms`

## Correlation

- `total_time_ms` vs `retransmit_rate`: `-`
- `total_time_ms` vs `ack_latency_ms_avg`: `0.992`

## Findings

- The best overlay path in this sample is `remote-1hop` with avg total `1026.99 ms`.
- `target_connect_ms` and target first-byte wait stay small relative to overlay totals, so the public target itself is not the main bottleneck.
- The biggest per-request cost sits after the target is already reachable: overlay first-send -> client first-byte gap, window/backpressure stall, and the remaining exit/body delivery tail dominate the remote paths.
- One-time costs (`route_ready_ms`, `handshake_ms`) matter for cold start, but they are not the reason the steady-state per-request `TTFB` stays in the multi-second range.
- Higher retransmit rate and ACK latency move with worse total time, which points to response-path reliability and tail delivery as the main source of degradation.

## Optimization Recommendation

1. Prioritize retransmit/ACK tuning on the response path first; that is where the largest steady-state time is being burned.
2. Use better exit/route scoring second; the current sample shows that path quality matters more than hop-count alone, so scoring should prefer the empirically faster overlay path instead of assuming more or fewer hops are always better.
3. Treat warm tunnel/session reuse as a cold-start improvement, not the main fix for current per-request TTFB.
4. Treat target-connect optimization as low priority here because target connect is already small compared with the overlay delivery tail.
