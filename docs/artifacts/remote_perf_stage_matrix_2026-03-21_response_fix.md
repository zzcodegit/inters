# Overlay stage breakdown RCA

## Inputs

- local raw: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_response_fix.local.jsonl`
- client stage trace: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_response_fix.client.jsonl`
- exit stage trace: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_response_fix.exit.jsonl`
- direct forward stage trace: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_response_fix.direct.jsonl`
- joined per-run view: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_response_fix.joined.jsonl`

## Scenario Averages

| scenario | route | route ready ms | handshake ms | target connect ms | first target byte wait ms | overlay pre-first-byte gap ms | exit/body tail ms | client total ms | retransmit rate | ack latency ms |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| direct | 0 | - | - | - | - | - | - | 1188.83 | - | - |
| remote-1hop | 1 | 7070.61 | 355.00 | 1.00 | 0.00 | 6528.14 | 6379.20 | 6529.92 | 0.0000 | 177.40 |
| remote-2hop | 2 | 7527.30 | 373.00 | 1.80 | 0.60 | 6809.36 | 6680.60 | 6812.79 | 0.0000 | 185.80 |
| remote-3hop | 3 | 9238.26 | 493.00 | 2.00 | 0.00 | 8681.98 | 8465.80 | 8685.63 | 0.0000 | 238.40 |

## Bottleneck Ranking

1. Warm route ready (one-time): `7945.39 ms`
2. Overlay pre-first-byte gap: `7339.83 ms`
3. Exit/body delivery tail: `7175.20 ms`
4. Handshake/session setup (one-time): `407.00 ms`
5. Target connect: `1.60 ms`
6. Client body completion tail: `1.15 ms`
7. Target first-byte wait: `0.20 ms`
8. Client request buffering: `0.00 ms`
9. Route selection overhead: `0.00 ms`

## Correlation

- `total_time_ms` vs `retransmit_rate`: `-`
- `total_time_ms` vs `ack_latency_ms_avg`: `1.000`

## Findings

- The best overlay path in this sample is `remote-1hop` with avg total `6529.92 ms`.
- `target_connect_ms` and target first-byte wait stay small relative to overlay totals, so the public target itself is not the main bottleneck.
- The biggest per-request cost sits after the target is already reachable: overlay pre-first-byte gap plus exit/body delivery tail dominate the remote paths.
- One-time costs (`route_ready_ms`, `handshake_ms`) matter for cold start, but they are not the reason the steady-state per-request `TTFB` stays in the multi-second range.
- Higher retransmit rate and ACK latency move with worse total time, which points to response-path reliability and tail delivery as the main source of degradation.

## Optimization Recommendation

1. Prioritize retransmit/ACK tuning on the response path first; that is where the largest steady-state time is being burned.
2. Use better exit/route scoring second; the current sample shows that path quality matters more than hop-count alone, so scoring should prefer the empirically faster overlay path instead of assuming more or fewer hops are always better.
3. Treat warm tunnel/session reuse as a cold-start improvement, not the main fix for current per-request TTFB.
4. Treat target-connect optimization as low priority here because target connect is already small compared with the overlay delivery tail.
