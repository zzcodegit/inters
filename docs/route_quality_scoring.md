# Route-quality scoring

## Goal

The client should stop treating "reachable" and "shortest" as the same thing.
Route choice now blends route availability with recent quality signals gathered
from real response-path traffic.

## Inputs used by the score

Each candidate route keeps three classes of signal:

1. Route baseline
   - recent route success rate
   - recent route RTT
   - recent route failure count

2. Per-hop transport reliability
   - protocol/port success and failure history
   - recent transport failures with cooldown
   - pheromone reinforcement already present in the existing routing layer

3. Response-path quality
   - client-side `recent_ttfb_ms`
   - client-side `recent_total_ms`
   - recent success/failure count for the route
   - exit-side `ack_latency_ms_p95`
   - exit-side `retransmit_rate_ppm`
   - exit-side `window_wait_ratio_ppm`
   - hop count as a mild secondary factor only

## How the data is collected

### Client-side observations

Every buffered HTTP round-trip records:

- status code
- `first_client_byte_ms`
- total stream duration
- response size

Those values are attached to the route that was actually used for the stream.

### Exit-side feedback

When a buffered HTTP stream closes, the exit serializes a
`ResponseQualityFeedback` payload into `CloseStream`.
That payload contains:

- route length
- response bytes / frames sent
- stream duration
- first target byte and first overlay send timestamps
- ACK latency summary
- retransmit rate
- window stall totals
- HTTP status

The client consumes that feedback in the normal receive loop and updates the
matching route by hop-list, so route selection can use real response-path
signals instead of only client-local RTT.

## Selection policy

At selection time the client scores every non-excluded candidate:

`final_score = base * transport_agg * quality_agg * hop_factor * pheromone_factor`

Where the components are:

- `base`
  - route success rate + inverse RTT
  - penalized by route failure count
- `transport_agg`
  - per-hop protocol/port reliability
  - decayed over time
  - recent transport failures carry a short cooldown penalty
- `quality_agg`
  - geometric blend of recent total time, TTFB, ACK p95, retransmit rate,
    stall ratio, and recent route success/failure
  - confidence-gated, so one sample does not dominate immediately
- `hop_factor`
  - mild `-2%` per extra hop
  - explicitly secondary to quality
- `pheromone_factor`
  - existing routing reinforcement layer

## Exact normalization and smoothing

### Base score

For one route candidate:

- `rtt_part = 1 / rtt_ms`
- `base_raw = success_rate * 0.7 + rtt_part * 0.3`
- `base = base_raw * 0.80^failure_count`

`success_rate` is itself smoothed over time by the existing route success/failure updates.

### EMA

All local and exit-side timing counters use the same EMA:

- previous weight: `0.7`
- new sample weight: `0.3`

That is:

`ema_next = 0.7 * ema_prev + 0.3 * sample`

### Quality factors

Each latency-like metric is turned into a multiplicative factor where lower is better.
If the metric is better than the `good` threshold, it gets the `best_factor`.
If it is worse than the `bad` threshold, it gets the `worst_factor`.
Between those points it is linearly interpolated.

Current ranges:

- total time:
  - `good=700 ms`
  - `bad=6000 ms`
  - factor range `1.12 .. 0.70`
- TTFB:
  - `good=250 ms`
  - `bad=4000 ms`
  - factor range `1.10 .. 0.68`
- ACK p95:
  - `good=180 ms`
  - `bad=1200 ms`
  - factor range `1.08 .. 0.68`
- retransmit rate:
  - `good=0 ppm`
  - `bad=120000 ppm`
  - factor range `1.06 .. 0.65`
- window stall ratio:
  - `good=20000 ppm`
  - `bad=500000 ppm`
  - factor range `1.05 .. 0.68`

Route success/failure history contributes:

- `success_rel = (success_count + 1) / (success_count + failure_count + 2)`
- `success_factor = 0.85 + success_rel * 0.30`

