# Overlay stage breakdown RCA

## Inputs

- local raw: `docs/artifacts/chaos_terminal_payload_tail_remote_perf_stage_2026-03-22.local.jsonl`
- client stage trace: `docs/artifacts/chaos_terminal_payload_tail_remote_perf_stage_2026-03-22.client.jsonl`
- exit stage trace: `docs/artifacts/chaos_terminal_payload_tail_remote_perf_stage_2026-03-22.exit.jsonl`
- direct forward stage trace: `docs/artifacts/chaos_terminal_payload_tail_remote_perf_stage_2026-03-22.direct.jsonl`
- joined per-run view: `docs/artifacts/chaos_terminal_payload_tail_remote_perf_stage_2026-03-22.joined.jsonl`

## Scenario Averages

| scenario | route | route ready ms | handshake ms | target connect ms | first target byte wait ms | exit first send delay ms | overlay first-send -> client ms | window stall ms | exit/body tail ms | client total ms | retransmit rate | ack p95 ms | window frames |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| direct | 0 | - | - | - | - | - | - | - | - | 2548.03 | - | - | - |
| remote-1hop | 1 | 2693.56 | 437.00 | 0.00 | 1.20 | 0.00 | 1244.53 | 927.00 | 1056.00 | 1247.00 | 0.0083 | 375.60 | 64 |
| remote-2hop | 2 | 7560.15 | 5439.00 | 0.40 | 0.20 | 0.20 | 1642.40 | 1327.40 | 1510.20 | 1644.54 | 0.0858 | 551.40 | 64 |
| remote-3hop | 3 | 4018.11 | 1122.00 | 0.00 | 1.60 | 0.20 | 1801.47 | 1425.60 | 1570.40 | 1804.78 | 0.0510 | 604.80 | 64 |

## Bottleneck Ranking

1. Warm route ready (one-time): `4757.28 ms`
2. Handshake/session setup (one-time): `2332.67 ms`
3. Overlay pre-first-byte gap: `1562.93 ms`
4. Overlay first-send -> client first-byte gap: `1562.80 ms`
5. Exit/body delivery tail: `1378.87 ms`
6. Window/backpressure stall: `1226.67 ms`
7. Route selection overhead: `2.53 ms`
8. Client body completion tail: `1.38 ms`
9. Target first-byte wait: `1.00 ms`
10. Target connect: `0.13 ms`
11. Exit first overlay send delay: `0.13 ms`
12. Client request buffering: `0.00 ms`

## Correlation

- `total_time_ms` vs `retransmit_rate`: `0.917`
- `total_time_ms` vs `ack_latency_ms_avg`: `0.966`

## Findings

- The best overlay path in this sample is `remote-1hop` with avg total `1247.00 ms`.
- `target_connect_ms` and target first-byte wait stay small relative to overlay totals, so the public target itself is not the main bottleneck.
- The biggest per-request cost sits after the target is already reachable: overlay first-send -> client first-byte gap, window/backpressure stall, and the remaining exit/body delivery tail dominate the remote paths.
- One-time costs (`route_ready_ms`, `handshake_ms`) matter for cold start, but they are not the reason the steady-state per-request `TTFB` stays in the multi-second range.
- Higher retransmit rate and ACK latency move with worse total time, which points to response-path reliability and tail delivery as the main source of degradation.

## Optimization Recommendation

1. Prioritize retransmit/ACK tuning on the response path first; that is where the largest steady-state time is being burned.
2. Use better exit/route scoring second; the current sample shows that path quality matters more than hop-count alone, so scoring should prefer the empirically faster overlay path instead of assuming more or fewer hops are always better.
3. Treat warm tunnel/session reuse as a cold-start improvement, not the main fix for current per-request TTFB.
4. Treat target-connect optimization as low priority here because target connect is already small compared with the overlay delivery tail.
