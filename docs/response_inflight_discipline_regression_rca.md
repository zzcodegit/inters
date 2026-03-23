# Response Inflight Discipline Regression RCA

## Rejected Baseline

Rejected baseline: `b14e1eaabf2e9e4fd2203b990117b94a53fc5c6a`

The rejected Stage 5 baseline did not fail because the hard window `64` was too small. It failed because the first RTT-aware inflight policy reacted to a single transient pressure sample as if it were sustained pressure.

## Symptom

- `2-hop` average total time regressed from `1183.38 ms` on the accepted pacing baseline to `1811.82 ms` on the rejected inflight baseline.
- In the rejected exit trace, `effective_inflight_cap_reduced` fired on all live remote lengths:
  - route `1`: `2` reductions
  - route `2`: `2` reductions
  - route `3`: `4` reductions
- In the same trace, `send_blocked_by_effective_cap` appeared on all live routes:
  - route `1`: `2`
  - route `2`: `3`
  - route `3`: `189`

These counts were extracted from [stage5_inflight_remote_perf_stage_final_2026-03-23.exit.jsonl](../docs/artifacts/stage5_inflight_remote_perf_stage_final_2026-03-23.exit.jsonl).

## One Root Cause

One transient hard-window stall spike could reduce the effective cap immediately, even on a route that had not shown sustained pressure yet.

That behavior was too coarse for moderate WAN paths:

- the trigger was too early
- the reduction happened on a single noisy sample
- restore was too sticky after the drop

So the controller throttled `1-hop` and `2-hop` before there was enough evidence that the route really needed protection.

## Concrete Rejected-Run Examples

From the rejected remote exit trace:

- `1-hop run2`: cap dropped after `recent_window_wait_ms=370`, `ack_latency_ms_avg=237`, `ack_latency_ms_p95=541`
- `2-hop run2`: cap dropped after `recent_window_wait_ms=410`, `ack_latency_ms_avg=189`, `ack_latency_ms_p95=202`

That second case is the clearest proof of over-trigger: a single stall spike with only moderate ACK values was enough to cut the cap.

## Corrective Direction

The corrective pass keeps the hard window at `64` but changes the policy from "single noisy sample can reduce" to "only sustained near-window pressure with slow ACK can reduce".

That is the only structural change in this pass. It does not add full congestion control, does not resize the hard window, and does not touch the Stage 4 transport fixes.