Those factors are blended with a geometric mean:

`measured_quality = geometric_mean(total_factor, ttfb_factor, success_factor, ack_factor?, retransmit_factor?, stall_factor?)`

Optional factors are included only when data exists.

### Confidence model

Quality is confidence-gated, so one fresh sample does not dominate:

- `confidence = min((local_samples + feedback_samples) / 8, 1.0)`
- `quality_agg = clamp((1 - confidence) + confidence * measured_quality, 0.75, 1.25)`

### Aging and recency penalties

Two recency controls apply on top of `quality_agg`:

- recent-failure penalty:
  - if the newest failure is newer than the newest success and happened within `20s`
  - multiply by `0.88`
- age decay:
  - exponential half-life `5 minutes`
  - `age_weight = exp(-ln(2) * age / half_life)`
  - clamped to `0.05 .. 1.0`

Final quality factor after aging is clamped to `0.60 .. 1.25`.

### Transport factor

Transport reliability is scored per hop:

- success/failure counts are age-weighted with the same `5 minute` half-life
- transport confidence ramps to `1.0` over `20` effective samples
- recent failure cooldown window: `5s`
- recent uncured failure penalty: `0.80`
- reliability term:
  - `reliability = (succ + 1) / (succ + fail * fail_recovery + 2)`
  - `rel_adj = 0.85 + reliability * 0.30`

Per-hop transport factors are combined as:

`transport_agg = clamp(min_hop_effect * sqrt(product_hop_effect), 0.7, 1.3) * staleness_factor`

The staleness factor stays `1.0` for the first `30s` after use, then decays down toward `0.7`.

### Hop factor

Hop count is explicitly secondary:

- `hop_factor = clamp(1 - 0.02 * (hop_count - 1), 0.92, 1.0)`

That means one extra hop is only a `2%` penalty unless the quality signals say otherwise.

## Tie-break and switch policy

There are three separate decision bands:

1. Tie band
   - absolute score delta `< 0.015`
   - relative score delta `< 3%`
   - when no route is currently selected, the shorter route wins the tie
2. Normal switch band
   - challenger must beat the current route by:
     - absolute delta `>= 0.040`
     - and relative delta `>= 8%`
3. Emergency switch during hold time
   - while hold time is active, the challenger must beat the current route by:
     - absolute delta `>= 0.100`
     - or relative delta `>= 20%`

Hold time:

- after a selection, the current route is sticky for `15s`
- during that window a merely somewhat-better challenger is recorded in the scoreboard, but not selected

Decision reasons emitted into stage trace:

- `initial_selection`
- `current_still_best`
- `hold_time_active`
- `within_hysteresis_margin`
- `switch_margin_exceeded`

Tie-break reason emitted when relevant:

- `shorter_route_tie_break`

## Why this is safer than shortest-path fallback

- A 2-hop or 3-hop path can now beat a 1-hop path if its observed quality is better.
- Recent failures degrade score even when a route is technically still reachable.
- Route scoring can react to response-path bottlenecks that RTT-only probing cannot see.
- The candidate scoreboard is emitted into the client stage trace, so selection is auditable.

## Evidence

Controlled causality and anti-flap proof:

- `docs/artifacts/route_quality_controlled_2026-03-21.md`
- `docs/artifacts/route_quality_controlled_2026-03-21.jsonl`

Live WAN adaptive run:

- `docs/artifacts/route_quality_remote_2026-03-21.md`
- `docs/artifacts/route_quality_remote_2026-03-21.client.jsonl`

Live WAN series:

- `docs/artifacts/route_quality_series_2026-03-21.md`
- `docs/artifacts/route_quality_series_2026-03-21.jsonl`

## Known limitation

The score is now quality-aware, but it is still sample-driven. A short burst of
bad early outcomes can down-rank a route aggressively until fresh successful
samples arrive. The follow-up document records the current remote WAN behavior
and that remaining sensitivity.
