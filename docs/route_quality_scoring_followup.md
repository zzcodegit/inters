# Route-quality scoring follow-up

## What changed in this step

Route-quality scoring was hardened against three concrete failure modes:

1. early lucky or unlucky samples
2. noisy latency spikes
3. route flapping after a recent switch

The scoring model is still the same family of score:

`final_score = base * transport_agg * quality_agg * hop_factor * pheromone_factor`

but the quality and selection layers are now more stable over time:

- temporal smoothing now keeps both EMA mean and EMA deviation
- warmup confidence keeps cold routes closer to neutral
- instability is penalized explicitly
- recent switch history adds cooldown and stronger switch margins
- routes that flap get a route-level penalty

Exact policy details are documented in `docs/route_quality_scoring.md`.

## Controlled proof

The controlled artifact now proves four behaviors in one report:

- `A`: healthy route A wins normally
- `W1`: one lucky cold-start sample on route B does not steal selection
- `N`: a noisy route is penalized even when its average latency looks competitive
- `C`: once route A is genuinely degraded, route B still takes over

Artifacts:

- log: `docs/artifacts/route_quality_controlled_stability_2026-03-21.log`
- raw phases: `docs/artifacts/route_quality_controlled_stability_2026-03-21.jsonl`
- report: `docs/artifacts/route_quality_controlled_stability_2026-03-21.md`

Key numbers from the controlled report:

- cold-start lucky challenger:
  - warmed A score `0.6493`
  - cold B score `0.6071`
  - B has much better raw timing, but only `q conf=0.250`, `warmup=0.089`
- noisy route penalty:
  - stable A instability `0 ppm`
  - noisy B instability `1133065 ppm`
  - noisy B loses despite competitive average latency
- real switch:
  - degraded A score `0.2146`
  - healthy B score `0.6137`
  - selector switches with `switch_margin_exceeded`
- anti-flap:
  - H1 best route is B, but selector stays on A with `hold_time_active`
  - H2 best route is still B, but selector still stays on A with `within_hysteresis_margin`

## WAN evidence

### Single adaptive WAN run

Artifacts:

- log: `docs/artifacts/route_quality_remote_2026-03-21_stability.log`
- raw measurements: `docs/artifacts/route_quality_remote_2026-03-21_stability.jsonl`
- client stage trace: `docs/artifacts/route_quality_remote_2026-03-21_stability.client.jsonl`
- report: `docs/artifacts/route_quality_remote_2026-03-21_stability.md`

Observed topology:

- relay-1: `45.197.133.115:30001`
- relay-2: `185.144.28.95:30002`
- exit: `31.192.232.26:30000`

Measured result:

- 6/6 successful WAN requests
- average total `1749.98 ms`
- average TTFB `1748.72 ms`
- average throughput `158738.16 Bps`
- non-shortest selections observed: `7`

The stage trace now shows confidence and instability directly in the selection record, not only the final score.

### WAN series before vs after

Before and after were compared on the same public topology.

Artifacts:

- recomputed baseline series:
  - `docs/artifacts/route_quality_series_2026-03-21_recomputed.jsonl`
  - `docs/artifacts/route_quality_series_2026-03-21_recomputed.md`
- hardened series:
  - `docs/artifacts/route_quality_series_2026-03-21_stability.jsonl`
  - `docs/artifacts/route_quality_series_2026-03-21_stability.md`
  - `docs/artifacts/route_quality_series_2026-03-21_stability.log`
- compare:
  - `docs/artifacts/route_quality_stability_compare_2026-03-21.md`

Important result:

- hard switches were already `0` before, so there was no honest way to claim "less than zero"
- the meaningful WAN improvement is reduced jitter, not reduced hard-switch count

Measured stability deltas:

- avg selected-score stdev:
  - before `0.0480`
  - after `0.0412`
- avg total-time CV:
  - before `33.62%`
  - after `19.97%`
- avg TTFB CV:
  - before `33.64%`
  - after `19.98%`

So on live WAN samples the selector stayed on the same best route as before, but the score and latency series became materially less noisy.

## Regression status

Local:

- readiness log: `docs/artifacts/route_quality_stability_http_probe_ready_2026-03-21.log`
- local baseline log: `docs/artifacts/route_quality_stability_local_baseline_2026-03-21.log`

Remote:

- WAN matrix log: `docs/artifacts/remote_wan_matrix_2026-03-21_route_quality_stability.log`
- perf matrix log: `docs/artifacts/remote_perf_matrix_2026-03-21_route_quality_stability.log`
- perf matrix report: `docs/artifacts/remote_perf_matrix_2026-03-21_route_quality_stability.md`
- stage log: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_route_quality_stability.log`
- stage report: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_route_quality_stability.md`

## Honest limitation

This step makes route selection more stable and more confidence-aware.

It does not guarantee that the public WAN sample will suddenly start switching routes more often. In this topology the same 3-hop path still stayed on top through the observed windows. The proof here is:

- the selector is less sensitive to early noise
- noisy routes are penalized structurally
- cooldown and hysteresis prevent pointless churn
- live WAN score jitter and latency variance are lower than before

That is a stability improvement, not a claim that every live sample must produce visible route switching.
