# Overlay stage breakdown RCA

## Inputs

- local raw: `docs/artifacts/retransmit_backoff_remote_perf_stage_2026-03-23.local.jsonl`
- client stage trace: `docs/artifacts/retransmit_backoff_remote_perf_stage_2026-03-23.client.jsonl`
- exit stage trace: `docs/artifacts/retransmit_backoff_remote_perf_stage_2026-03-23.exit.jsonl`
- direct forward stage trace: `docs/artifacts/retransmit_backoff_remote_perf_stage_2026-03-23.direct.jsonl`
- joined per-run view: `docs/artifacts/retransmit_backoff_remote_perf_stage_2026-03-23.joined.jsonl`

## Scenario Averages

| scenario | route | route ready ms | handshake ms | target connect ms | first target byte wait ms | exit first send delay ms | overlay first-send -> client ms | hard window stall ms | effective cap wait ms | exit/body tail ms | client total ms | retransmit rate | retx timeout avg ms | retx timeout p95 ms | retx triggers | retx early | retx late | retx/RTT ratio | ack avg ms | ack p95 ms | max burst frames | pacing interval ms | pacing delay ms | burst prevented | effective cap avg | cap reduced | blocked by cap | window frames |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| direct | 0 | - | - | - | - | - | - | - | - | - | 1250.00 | - | - | - | - | - | - | - | - | - | - | - | - | - | - | - | - | - |
| remote-1hop | 1 | 2162.29 | 364.00 | 0.80 | 0.00 | 0.00 | 1105.43 | 1.20 | 0.00 | 906.20 | 1107.18 | 0.0000 | - | - | 0 | 0 | 0 | - | 181.00 | 184.40 | 4 | 3.00 | 747.80 | 282 | 64 | 0 | 0 | 64 |
| remote-2hop | 2 | 2219.73 | 367.00 | 0.40 | 0.00 | 0.00 | 1165.78 | 94.80 | 0.00 | 1057.20 | 1167.11 | 0.0441 | 273.00 | 273.00 | 13 | 0 | 0 | 1.50 | 179.40 | 189.60 | 4 | 3.00 | 729.20 | 271 | 64 | 0 | 0 | 64 |
| remote-3hop | 3 | 2262.29 | 484.00 | 1.20 | 0.00 | 0.00 | 1468.53 | 103.00 | 0.00 | 1269.40 | 1470.85 | 0.0401 | 369.00 | 369.00 | 12 | 0 | 0 | 1.50 | 230.80 | 248.40 | 4 | 4.00 | 969.20 | 276 | 64 | 0 | 0 | 64 |
| remote-5hop | 5 | 2808.08 | 661.00 | 1.00 | 0.00 | 0.00 | 1895.15 | 458.40 | 0.00 | 1601.80 | 1897.28 | 0.2810 | 231.00 | 242.00 | 81 | 0 | 6 | 1.63 | 271.20 | 293.80 | 4 | 4.00 | 952.20 | 213 | 64 | 0 | 0 | 64 |

## Bottleneck Ranking

1. Warm route ready (one-time): `2363.10 ms`
2. Overlay pre-first-byte gap: `1408.72 ms`
3. Overlay first-send -> client first-byte gap: `1408.72 ms`
4. Exit/body delivery tail: `1208.65 ms`
5. Handshake/session setup (one-time): `469.00 ms`
6. Window/backpressure stall: `164.35 ms`
7. Route selection overhead: `1.25 ms`
8. Client body completion tail: `1.03 ms`
9. Target connect: `0.85 ms`
10. Client request buffering: `0.00 ms`
11. Target first-byte wait: `0.00 ms`
12. Exit first overlay send delay: `0.00 ms`

## Correlation

- `total_time_ms` vs `retransmit_rate`: `0.367`
- `total_time_ms` vs `ack_latency_ms_avg`: `0.567`

## Findings

- The best overlay path in this sample is `remote-1hop` with avg total `1107.18 ms`.
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
