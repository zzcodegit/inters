# Overlay stage breakdown RCA

## Inputs

- local raw: `docs/artifacts/recovery_coupled_control_v2_probe_1hop.local.jsonl`
- client stage trace: `docs/artifacts/recovery_coupled_control_v2_probe_1hop.client.jsonl`
- exit stage trace: `docs/artifacts/recovery_coupled_control_v2_probe_1hop.exit.jsonl`
- direct forward stage trace: `docs/artifacts/recovery_coupled_control_v2_probe_1hop.direct.jsonl`
- joined per-run view: `docs/artifacts/recovery_coupled_control_v2_probe_1hop.joined.jsonl`

## Scenario Averages

| scenario | route | route ready ms | handshake ms | target connect ms | first target byte wait ms | exit first send delay ms | overlay first-send -> client ms | hard window stall ms | effective cap wait ms | exit/body tail ms | client total ms | retransmit rate | retx timeout avg ms | retx timeout p95 ms | retx triggers | retx early | retx late | retx/RTT ratio | ack avg ms | ack p95 ms | max burst frames | pacing interval ms | pacing delay ms | burst prevented | effective cap avg | cap reduced | blocked by cap | congestion events | congestion duration ms | congestion cap limit | congestion pacing extra ms | window frames |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | 1 | 4801.63 | 1559.00 | 1.00 | 0.00 | 0.00 | 1349.28 | 8.00 | 0.00 | 905.00 | 1351.36 | 0.0000 | - | - | 0 | 0 | 0 | - | 184.00 | 187.00 | 4 | 3.00 | 735.00 | 280 | 64 | 0 | 0 | 0 | 0.00 | 64 | 0.00 | 64 |

## Bottleneck Ranking

1. Warm route ready (one-time): `4801.63 ms`
2. Handshake/session setup (one-time): `1559.00 ms`
3. Overlay pre-first-byte gap: `1349.28 ms`
4. Overlay first-send -> client first-byte gap: `1349.28 ms`
5. Exit/body delivery tail: `905.00 ms`
6. Window/backpressure stall: `8.00 ms`
7. Route selection overhead: `3.00 ms`
8. Client body completion tail: `1.08 ms`
9. Target connect: `1.00 ms`
10. Client request buffering: `0.00 ms`
11. Target first-byte wait: `0.00 ms`
12. Exit first overlay send delay: `0.00 ms`

## Correlation

- `total_time_ms` vs `retransmit_rate`: `-`
- `total_time_ms` vs `ack_latency_ms_avg`: `-`

## Findings

- The best overlay path in this sample is `remote-1hop` with avg total `1351.36 ms`.
- `target_connect_ms` and target first-byte wait stay small relative to overlay totals, so the public target itself is not the main bottleneck.
- The biggest per-request cost sits after the target is already reachable: overlay first-send -> client first-byte gap, window/backpressure stall, and the remaining exit/body delivery tail dominate the remote paths.
- `max_send_burst_frames`, `pacing_interval_ms_avg`, `pacing_delay_applied_ms_total`, and `burst_prevented_count` expose whether the exit is still dumping response frames in bursts or spreading them across the ACK window.
- `effective_inflight_cap_avg`, `inflight_cap_reduced_count`, `send_blocked_by_effective_cap`, and `effective_cap_wait_total_ms` expose whether the exit still drives the hard window directly or whether RTT/ACK pressure is actively shaping in-flight occupancy.
- `congestion_events`, `congestion_duration_ms`, `congestion_cap_limit`, and `congestion_pacing_extra_ms` expose whether the lightweight congestion response actually entered a non-default state on the slower paths.
- One-time costs (`route_ready_ms`, `handshake_ms`) matter for cold start, but they are not the reason the steady-state per-request `TTFB` stays in the multi-second range.

## Optimization Recommendation

1. Prioritize retransmit/ACK tuning on the response path first; that is where the largest steady-state time is being burned.
2. Use better exit/route scoring second; the current sample shows that path quality matters more than hop-count alone, so scoring should prefer the empirically faster overlay path instead of assuming more or fewer hops are always better.
3. Treat warm tunnel/session reuse as a cold-start improvement, not the main fix for current per-request TTFB.
4. Treat target-connect optimization as low priority here because target connect is already small compared with the overlay delivery tail.
