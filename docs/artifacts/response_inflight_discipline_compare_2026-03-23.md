# RTT-Aware Inflight Discipline Compare

## Inputs

- before: `docs/artifacts/stage5_pacing_remote_perf_stage_2026-03-22.md`
- before: `docs/artifacts/stage5_pacing_remote_perf_matrix_2026-03-22.md`
- after: `docs/artifacts/stage5_inflight_remote_perf_stage_final_2026-03-23.md`
- after: `docs/artifacts/stage5_inflight_remote_perf_matrix_final_2026-03-23.md`
- targeted regression: `docs/artifacts/stage5_inflight_targeted_regression_2026-03-23.log`

Hard response window remained `64` in both runs.

## Stage Breakdown Compare

| route | burst avg before | burst avg after | avg inflight before | avg inflight after | max inflight avg before | max inflight avg after | ACK avg before | ACK avg after | ACK p95 before | ACK p95 after | hard stall before ms | hard stall after ms | effective cap avg after | blocked by cap after | stage total before ms | stage total after ms |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1-hop | 4.0 | 4.0 | 55.53 | 53.65 | 61.80 | 63.20 | 182.8 | 216.2 | 190.0 | 340.2 | 32.4 | 199.4 | 63.4 | 2 | 1113.37 | 1300.41 |
| 2-hop | 4.0 | 4.0 | 55.42 | 54.96 | 61.00 | 62.00 | 183.4 | 193.2 | 200.4 | 263.6 | 44.0 | 105.8 | 63.8 | 1 | 1133.96 | 1201.48 |
| 3-hop | 4.0 | 4.0 | 54.13 | 53.08 | 62.60 | 60.40 | 251.2 | 239.4 | 288.2 | 269.8 | 155.0 | 24.2 | 61.2 | 189 | 1517.50 | 1461.79 |

## End-to-End Perf Compare

| route | TTFB before ms | TTFB after ms | delta ms | delta % | total before ms | total after ms | delta ms | delta % | throughput before Bps | throughput after Bps |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1-hop | 1150.94 | 1114.95 | -35.99 | -3.13% | 1151.68 | 1116.22 | -35.46 | -3.08% | 227750 | 234930 |
| 2-hop | 1182.46 | 1810.92 | 628.46 | 53.15% | 1183.38 | 1811.82 | 628.44 | 53.11% | 222295 | 184787 |
| 3-hop | 1608.21 | 1483.18 | -125.03 | -7.77% | 1609.04 | 1487.11 | -121.93 | -7.58% | 169481 | 178457 |

## Targeted Regression

`cargo test --test chaos_matrix response_inflight_discipline_reduces_ack_tail_after_pacing -- --test-threads=1`

Evidence:

- `docs/artifacts/stage5_inflight_targeted_regression_2026-03-23.log`

The regression stayed green and verifies:

- effective cap engages below `64`
- effective-cap blocking is non-zero
- ACK p95 is lower in the controlled A/B local pressure profile
- hard-window wait is lower in the controlled A/B local pressure profile
- inflight occupancy is lower in the controlled A/B local pressure profile
- total time stays within the test's no-regression guardrail

## Interpretation

This step produced a route-dependent result.

- `3-hop` is the clearest positive case:
  - effective cap engaged materially
  - hard-window stall dropped from `155.0 ms` to `24.2 ms`
  - ACK p95 dropped from `288.2 ms` to `269.8 ms`
  - stage total dropped from `1517.50 ms` to `1461.79 ms`
  - end-to-end total dropped from `1609.04 ms` to `1487.11 ms`
- `1-hop` and `2-hop` did not get the same benefit in this WAN sample:
  - the controller barely engaged there
  - `effective cap avg` stayed near the hard ceiling
  - end-to-end `2-hop` included a large outlier (`max total 3915.53 ms`)

So the fix is real, but it is not a full congestion controller. It is a first pressure-response layer that helps most when the route actually needs a lower working inflight ceiling.
