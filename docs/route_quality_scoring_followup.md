# Route-quality scoring follow-up

## What was verified

- local baseline stayed green
- readiness semantics stayed green
- accepted remote WAN matrix stayed green
- accepted remote perf runner stayed green
- accepted remote stage runner stayed green
- adaptive runner `scripts/run_route_quality_remote.sh` passed on the real WAN topology
- new series runner `scripts/run_route_quality_remote_series.sh` passed across multiple independent WAN runs
- new controlled policy test proved causality and anti-flap behavior without relying on live WAN luck

Topology used for live WAN runs:

- `relay-1`: `45.197.133.115:30001`
- `relay-2`: `185.144.28.95:30002`
- `exit`: `31.192.232.26:30000`

## Key evidence

Controlled causality proof:

- log: `docs/artifacts/route_quality_controlled_2026-03-21.log`
- raw phase records: `docs/artifacts/route_quality_controlled_2026-03-21.jsonl`
- report: `docs/artifacts/route_quality_controlled_2026-03-21.md`

Single adaptive WAN run:

- log: `docs/artifacts/route_quality_remote_2026-03-21.log`
- raw measurements: `docs/artifacts/route_quality_remote_2026-03-21.jsonl`
- client stage trace: `docs/artifacts/route_quality_remote_2026-03-21.client.jsonl`
- report: `docs/artifacts/route_quality_remote_2026-03-21.md`

WAN series:

- log: `docs/artifacts/route_quality_series_2026-03-21.log`
- summary jsonl: `docs/artifacts/route_quality_series_2026-03-21.jsonl`
- summary report: `docs/artifacts/route_quality_series_2026-03-21.md`

Regression checks after scoring/hysteresis changes:

- local readiness log: `docs/artifacts/route_quality_http_probe_ready_2026-03-21.log`
- local baseline log: `docs/artifacts/route_quality_local_baseline_2026-03-21.log`
- matrix log: `docs/artifacts/remote_wan_matrix_2026-03-21_route_quality_policy.log`
- perf log: `docs/artifacts/remote_perf_matrix_2026-03-21_route_quality_policy.log`
- perf report: `docs/artifacts/remote_perf_matrix_2026-03-21_route_quality_policy.md`
- stage log: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_route_quality_policy.log`
- stage report: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_route_quality_policy.md`

## What the controlled proof established

From `docs/artifacts/route_quality_controlled_2026-03-21.md`:

- healthy `A` = 1-hop starts as the selected and best-scored route
- after explicit degradation, `A` drops from `0.6435` to `0.1709`
- healthy `B` = 2-hop rises to `0.6240`
- selector switches from `A` to `B` with reason `switch_margin_exceeded`
- the switch gap is not ambiguous:
  - absolute delta `0.4532`
  - relative delta `265.24%`

That is the required causality chain:

- A: route `A` best -> selected
- B: `A` degraded with ACK/retransmit/stall + failures
- C: `A` score falls below `B`
- D: selector switches to `B`

The same report also proves anti-flap:

- in `H1`, `B` is the best-score route
- the selector still keeps `A` because hold time is active
- score delta there is only:
  - absolute `0.0181`
  - relative `2.94%`
- in `H2`, after hold time expires, the selector still keeps `A`
- reason becomes `within_hysteresis_margin`

This is the required "bad 1-hop vs good 2-hop" case, and it is proven without any hidden localhost fallback.

## What the live WAN run established

From `docs/artifacts/route_quality_remote_2026-03-21.md`:

- multi-candidate score events: `7`
- selections with quality metrics attached: `6`
- feedback events received from exit: `27`
- non-shortest selections observed: `7`

Representative snapshot:

- 3-hop final score: `0.6720`
- 1-hop final score: `0.4200`
- 2-hop final score: `0.4116`

Later in the same run the selected 3-hop path kept improving:

- selected score reaches `0.7462`
- `decision_reason` becomes `current_still_best`
- attached route-quality metrics include:
  - `recent_ttfb_ms=996`
  - `recent_total_ms=2170`
  - `recent_ack_p95_ms=294`
  - `recent_retransmit_rate_ppm=2109`
  - `recent_window_wait_ratio_ppm=831124`

So the live selector is now visibly quality-driven and auditable. It is no longer "shortest path unless broken".

## What the WAN series established

From `docs/artifacts/route_quality_series_2026-03-21.md`:

- independent WAN series runs: `3`
- total selections observed: `15`
- selected route matched the current best-score route: `15 / 15`
- best-score pick ratio: `100%`
- total switches inside those runs: `0`

Selection distribution in that sample:

- 3-hop route `45.197.133.115 -> 185.144.28.95 -> 31.192.232.26`: `15`

Reason distribution:

- `initial_selection`: `3`
- `current_still_best`: `12`

This is important because it separates two claims:

- route choice is caused by the score, not by blind shortest-path bias
- once a route is clearly winning, the selector stays stable instead of churning

## Remaining limitation

The current scoring policy is quality-aware and now has explicit hysteresis, but it is still sample-driven.

That means:

- yes, route choice is now visibly quality-driven and auditable
- yes, a healthy 2-hop route can beat a degraded 1-hop route
- yes, small score differences no longer cause route flapping
- no, the current policy should not yet be presented as globally optimal under every traffic window

The live WAN series on March 21, 2026 happened to keep the 3-hop route on top for all observed selections. That is good evidence of stability, but it is not a proof that 3-hop is always globally fastest. It is proof that the system now chooses and sticks to the route that its quality model currently scores highest.
