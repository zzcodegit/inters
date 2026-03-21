# Overlay stage breakdown RCA

## Inputs

- local raw: `docs/artifacts/remote_perf_stage_matrix_2026-03-21.local.jsonl`
- client stage trace: `docs/artifacts/remote_perf_stage_matrix_2026-03-21.client.jsonl`
- exit stage trace: `docs/artifacts/remote_perf_stage_matrix_2026-03-21.exit.jsonl`
- direct forward stage trace: `docs/artifacts/remote_perf_stage_matrix_2026-03-21.direct.jsonl`
- joined per-run view: `docs/artifacts/remote_perf_stage_matrix_2026-03-21.joined.jsonl`

## Scenario Averages

| scenario | route | route ready ms | handshake ms | target connect ms | first target byte wait ms | overlay pre-first-byte gap ms | exit/body tail ms | client total ms | retransmit rate | ack latency ms |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| direct | 0 | - | - | 1.30 | 6.35 | 378.51 | 782.25 | 2417.13 | - | - |
| remote-1hop | 1 | 8206.71 | 394.00 | 1.60 | 0.00 | 7881.21 | 7681.40 | 7884.02 | 0.0285 | 215.80 |
| remote-2hop | 2 | 11670.36 | 469.00 | 1.20 | 0.80 | 11026.88 | 10790.00 | 11034.96 | 0.2474 | 304.00 |
| remote-3hop | 3 | 15143.17 | 694.00 | 1.40 | 1.00 | 11363.22 | 11125.80 | 11366.82 | 0.2178 | 311.40 |

## Bottleneck Ranking

1. Warm route ready (one-time): `11673.41 ms`
2. Overlay pre-first-byte gap: `10090.44 ms`
3. Exit/body delivery tail: `9865.73 ms`
4. Handshake/session setup (one-time): `519.00 ms`
5. Client body completion tail: `2.83 ms`
6. Target connect: `1.40 ms`
7. Target first-byte wait: `0.60 ms`
8. Route selection overhead: `0.40 ms`
9. Client request buffering: `0.00 ms`

## Correlation

- `total_time_ms` vs `retransmit_rate`: `0.887`
- `total_time_ms` vs `ack_latency_ms_avg`: `0.999`

## Findings

- The best overlay path in this sample is `remote-1hop` with avg total `7884.02 ms`.
- `target_connect_ms` and target first-byte wait stay small relative to overlay totals, so the public target itself is not the main bottleneck.
- The biggest per-request cost sits after the target is already reachable: overlay pre-first-byte gap plus exit/body delivery tail dominate the remote paths.
- One-time costs (`route_ready_ms`, `handshake_ms`) matter for cold start, but they are not the reason the steady-state per-request `TTFB` stays in the multi-second range.
- Higher retransmit rate and ACK latency move with worse total time, which points to response-path reliability and tail delivery as the main source of degradation.

## Optimization Recommendation

1. Prioritize retransmit/ACK tuning on the response path first; that is where the largest steady-state time is being burned.
2. Use better exit/route scoring second; the current sample shows that path quality matters more than hop-count alone, so scoring should prefer the empirically faster overlay path instead of assuming more or fewer hops are always better.
3. Treat warm tunnel/session reuse as a cold-start improvement, not the main fix for current per-request TTFB.
4. Treat target-connect optimization as low priority here because target connect is already small compared with the overlay delivery tail.
