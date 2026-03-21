# Overlay stage breakdown RCA

## Inputs

- local raw: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_route_quality_policy.local.jsonl`
- client stage trace: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_route_quality_policy.client.jsonl`
- exit stage trace: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_route_quality_policy.exit.jsonl`
- direct forward stage trace: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_route_quality_policy.direct.jsonl`
- joined per-run view: `docs/artifacts/remote_perf_stage_matrix_2026-03-21.joined.jsonl`

## Scenario Averages

| scenario | route | route ready ms | handshake ms | target connect ms | first target byte wait ms | exit first send delay ms | overlay first-send -> client ms | window stall ms | exit/body tail ms | client total ms | retransmit rate | ack p95 ms | window frames |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| direct | 0 | - | - | - | - | - | - | - | - | 1405.84 | - | - | - |
| remote-1hop | 1 | 1887.41 | 360.00 | 0.20 | 1.00 | 0.40 | 998.85 | 739.60 | 875.60 | 1001.26 | 0.0000 | 232.80 | 64 |
| remote-2hop | 2 | 2190.30 | 385.00 | 0.00 | 1.60 | 0.40 | 1164.96 | 926.00 | 1056.80 | 1167.72 | 0.0775 | 389.00 | 64 |
| remote-3hop | 3 | 1760.23 | 475.00 | 0.80 | 1.00 | 0.20 | 1258.14 | 959.00 | 1057.20 | 1260.87 | 0.0000 | 267.00 | 64 |

## Bottleneck Ranking

1. Warm route ready (one-time): `1945.98 ms`
2. Overlay pre-first-byte gap: `1140.98 ms`
3. Overlay first-send -> client first-byte gap: `1140.65 ms`
4. Exit/body delivery tail: `996.53 ms`
5. Window/backpressure stall: `874.87 ms`
6. Handshake/session setup (one-time): `406.67 ms`
7. Target first-byte wait: `1.20 ms`
8. Route selection overhead: `1.00 ms`
9. Client body completion tail: `0.77 ms`
10. Target connect: `0.33 ms`
11. Exit first overlay send delay: `0.33 ms`
12. Client request buffering: `0.00 ms`

## Correlation

- `total_time_ms` vs `retransmit_rate`: `0.742`
- `total_time_ms` vs `ack_latency_ms_avg`: `0.985`

## Findings

- The best overlay path in this sample is `remote-1hop` with avg total `1001.26 ms`.
- `target_connect_ms` and target first-byte wait stay small relative to overlay totals, so the public target itself is not the main bottleneck.
- The biggest per-request cost sits after the target is already reachable: overlay first-send -> client first-byte gap, window/backpressure stall, and the remaining exit/body delivery tail dominate the remote paths.
- One-time costs (`route_ready_ms`, `handshake_ms`) matter for cold start, but they are not the reason the steady-state per-request `TTFB` stays in the multi-second range.
- Higher retransmit rate and ACK latency move with worse total time, which points to response-path reliability and tail delivery as the main source of degradation.

## Optimization Recommendation

1. Prioritize retransmit/ACK tuning on the response path first; that is where the largest steady-state time is being burned.
2. Use better exit/route scoring second; the current sample shows that path quality matters more than hop-count alone, so scoring should prefer the empirically faster overlay path instead of assuming more or fewer hops are always better.
3. Treat warm tunnel/session reuse as a cold-start improvement, not the main fix for current per-request TTFB.
4. Treat target-connect optimization as low priority here because target connect is already small compared with the overlay delivery tail.
