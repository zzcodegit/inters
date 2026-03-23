# Response Inflight Corrective Compare

## States

- pacing baseline: [stage5_pacing_remote_perf_matrix_2026-03-22.md](./stage5_pacing_remote_perf_matrix_2026-03-22.md)
- rejected inflight baseline: [stage5_inflight_remote_perf_matrix_final_2026-03-23.md](./stage5_inflight_remote_perf_matrix_final_2026-03-23.md)
- corrected final run: [stage5_inflight_corrected_final_remote_perf_matrix_2026-03-23.md](./stage5_inflight_corrected_final_remote_perf_matrix_2026-03-23.md)

## End-to-End Perf

| route | pacing total ms | rejected total ms | corrected total ms | corrected vs pacing | corrected vs rejected |
| --- | --- | --- | --- | --- | --- |
| 1-hop | 1151.68 | 1116.22 | 1555.92 | `+404.24 ms` / `+35.10%` | `+439.70 ms` / `+39.39%` |
| 2-hop | 1183.38 | 1811.82 | 1212.74 | `+29.36 ms` / `+2.48%` | `-599.08 ms` / `-33.07%` |
| 3-hop | 1609.04 | 1487.11 | 1407.29 | `-201.75 ms` / `-12.54%` | `-79.82 ms` / `-5.37%` |

## Stage Metrics

| route | pacing client total ms | rejected client total ms | corrected client total ms | pacing ack p95 | rejected ack p95 | corrected ack p95 | pacing stall ms | rejected stall ms | corrected stall ms | corrected cap avg | corrected blocked by cap |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| 1-hop | 1113.37 | 1300.41 | 1708.77 | 190.0 | 340.2 | 397.6 | 32.4 | 199.4 | 201.6 | 64 | 0 |
| 2-hop | 1133.96 | 1201.48 | 1185.07 | 200.4 | 263.6 | 203.6 | 44.0 | 105.8 | 54.8 | 64 | 0 |
| 3-hop | 1517.50 | 1461.79 | 1422.69 | 288.2 | 269.8 | 251.8 | 155.0 | 24.2 | 41.4 | 64 | 0 |

Stage sources:

- pacing: [stage5_pacing_remote_perf_stage_2026-03-22.md](./stage5_pacing_remote_perf_stage_2026-03-22.md)
- rejected: [stage5_inflight_remote_perf_stage_final_2026-03-23.md](./stage5_inflight_remote_perf_stage_final_2026-03-23.md)
- corrected: [stage5_inflight_corrected_final_remote_perf_stage_2026-03-23.md](./stage5_inflight_corrected_final_remote_perf_stage_2026-03-23.md)

## Cap Decision Proof

Rejected exit trace:

- route `1`: `2` cap reductions, `2` blocked-by-cap events
- route `2`: `2` cap reductions, `3` blocked-by-cap events
- route `3`: `4` cap reductions, `189` blocked-by-cap events

Corrected final exit trace:

- no `effective_inflight_cap_reduced` events
- no `send_blocked_by_effective_cap` events
- `effective_inflight_cap_avg=64` on `1-hop/2-hop/3-hop`

## Targeted Regression Proof

Local targeted regression:

- test: `response_inflight_discipline_avoids_transient_overtrigger_on_moderate_route`
- log: [stage5_inflight_corrected_targeted_regression_2026-03-23.log](./stage5_inflight_corrected_targeted_regression_2026-03-23.log)

The regression asserts:

- average effective cap stays at `64`
- cap reductions stay at `0`
- blocked-by-cap stays at `0`
- effective-cap wait stays at `0`

Policy unit proof:

- log: [stage5_inflight_corrected_final_policy_unit_2026-03-23.log](./stage5_inflight_corrected_final_policy_unit_2026-03-23.log)
- `response_inflight_state_requires_sustained_pressure_before_reducing`
- `response_inflight_state_restores_after_shorter_clean_streak`

Those tests prove the controller is not disabled. It still reduces on sustained pressure and restores after a short clean streak.

## Honest Readout

- The rejected `2-hop` regression is gone.
- The rejected control-induced cap drops on `1-hop/2-hop` are gone.
- `3-hop` remains better than the pacing baseline in the corrected final WAN sample.
- `1-hop` does not show control-layer throttling anymore, but it is still vulnerable to WAN retransmit noise in the corrected final sample.

That last point is visible in the corrected final stage trace:

- `1-hop`: `effective cap avg=64`, `cap reduced=0`, `blocked by cap=0`
- `1-hop`: `retransmit_rate=0.0909`, `ack p95=397.6`, `hard stall=201.6`

So the remaining `1-hop` regression in the final clean sample is not caused by the corrected effective-cap policy.
