# Overlay stage breakdown RCA

## Inputs

- local raw: `docs/artifacts/stage5_shortpath_remote_perf_stage_2026-03-23.local.jsonl`
- client stage trace: `docs/artifacts/stage5_shortpath_remote_perf_stage_2026-03-23.client.jsonl`
- exit stage trace: `docs/artifacts/stage5_shortpath_remote_perf_stage_2026-03-23.exit.jsonl`
- direct forward stage trace: `docs/artifacts/stage5_shortpath_remote_perf_stage_2026-03-23.direct.jsonl`
- joined per-run view: `docs/artifacts/stage5_shortpath_remote_perf_stage_2026-03-23.joined.jsonl`

## Scenario Averages

| scenario | route | route ready ms | handshake ms | target connect ms | first target byte wait ms | exit first send delay ms | overlay first-send -> client ms | hard window stall ms | effective cap wait ms | exit/body tail ms | client total ms | retransmit rate | ack avg ms | ack p95 ms | max burst frames | pacing interval ms | pacing delay ms | burst prevented | effective cap avg | cap reduced | blocked by cap | window frames |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| direct | 0 | - | - | - | - | - | - | - | - | - | 1544.37 | - | - | - | - | - | - | - | - | - | - | - |
| remote-1hop | 1 | 2486.26 | 390.00 | 1.00 | 0.00 | 0.00 | 1247.29 | 102.20 | 0.00 | 1058.00 | 1249.32 | 0.0366 | 178.80 | 191.00 | 4 | 3.00 | 733.20 | 274 | 64 | 0 | 0 | 64 |
| remote-2hop | 2 | 1806.61 | 374.00 | 0.60 | 0.00 | 0.00 | 1121.54 | 46.20 | 0.00 | 998.00 | 1123.26 | 0.0000 | 183.80 | 193.60 | 4 | 3.00 | 712.80 | 268 | 64 | 0 | 0 | 64 |
| remote-3hop | 3 | 2499.64 | 485.00 | 0.80 | 0.00 | 0.00 | 1396.02 | 25.60 | 0.00 | 1209.40 | 1397.93 | 0.0000 | 235.00 | 246.60 | 4 | 4.00 | 966.00 | 275 | 64 | 0 | 0 | 64 |

## Bottleneck Ranking

1. Warm route ready (one-time): `2264.17 ms`
2. Overlay pre-first-byte gap: `1254.95 ms`
3. Overlay first-send -> client first-byte gap: `1254.95 ms`
4. Exit/body delivery tail: `1088.47 ms`
5. Handshake/session setup (one-time): `416.33 ms`
6. Window/backpressure stall: `58.00 ms`
7. Route selection overhead: `1.80 ms`
8. Client body completion tail: `1.09 ms`
9. Target connect: `0.80 ms`
10. Client request buffering: `0.00 ms`
11. Target first-byte wait: `0.00 ms`
12. Exit first overlay send delay: `0.00 ms`

## Correlation

- `total_time_ms` vs `retransmit_rate`: `0.361`
- `total_time_ms` vs `ack_latency_ms_avg`: `0.600`

## Findings

- The best overlay path in this sample is `remote-2hop` with avg total `1123.26 ms`.
- `target_connect_ms` and target first-byte wait stay small relative to overlay totals, so the public target itself is not the main bottleneck.
- The biggest per-request cost sits after the target is already reachable: overlay first-send -> client first-byte gap, window/backpressure stall, and the remaining exit/body delivery tail dominate the remote paths.
- `max_send_burst_frames`, `pacing_interval_ms_avg`, `pacing_delay_applied_ms_total`, and `burst_prevented_count` expose whether the exit is still dumping response frames in bursts or spreading them across the ACK window.
- `effective_inflight_cap_avg`, `inflight_cap_reduced_count`, `send_blocked_by_effective_cap`, and `effective_cap_wait_total_ms` expose whether the exit still drives the hard window directly or whether RTT/ACK pressure is actively shaping in-flight occupancy.
- One-time costs (`route_ready_ms`, `handshake_ms`) matter for cold start, but they are not the reason the steady-state per-request `TTFB` stays in the multi-second range.
- Higher retransmit rate and ACK latency move with worse total time, which points to response-path reliability and tail delivery as the main source of degradation.

## Optimization Recommendation

1. Prioritize retransmit/ACK tuning on the response path first; that is where the largest steady-state time is being burned.
2. Use better exit/route scoring second; the current sample shows that path quality matters more than hop-count alone, so scoring should prefer the empirically faster overlay path instead of assuming more or fewer hops are always better.
3. Treat warm tunnel/session reuse as a cold-start improvement, not the main fix for current per-request TTFB.
4. Treat target-connect optimization as low priority here because target connect is already small compared with the overlay delivery tail.
