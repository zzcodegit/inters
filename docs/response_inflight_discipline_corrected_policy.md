# Corrected RTT-Aware Inflight Policy

## Scope

This policy is a corrective pass on top of baseline pacing. It is not full congestion control.

- hard response window stays `64`
- pacing stays enabled
- routing stays unchanged
- only the effective inflight cap decision logic changes

## Policy

The exit now uses:

- hard cap: `64`
- effective cap: `<= 64`
- moderate reduction target: `60`
- severe reduction target: `56`

Reduction is allowed only when all of these are true:

- the route is already running near the hard window
- recent hard-window wait is non-trivial
- ACK latency is also slow relative to bootstrap RTT
- pressure is sustained long enough

Sustained-pressure requirement:

- moderate pressure needs `2` consecutive pressure samples
- severe pressure needs `2` consecutive pressure samples

Restore policy:

- no reduction can happen on a single transient sample
- restore starts only after a clean streak
- clean streak threshold is `3`
- restore step is `+4`

## Why This Is More Selective

The corrected policy distinguishes between:

- a moderate route with one noisy stall sample
- a route that is persistently slow and still running close to the hard window

That means:

- normal `1-hop/2-hop` routes should stay at effective cap `64` unless pressure is real
- a genuinely pressured route can still reduce below `64`
- cap restore is faster once the route becomes clean again

## Proof Sources

- targeted regression log: [stage5_inflight_corrected_targeted_regression_2026-03-23.log](../docs/artifacts/stage5_inflight_corrected_targeted_regression_2026-03-23.log)
- unit policy log: [stage5_inflight_corrected_final_policy_unit_2026-03-23.log](../docs/artifacts/stage5_inflight_corrected_final_policy_unit_2026-03-23.log)
- corrected remote perf stage trace: [stage5_inflight_corrected_final_remote_perf_stage_2026-03-23.exit.jsonl](../docs/artifacts/stage5_inflight_corrected_final_remote_perf_stage_2026-03-23.exit.jsonl)
