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

Where:

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

## Why this is safer than shortest-path fallback

- A 2-hop or 3-hop path can now beat a 1-hop path if its observed quality is better.
- Recent failures degrade score even when a route is technically still reachable.
- Route scoring can react to response-path bottlenecks that RTT-only probing cannot see.
- The candidate scoreboard is emitted into the client stage trace, so selection is auditable.

## Known limitation

The score is now quality-aware, but it is still sample-driven. A short burst of
bad early outcomes can down-rank a route aggressively until fresh successful
samples arrive. The follow-up document records the current remote WAN behavior
and that remaining sensitivity.
