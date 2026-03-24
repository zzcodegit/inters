# Overlay stage breakdown RCA

## Inputs

- local raw: `docs/artifacts/remote_perf_stage_matrix_2026-03-21.local.jsonl`
- client stage trace: `docs/artifacts/remote_perf_stage_matrix_2026-03-21.client.jsonl`
- exit stage trace: `docs/artifacts/remote_perf_stage_matrix_2026-03-21.exit.jsonl`
- direct forward stage trace: `docs/artifacts/remote_perf_stage_matrix_2026-03-21.direct.jsonl`
- joined per-run view: `docs/artifacts/remote_perf_stage_matrix_2026-03-21.joined.jsonl`

## Scenario Averages

| scenario | route | route ready ms | handshake ms | target connect ms | first target byte wait ms | exit first send delay ms | overlay first-send -> client ms | hard window stall ms | effective cap wait ms | exit/body tail ms | client total ms | retransmit rate | retx timeout avg ms | retx timeout p95 ms | retx triggers | retx early | retx late | retx/RTT ratio | ack avg ms | ack p95 ms | max burst frames | pacing interval ms | pacing delay ms | burst prevented | effective cap avg | cap reduced | blocked by cap | congestion events | congestion duration ms | congestion cap limit | congestion pacing extra ms | window frames |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| direct | 0 | - | - | - | - | - | - | - | - | - | 1486.47 | - | - | - | - | - | - | - | - | - | - | - | - | - | - | - | - | - | - | - | - | - |
| remote-1hop | 1 | 2311.76 | 414.00 | 0.80 | 0.00 | 0.00 | 1222.55 | 28.00 | 69.60 | 1057.80 | 1229.43 | 0.0374 | 273.00 | 273.00 | 11 | 0 | 0 | 1.50 | 180.60 | 206.60 | 4 | 3.00 | 780.40 | 271 | 64 | 0 | 1 | 0 | 119.20 | 62 | 0.20 | 64 |
| remote-2hop | 2 | 2190.09 | 499.00 | 0.40 | 0.00 | 0.00 | 1162.85 | 23.40 | 3.60 | 1057.60 | 1164.53 | 0.0000 | - | - | 0 | 0 | 0 | - | 185.40 | 201.00 | 4 | 3.00 | 781.40 | 273 | 64 | 0 | 1 | 0 | 37.60 | 63 | 0.20 | 64 |
| remote-3hop | 3 | 2437.17 | 622.00 | 1.60 | 0.00 | 0.00 | 1524.15 | 120.20 | 65.00 | 1330.80 | 1527.14 | 0.0444 | 300.00 | 300.00 | 13 | 0 | 0 | 1.50 | 237.80 | 267.80 | 4 | 4.00 | 935.20 | 262 | 63 | 0 | 6 | 0 | 171.80 | 62 | 0.00 | 64 |
| remote-5hop | 5 | 3234.93 | 861.00 | 1.00 | 0.20 | 0.00 | 2208.35 | 288.40 | 313.40 | 1904.00 | 2211.46 | 0.3803 | 241.67 | 266.67 | 110 | 0 | 0 | 1.69 | 277.60 | 363.20 | 4 | 4.40 | 1107.40 | 233 | 64 | 0 | 27 | 1 | 646.80 | 62 | 0.00 | 64 |

## Bottleneck Ranking

1. Warm route ready (one-time): `2543.49 ms`
2. Overlay pre-first-byte gap: `1529.47 ms`
3. Overlay first-send -> client first-byte gap: `1529.47 ms`
4. Exit/body delivery tail: `1337.55 ms`
5. Handshake/session setup (one-time): `599.00 ms`
6. Window/backpressure stall: `115.00 ms`
7. Client body completion tail: `2.66 ms`
8. Route selection overhead: `2.20 ms`
9. Target connect: `0.95 ms`
10. Client request buffering: `0.95 ms`
11. Target first-byte wait: `0.05 ms`
12. Exit first overlay send delay: `0.00 ms`

## Correlation

- `total_time_ms` vs `retransmit_rate`: `0.451`
- `total_time_ms` vs `ack_latency_ms_avg`: `0.618`

## Findings

- The best overlay path in this sample is `remote-2hop` with avg total `1164.53 ms`.
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
