# Overlay stage breakdown RCA

## Inputs

- local raw: `docs/artifacts/recovery_coupled_control_remote_perf_stage_2026-03-28.local.jsonl`
- client stage trace: `docs/artifacts/recovery_coupled_control_remote_perf_stage_2026-03-28.client.jsonl`
- exit stage trace: `docs/artifacts/recovery_coupled_control_remote_perf_stage_2026-03-28.exit.jsonl`
- direct forward stage trace: `docs/artifacts/recovery_coupled_control_remote_perf_stage_2026-03-28.direct.jsonl`
- joined per-run view: `docs/artifacts/recovery_coupled_control_remote_perf_stage_2026-03-28.joined.jsonl`

## Scenario Averages

| scenario | route | route ready ms | handshake ms | target connect ms | first target byte wait ms | exit first send delay ms | overlay first-send -> client ms | hard window stall ms | effective cap wait ms | exit/body tail ms | client total ms | retransmit rate | retx timeout avg ms | retx timeout p95 ms | retx triggers | retx early | retx late | retx/RTT ratio | ack avg ms | ack p95 ms | max burst frames | pacing interval ms | pacing delay ms | burst prevented | effective cap avg | cap reduced | blocked by cap | congestion events | congestion duration ms | congestion cap limit | congestion pacing extra ms | window frames |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| direct | 0 | - | - | 1.95 | 3.60 | - | - | - | - | 529.72 | 1594.75 | - | - | - | - | - | - | - | - | - | - | - | - | - | - | - | - | - | - | - | - | - |
| remote-1hop | 1 | 2204.17 | 370.00 | 1.00 | 0.40 | 0.00 | 1138.14 | 45.60 | 0.00 | 906.20 | 1140.44 | 0.0000 | - | - | 0 | 0 | 0 | - | 184.40 | 188.80 | 4 | 3.00 | 711.20 | 264 | 64 | 0 | 0 | 0 | 0.00 | 64 | 0.00 | 64 |
| remote-2hop | 2 | 1757.16 | 378.00 | 1.20 | 0.00 | 0.00 | 1102.24 | 51.40 | 11.00 | 966.40 | 1104.49 | 0.0159 | 271.00 | 272.00 | 5 | 0 | 0 | 1.50 | 182.40 | 200.60 | 4 | 3.00 | 729.20 | 274 | 64 | 0 | 0 | 0 | 17.00 | 64 | 0.00 | 64 |
| remote-3hop | 3 | 1921.90 | 478.00 | 2.20 | 0.00 | 0.00 | 1394.13 | 18.60 | 0.00 | 1209.40 | 1397.26 | 0.0000 | - | - | 0 | 0 | 0 | - | 234.60 | 244.80 | 4 | 4.00 | 975.80 | 279 | 64 | 0 | 0 | 0 | 0.00 | 64 | 0.00 | 64 |
| remote-5hop | 5 | 2843.08 | 665.00 | 1.40 | 0.40 | 0.00 | 2078.74 | 186.20 | 0.00 | 1752.00 | 2081.37 | 0.0374 | 488.00 | 488.00 | 11 | 0 | 0 | 1.50 | 320.20 | 334.40 | 4 | 5.00 | 1378.40 | 276 | 64 | 0 | 0 | 0 | 0.00 | 64 | 0.00 | 64 |

## Bottleneck Ranking

1. Warm route ready (one-time): `2181.58 ms`
2. Overlay pre-first-byte gap: `1428.31 ms`
3. Overlay first-send -> client first-byte gap: `1428.31 ms`
4. Exit/body delivery tail: `1208.50 ms`
5. Handshake/session setup (one-time): `472.75 ms`
6. Window/backpressure stall: `75.45 ms`
7. Target connect: `1.45 ms`
8. Route selection overhead: `1.05 ms`
9. Client body completion tail: `0.93 ms`
10. Target first-byte wait: `0.20 ms`
11. Client request buffering: `0.00 ms`
12. Exit first overlay send delay: `0.00 ms`

## Correlation

- `total_time_ms` vs `retransmit_rate`: `0.452`
- `total_time_ms` vs `ack_latency_ms_avg`: `0.949`

## Findings

- The best overlay path in this sample is `remote-2hop` with avg total `1104.49 ms`.
- `target_connect_ms` and target first-byte wait stay small relative to overlay totals, so the public target itself is not the main bottleneck.
- The biggest per-request cost sits after the target is already reachable: overlay first-send -> client first-byte gap, window/backpressure stall, and the remaining exit/body delivery tail dominate the remote paths.
- `max_send_burst_frames`, `pacing_interval_ms_avg`, `pacing_delay_applied_ms_total`, and `burst_prevented_count` expose whether the exit is still dumping response frames in bursts or spreading them across the ACK window.
- `effective_inflight_cap_avg`, `inflight_cap_reduced_count`, `send_blocked_by_effective_cap`, and `effective_cap_wait_total_ms` expose whether the exit still drives the hard window directly or whether RTT/ACK pressure is actively shaping in-flight occupancy.
- `congestion_events`, `congestion_duration_ms`, `congestion_cap_limit`, and `congestion_pacing_extra_ms` expose whether the lightweight congestion response actually entered a non-default state on the slower paths.
- One-time costs (`route_ready_ms`, `handshake_ms`) matter for cold start, but they are not the reason the steady-state per-request `TTFB` stays in the multi-second range.
- Higher retransmit rate and ACK latency move with worse total time, which points to response-path reliability and tail delivery as the main source of degradation.

## Optimization Recommendation

1. Prioritize retransmit/ACK tuning on the response path first; that is where the largest steady-state time is being burned.
2. Use better exit/route scoring second; the current sample shows that path quality matters more than hop-count alone, so scoring should prefer the empirically faster overlay path instead of assuming more or fewer hops are always better.
3. Treat warm tunnel/session reuse as a cold-start improvement, not the main fix for current per-request TTFB.
4. Treat target-connect optimization as low priority here because target connect is already small compared with the overlay delivery tail.
