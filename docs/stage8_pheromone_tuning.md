## Stage 8 — Pheromone tuning notes

This document summarizes how local pheromone memory works today and what to watch operationally.

---

## Core model

Pheromones are **local, bounded scores** attached to `(dest_hint, protocol, port)` keys:

- `PheromoneKey { dest_hint, protocol, port }`
- `PheromoneEntry { score ∈ [0,1], confidence ∈ [0,1], last_update_ms }`
- stored in `PheromoneStore` with a `max_entries` cap and simple "oldest first" eviction.

They are:

- updated on:
  - route-level success/failure,
  - ant-based measurements (Echo ants),
- decayed over time,
- read in routing as a **bounded multiplicative factor** on top of the base route score.

---

## Reinforcement parameters (current)

In `PheromoneStore::reinforce_transport`:

- success:
  - `alpha = 0.15`
  - success value based on RTT:
    - `≤ 50ms` → `1.0`
    - `≤ 150ms` → `0.8`
    - `≤ 500ms` → `0.6`
    - else → `0.5`
  - `score = score * (1 - alpha) + sv * alpha`
  - `confidence += 0.05` (clamped to `1.0`)
- failure:
  - `score *= 0.85`
  - `confidence *= 0.7`

This keeps reinforcement:

- gradual (no instant jumps to 1.0),
- symmetric enough that repeated failures really degrade a key,
- with **confidence** reflecting "how sure we are" (successes increase, failures dampen).

---

## Decay / evaporation

In `PheromoneStore::decay`:

- decay factor: `0.98` per minute,
- entries with `score < 0.05` are dropped.

Operationally:

- strong winners fade if not updated (old bias washes out),
- weak/noisy entries eventually disappear.

---

## Integration into routing

In `RouteStore`:

- for each route:
  - base score uses success rate + RTT + transport stats,
  - pheromone read as `(score, confidence)` for the last hop,
  - integration factor:

    ```text
    pher_factor = (1.0 + 0.3 * pher_score)
    clamped to [0.8, 1.3]
    ```

- final route score:

  ```text
  final_score = base_score * transport_factor * pher_factor
  ```

So pheromones:

- can't fully override base metrics,
- give at most ~30% bump (or ~20% penalty) on top of other signals.

---

## Observability

Pheromone observability:

- `PheromoneStore::stats(now_ms)`:
  - `total_entries`
  - `average_score`
  - `average_confidence`
  - `oldest_entry_age_ms` / `newest_entry_age_ms`
  - `evictions_count`
  - `decay_drops_count`
  - `reinforce_success_count`
  - `reinforce_failure_count`

- `PheromoneStore::top_entries(limit, now_ms)`:
  - returns strongest entries `(key, score, confidence, age_ms)` sorted by descending score.

Debug logs (at `debug` level):

- `event=pheromone_update` with:
  - protocol, port, old/new score, old/new confidence.
- `event=pheromone_eviction` with reason:
  - `reason=cap` (bounded size eviction),
  - `reason=decay_drop` (dropped by low score).
- `event=pheromone_decay` with remaining entry count.

Routing stability (in `RouteStore`):

- `event=route_select` debug log with:
  - selected index + score,
  - `selection_count`, `switch_count`,
  - boolean `switched` if best route changed.

---

## Healthy behavior to look for

Signs that pheromones behave well:

- `total_entries` remains bounded and doesn't grow without limit.
- `average_score` sits in a reasonable mid–high range (e.g. `0.4–0.8`), not pinned at `0.0` or `1.0`.
- `average_confidence` grows only where there is repeated signal, and drops after series of failures.
- `route_select` logs show:
  - some stability (not switching every request),
  - switches when transport/RTT metrics change meaningfully.

Smoke symptom checks:

- sudden global preference for a single route with no corresponding success/RTT advantages → over-bias.
- route flapping without underlying network changes → under-bias / too aggressive decay.

---

## Known limitations / future work

- Single `dest_hint` (global bucket) — currently aggregates all destinations together.
- Last-hop key only (`protocol, port`) — no ASN/geo/traffic dims yet.
- No persistent storage: pheromone memory is in-process only.

These are intentional constraints for Stage 8 to keep behavior **simple, local, and explainable**.

