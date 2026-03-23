# Overlay stage breakdown RCA

## Inputs

- local raw: `docs/artifacts/stage5_inflight_remote_perf_stage_final_2026-03-23.local.jsonl`
- client stage trace: `docs/artifacts/stage5_inflight_remote_perf_stage_final_2026-03-23.client.jsonl`
- exit stage trace: `docs/artifacts/stage5_inflight_remote_perf_stage_final_2026-03-23.exit.jsonl`
- direct forward stage trace: `docs/artifacts/stage5_inflight_remote_perf_stage_final_2026-03-23.direct.jsonl`
- joined per-run view: `docs/artifacts/stage5_inflight_remote_perf_stage_final_2026-03-23.joined.jsonl`

## Scenario Averages

| scenario | route | route ready ms | handshake ms | target connect ms | first target byte wait ms | exit first send delay ms | overlay first-send -> client ms | hard window stall ms | effective cap wait ms | exit/body tail ms | client total ms | retransmit rate | ack avg ms | ack p95 ms | max burst frames | pacing interval ms | pacing delay ms | burst prevented | effective cap avg | cap reduced | blocked by cap | window frames |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| direct | 0 | - | - | - | - | - | - | - | - | - | 3783.09 | - | - | - | - | - | - | - | - | - | - | - |
| remote-1hop | 1 | 2161.23 | 408.00 | 1.00 | 0.20 | 0.00 | 1298.10 | 199.40 | 6.00 | 1147.20 | 1300.41 | 0.0858 | 216.20 | 340.20 | 4 | 3.00 | 740.20 | 268 | 63 | 0 | 0 | 64 |
| remote-2hop | 2 | 3024.50 | 383.00 | 1.00 | 0.40 | 0.20 | 1198.85 | 105.80 | 0.80 | 1028.00 | 1201.48 | 0.0443 | 193.20 | 263.60 | 4 | 3.00 | 734.20 | 274 | 64 | 0 | 0 | 64 |
| remote-3hop | 3 | 2324.14 | 481.00 | 0.60 | 0.00 | 0.00 | 1455.56 | 24.20 | 198.00 | 1269.40 | 1461.79 | 0.0014 | 239.40 | 269.80 | 4 | 4.00 | 818.80 | 230 | 61 | 1 | 38 | 64 |

## Bottleneck Ranking

1. Warm route ready (one-time): `2503.29 ms`
2. Overlay pre-first-byte gap: `1317.57 ms`
3. Overlay first-send -> client first-byte gap: `1317.50 ms`
4. Exit/body delivery tail: `1148.20 ms`
5. Handshake/session setup (one-time): `424.00 ms`
6. Window/backpressure stall: `109.80 ms`
7. Client body completion tail: `2.59 ms`
8. Route selection overhead: `2.07 ms`
9. Target connect: `0.87 ms`
10. Target first-byte wait: `0.20 ms`
11. Exit first overlay send delay: `0.07 ms`
12. Client request buffering: `0.00 ms`

## Correlation

- `total_time_ms` vs `retransmit_rate`: `0.624`
- `total_time_ms` vs `ack_latency_ms_avg`: `0.985`

## Findings

- The best overlay path in this sample is `remote-2hop` with avg total `1201.48 ms`.
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
