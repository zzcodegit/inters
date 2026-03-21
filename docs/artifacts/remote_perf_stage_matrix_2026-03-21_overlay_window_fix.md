# Overlay stage breakdown RCA

## Inputs

- local raw: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_overlay_window_fix.local.jsonl`
- client stage trace: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_overlay_window_fix.client.jsonl`
- exit stage trace: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_overlay_window_fix.exit.jsonl`
- direct forward stage trace: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_overlay_window_fix.direct.jsonl`
- joined per-run view: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_overlay_window_fix.joined.jsonl`

## Scenario Averages

| scenario | route | route ready ms | handshake ms | target connect ms | first target byte wait ms | exit first send delay ms | overlay first-send -> client ms | window stall ms | exit/body tail ms | client total ms | retransmit rate | ack p95 ms | window frames |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| direct | 0 | - | - | - | - | - | - | - | - | 2284.78 | - | - | - |
| remote-1hop | 1 | 2888.66 | 373.00 | 0.20 | 1.20 | 0.40 | 1202.01 | 893.80 | 1026.80 | 1205.89 | 0.0214 | 311.20 | 64 |
| remote-2hop | 2 | 2251.38 | 522.00 | 0.00 | 1.40 | 0.00 | 1396.09 | 1051.80 | 1177.20 | 1402.47 | 0.1058 | 390.60 | 64 |
| remote-3hop | 3 | 2564.16 | 659.00 | 0.00 | 1.20 | 0.20 | 1579.78 | 1223.40 | 1328.00 | 1590.06 | 0.1820 | 394.00 | 64 |

## Bottleneck Ranking

1. Warm route ready (one-time): `2568.07 ms`
2. Overlay pre-first-byte gap: `1392.82 ms`
3. Overlay first-send -> client first-byte gap: `1392.62 ms`
4. Exit/body delivery tail: `1177.33 ms`
5. Window/backpressure stall: `1056.33 ms`
6. Handshake/session setup (one-time): `518.00 ms`
7. Client body completion tail: `5.31 ms`
8. Target first-byte wait: `1.27 ms`
9. Route selection overhead: `0.73 ms`
10. Exit first overlay send delay: `0.20 ms`
11. Target connect: `0.07 ms`
12. Client request buffering: `0.00 ms`

## Correlation

- `total_time_ms` vs `retransmit_rate`: `0.813`
- `total_time_ms` vs `ack_latency_ms_avg`: `0.967`

## Findings

- The best overlay path in this sample is `remote-1hop` with avg total `1205.89 ms`.
- `target_connect_ms` and target first-byte wait stay small relative to overlay totals, so the public target itself is not the main bottleneck.
- The biggest per-request cost sits after the target is already reachable: overlay first-send -> client first-byte gap, window/backpressure stall, and the remaining exit/body delivery tail dominate the remote paths.
- One-time costs (`route_ready_ms`, `handshake_ms`) matter for cold start, but they are not the reason the steady-state per-request `TTFB` stays in the multi-second range.
- Higher retransmit rate and ACK latency move with worse total time, which points to response-path reliability and tail delivery as the main source of degradation.

## Optimization Recommendation

1. Prioritize retransmit/ACK tuning on the response path first; that is where the largest steady-state time is being burned.
2. Use better exit/route scoring second; the current sample shows that path quality matters more than hop-count alone, so scoring should prefer the empirically faster overlay path instead of assuming more or fewer hops are always better.
3. Treat warm tunnel/session reuse as a cold-start improvement, not the main fix for current per-request TTFB.
4. Treat target-connect optimization as low priority here because target connect is already small compared with the overlay delivery tail.
