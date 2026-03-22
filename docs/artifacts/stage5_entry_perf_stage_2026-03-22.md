# Overlay stage breakdown RCA

## Inputs

- local raw: `docs/artifacts/stage5_entry_perf_stage_2026-03-22.local.jsonl`
- client stage trace: `docs/artifacts/stage5_entry_perf_stage_2026-03-22.client.jsonl`
- exit stage trace: `docs/artifacts/stage5_entry_perf_stage_2026-03-22.exit.jsonl`
- direct forward stage trace: `docs/artifacts/stage5_entry_perf_stage_2026-03-22.direct.jsonl`
- joined per-run view: `docs/artifacts/stage5_entry_perf_stage_2026-03-22.joined.jsonl`

## Scenario Averages

| scenario | route | route ready ms | handshake ms | target connect ms | first target byte wait ms | exit first send delay ms | overlay first-send -> client ms | window stall ms | exit/body tail ms | client total ms | retransmit rate | ack p95 ms | window frames |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| direct | 0 | - | - | - | - | - | - | - | - | 1734.91 | - | - | - |
| remote-1hop | 1 | 2724.42 | 550.00 | 0.00 | 1.60 | 0.00 | 1096.04 | 817.40 | 966.40 | 1099.25 | 0.0048 | 306.40 | 64 |
| remote-2hop | 2 | 1624.82 | 417.00 | 0.00 | 1.40 | 0.00 | 1122.43 | 800.60 | 935.80 | 1125.36 | 0.0000 | 293.00 | 64 |
| remote-3hop | 3 | 4193.76 | 628.00 | 0.00 | 1.40 | 0.00 | 1521.19 | 1135.20 | 1208.60 | 1524.05 | 0.0000 | 325.80 | 64 |

## Bottleneck Ranking

1. Warm route ready (one-time): `2847.66 ms`
2. Overlay pre-first-byte gap: `1246.55 ms`
3. Overlay first-send -> client first-byte gap: `1246.55 ms`
4. Exit/body delivery tail: `1036.93 ms`
5. Window/backpressure stall: `917.73 ms`
6. Handshake/session setup (one-time): `531.67 ms`
7. Route selection overhead: `3.47 ms`
8. Client body completion tail: `1.53 ms`
9. Target first-byte wait: `1.47 ms`
10. Client request buffering: `0.00 ms`
11. Target connect: `0.00 ms`
12. Exit first overlay send delay: `0.00 ms`

## Correlation

- `total_time_ms` vs `retransmit_rate`: `0.071`
- `total_time_ms` vs `ack_latency_ms_avg`: `0.976`

## Findings

- The best overlay path in this sample is `remote-1hop` with avg total `1099.25 ms`.
- `target_connect_ms` and target first-byte wait stay small relative to overlay totals, so the public target itself is not the main bottleneck.
- The biggest per-request cost sits after the target is already reachable: overlay first-send -> client first-byte gap, window/backpressure stall, and the remaining exit/body delivery tail dominate the remote paths.
- One-time costs (`route_ready_ms`, `handshake_ms`) matter for cold start, but they are not the reason the steady-state per-request `TTFB` stays in the multi-second range.
- Higher retransmit rate and ACK latency move with worse total time, which points to response-path reliability and tail delivery as the main source of degradation.

## Optimization Recommendation

1. Prioritize retransmit/ACK tuning on the response path first; that is where the largest steady-state time is being burned.
2. Use better exit/route scoring second; the current sample shows that path quality matters more than hop-count alone, so scoring should prefer the empirically faster overlay path instead of assuming more or fewer hops are always better.
3. Treat warm tunnel/session reuse as a cold-start improvement, not the main fix for current per-request TTFB.
4. Treat target-connect optimization as low priority here because target connect is already small compared with the overlay delivery tail.
