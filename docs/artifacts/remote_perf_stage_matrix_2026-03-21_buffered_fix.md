# Overlay stage breakdown RCA

## Inputs

- local raw: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_buffered_fix.local.jsonl`
- client stage trace: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_buffered_fix.client.jsonl`
- exit stage trace: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_buffered_fix.exit.jsonl`
- direct forward stage trace: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_buffered_fix.direct.jsonl`
- joined per-run view: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_buffered_fix.joined.jsonl`

## Scenario Averages

| scenario | route | route ready ms | handshake ms | target connect ms | first target byte wait ms | overlay pre-first-byte gap ms | exit/body tail ms | client total ms | retransmit rate | ack latency ms |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| direct | 0 | - | - | - | - | - | - | 1846.10 | - | - |
| remote-1hop | 1 | 11386.38 | 393.00 | 1.40 | 0.20 | 11553.99 | 11278.40 | 11556.65 | 0.3325 | 314.80 |
| remote-2hop | 2 | 11772.63 | 472.00 | 0.80 | 0.80 | 11873.63 | 11667.80 | 11876.86 | 0.3340 | 328.60 |
| remote-3hop | 3 | 20786.58 | 5533.00 | 1.80 | 1.80 | 14483.25 | 13967.20 | 14495.60 | 0.6581 | 393.80 |

## Bottleneck Ranking

1. Warm route ready (one-time): `14648.53 ms`
2. Overlay pre-first-byte gap: `12636.96 ms`
3. Exit/body delivery tail: `12304.47 ms`
4. Handshake/session setup (one-time): `2132.67 ms`
5. Client body completion tail: `3.81 ms`
6. Target connect: `1.33 ms`
7. Target first-byte wait: `0.93 ms`
8. Route selection overhead: `0.67 ms`
9. Client request buffering: `0.00 ms`

## Correlation

- `total_time_ms` vs `retransmit_rate`: `0.975`
- `total_time_ms` vs `ack_latency_ms_avg`: `0.987`

## Findings

- The best overlay path in this sample is `remote-1hop` with avg total `11556.65 ms`.
- `target_connect_ms` and target first-byte wait stay small relative to overlay totals, so the public target itself is not the main bottleneck.
- The biggest per-request cost sits after the target is already reachable: overlay pre-first-byte gap plus exit/body delivery tail dominate the remote paths.
- One-time costs (`route_ready_ms`, `handshake_ms`) matter for cold start, but they are not the reason the steady-state per-request `TTFB` stays in the multi-second range.
- Higher retransmit rate and ACK latency move with worse total time, which points to response-path reliability and tail delivery as the main source of degradation.

## Optimization Recommendation

1. Prioritize retransmit/ACK tuning on the response path first; that is where the largest steady-state time is being burned.
2. Use better exit/route scoring second; the current sample shows that path quality matters more than hop-count alone, so scoring should prefer the empirically faster overlay path instead of assuming more or fewer hops are always better.
3. Treat warm tunnel/session reuse as a cold-start improvement, not the main fix for current per-request TTFB.
4. Treat target-connect optimization as low priority here because target connect is already small compared with the overlay delivery tail.
