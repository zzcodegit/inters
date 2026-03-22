# Overlay stage breakdown RCA

## Inputs

- local raw: `docs/artifacts/chaos_duplicate_payload_tail_remote_perf_stage_2026-03-22.local.jsonl`
- client stage trace: `docs/artifacts/chaos_duplicate_payload_tail_remote_perf_stage_2026-03-22.client.jsonl`
- exit stage trace: `docs/artifacts/chaos_duplicate_payload_tail_remote_perf_stage_2026-03-22.exit.jsonl`
- direct forward stage trace: `docs/artifacts/chaos_duplicate_payload_tail_remote_perf_stage_2026-03-22.direct.jsonl`
- joined per-run view: `docs/artifacts/chaos_duplicate_payload_tail_remote_perf_stage_2026-03-22.joined.jsonl`

## Scenario Averages

| scenario | route | route ready ms | handshake ms | target connect ms | first target byte wait ms | exit first send delay ms | overlay first-send -> client ms | window stall ms | exit/body tail ms | client total ms | retransmit rate | ack p95 ms | window frames |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| direct | 0 | - | - | - | - | - | - | - | - | 1462.08 | - | - | - |
| remote-1hop | 1 | 2342.74 | 545.00 | 0.00 | 1.60 | 0.20 | 1240.68 | 994.40 | 1116.20 | 1243.29 | 0.0883 | 434.60 | 64 |
| remote-2hop | 2 | 2661.14 | 1245.00 | 0.20 | 1.20 | 0.00 | 1181.95 | 937.80 | 1118.40 | 1184.17 | 0.1218 | 413.00 | 64 |
| remote-3hop | 3 | 2293.77 | 505.00 | 0.00 | 0.80 | 0.60 | 1259.06 | 965.20 | 1058.40 | 1261.25 | 0.0000 | 265.80 | 64 |

## Bottleneck Ranking

1. Warm route ready (one-time): `2432.55 ms`
2. Overlay pre-first-byte gap: `1227.50 ms`
3. Overlay first-send -> client first-byte gap: `1227.23 ms`
4. Exit/body delivery tail: `1097.67 ms`
5. Window/backpressure stall: `965.80 ms`
6. Handshake/session setup (one-time): `765.00 ms`
7. Target first-byte wait: `1.20 ms`
8. Route selection overhead: `1.07 ms`
9. Client body completion tail: `0.80 ms`
10. Exit first overlay send delay: `0.27 ms`
11. Target connect: `0.07 ms`
12. Client request buffering: `0.00 ms`

## Correlation

- `total_time_ms` vs `retransmit_rate`: `0.904`
- `total_time_ms` vs `ack_latency_ms_avg`: `0.998`

## Findings

- The best overlay path in this sample is `remote-2hop` with avg total `1184.17 ms`.
- `target_connect_ms` and target first-byte wait stay small relative to overlay totals, so the public target itself is not the main bottleneck.
- The biggest per-request cost sits after the target is already reachable: overlay first-send -> client first-byte gap, window/backpressure stall, and the remaining exit/body delivery tail dominate the remote paths.
- One-time costs (`route_ready_ms`, `handshake_ms`) matter for cold start, but they are not the reason the steady-state per-request `TTFB` stays in the multi-second range.
- Higher retransmit rate and ACK latency move with worse total time, which points to response-path reliability and tail delivery as the main source of degradation.

## Optimization Recommendation

1. Prioritize retransmit/ACK tuning on the response path first; that is where the largest steady-state time is being burned.
2. Use better exit/route scoring second; the current sample shows that path quality matters more than hop-count alone, so scoring should prefer the empirically faster overlay path instead of assuming more or fewer hops are always better.
3. Treat warm tunnel/session reuse as a cold-start improvement, not the main fix for current per-request TTFB.
4. Treat target-connect optimization as low priority here because target connect is already small compared with the overlay delivery tail.
