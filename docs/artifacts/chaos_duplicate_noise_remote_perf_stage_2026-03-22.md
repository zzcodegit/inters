# Overlay stage breakdown RCA

## Inputs

- local raw: `docs/artifacts/chaos_duplicate_noise_remote_perf_stage_2026-03-22.local.jsonl`
- client stage trace: `docs/artifacts/chaos_duplicate_noise_remote_perf_stage_2026-03-22.client.jsonl`
- exit stage trace: `docs/artifacts/chaos_duplicate_noise_remote_perf_stage_2026-03-22.exit.jsonl`
- direct forward stage trace: `docs/artifacts/chaos_duplicate_noise_remote_perf_stage_2026-03-22.direct.jsonl`
- joined per-run view: `docs/artifacts/chaos_duplicate_noise_remote_perf_stage_2026-03-22.joined.jsonl`

## Scenario Averages

| scenario | route | route ready ms | handshake ms | target connect ms | first target byte wait ms | exit first send delay ms | overlay first-send -> client ms | window stall ms | exit/body tail ms | client total ms | retransmit rate | ack p95 ms | window frames |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| direct | 0 | - | - | - | - | - | - | - | - | 1236.66 | - | - | - |
| remote-1hop | 1 | 1898.61 | 359.00 | 0.40 | 2.80 | 0.40 | 943.26 | 710.60 | 843.40 | 947.47 | 0.0000 | 203.80 | 64 |
| remote-2hop | 2 | 1691.71 | 379.00 | 0.00 | 1.40 | 0.20 | 1022.63 | 771.80 | 904.80 | 1024.92 | 0.0000 | 223.60 | 64 |
| remote-3hop | 3 | 1735.31 | 475.00 | 0.40 | 1.20 | 0.20 | 1233.15 | 943.00 | 1056.80 | 1235.68 | 0.0000 | 264.40 | 64 |

## Bottleneck Ranking

1. Warm route ready (one-time): `1775.21 ms`
2. Overlay pre-first-byte gap: `1066.62 ms`
3. Overlay first-send -> client first-byte gap: `1066.35 ms`
4. Exit/body delivery tail: `935.00 ms`
5. Window/backpressure stall: `808.47 ms`
6. Handshake/session setup (one-time): `404.33 ms`
7. Target first-byte wait: `1.80 ms`
8. Route selection overhead: `1.00 ms`
9. Client body completion tail: `0.67 ms`
10. Target connect: `0.27 ms`
11. Exit first overlay send delay: `0.27 ms`
12. Client request buffering: `0.00 ms`

## Correlation

- `total_time_ms` vs `retransmit_rate`: `-`
- `total_time_ms` vs `ack_latency_ms_avg`: `0.999`

## Findings

- The best overlay path in this sample is `remote-1hop` with avg total `947.47 ms`.
- `target_connect_ms` and target first-byte wait stay small relative to overlay totals, so the public target itself is not the main bottleneck.
- The biggest per-request cost sits after the target is already reachable: overlay first-send -> client first-byte gap, window/backpressure stall, and the remaining exit/body delivery tail dominate the remote paths.
- One-time costs (`route_ready_ms`, `handshake_ms`) matter for cold start, but they are not the reason the steady-state per-request `TTFB` stays in the multi-second range.
- Higher retransmit rate and ACK latency move with worse total time, which points to response-path reliability and tail delivery as the main source of degradation.

## Optimization Recommendation

1. Prioritize retransmit/ACK tuning on the response path first; that is where the largest steady-state time is being burned.
2. Use better exit/route scoring second; the current sample shows that path quality matters more than hop-count alone, so scoring should prefer the empirically faster overlay path instead of assuming more or fewer hops are always better.
3. Treat warm tunnel/session reuse as a cold-start improvement, not the main fix for current per-request TTFB.
4. Treat target-connect optimization as low priority here because target connect is already small compared with the overlay delivery tail.
