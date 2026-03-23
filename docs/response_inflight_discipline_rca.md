# RTT-Aware Inflight Discipline RCA

## Goal

Stage 5 pacing removed the burst-send signature, but the exit still drove the full hard response window of `64` frames on every route. This step adds a simple RTT-aware inflight discipline layer under that hard cap without turning the transport into a full congestion-control system.

## Inputs

- Pacing-era stage baseline: `docs/artifacts/stage5_pacing_remote_perf_stage_2026-03-22.md`
- Pacing-era perf baseline: `docs/artifacts/stage5_pacing_remote_perf_matrix_2026-03-22.md`
- Inflight-discipline stage run: `docs/artifacts/stage5_inflight_remote_perf_stage_final_2026-03-23.md`
- Inflight-discipline perf run: `docs/artifacts/stage5_inflight_remote_perf_matrix_final_2026-03-23.md`
- Targeted regression log: `docs/artifacts/stage5_inflight_targeted_regression_2026-03-23.log`

## Bottleneck Proof After Pacing

Pacing already capped burst size at `4` frames on every remote path:

| route | pacing-era max burst avg | pacing-era avg inflight | pacing-era max inflight avg | pacing-era ACK p95 ms | pacing-era hard-window stall ms |
| --- | --- | --- | --- | --- | --- |
| 1-hop | 4.0 | 55.53 | 61.80 | 190.0 | 32.4 |
| 2-hop | 4.0 | 55.42 | 61.00 | 200.4 | 44.0 |
| 3-hop | 4.0 | 54.13 | 62.60 | 288.2 | 155.0 |

The burst signature was gone, but the exit still sat near the hard `64`-frame ceiling on all routes. The long route paid most for that fixed discipline:

- `3-hop` had nearly the same inflight occupancy as `1-hop` and `2-hop`
- but `3-hop` also had the highest ACK tail and the highest hard-window stall
- so the remaining limiter was no longer burst-send itself
- it was the lack of any RTT-aware pressure response beneath the hard window

That is the key causal point for this step: once bursts were flattened, the transport still held too many frames in flight on slower ACK paths because the effective send cap stayed pinned to `64`.

## Design

The new policy is intentionally simple:

- keep the hard response window fixed at `64`
- introduce an `effective_send_cap <= 64`
- derive pressure from recent ACK latency and recent hard-window wait
- reduce the effective cap under pressure
- restore the effective cap only after a clean streak

The control law is:

- hard cap: `64`
- minimum effective cap: `40`
- moderate pressure:
  - recent hard-window wait `>= 24 ms`
  - or occupancy near the hard window with ACK p95 above bootstrap RTT pressure
  - target cap becomes `60`
- severe pressure:
  - recent hard-window wait `>= 96 ms`
  - or occupancy near the hard window with ACK avg above stronger pressure threshold
  - target cap becomes `56`
- restore behavior:
  - no immediate bounce back to `64`
  - require `6` clean samples
  - then restore by `+2` frames at a time

This keeps the model monotonic and explainable:

- the hard protocol limit never changes
- the controller only tightens or relaxes how close the sender is allowed to run to that limit
- there is no cubic, BBR, dynamic cwnd, or loss-driven congestion controller here

## Implementation Notes

Main code paths:

- `src/roles/exit.rs`
  - `ResponseInflightDisciplineConfig`
  - `ResponseInflightDisciplineState`
  - `response_inflight_target_cap_frames(...)`
  - `response_inflight_restore_is_clean(...)`
  - send-path enforcement of `effective_send_cap`
- `src/config.rs`
- `src/node_config.rs`
- `src/main.rs`

Support changes needed to keep artifacts truthful:

- `src/stage_trace.rs`
  - stage-trace output path now refreshes from env instead of staying frozen in a process-global `OnceLock`
- `tests/chaos_matrix.rs`
  - local stack tasks are explicitly aborted after each scenario
  - the inflight regression uses `5` runs to stabilize the A/B sample for this control policy
- `scripts/summarize_perf_stages.py`
  - now reports effective-cap metrics and blocking counters

## What The Controller Actually Did

Remote stage sample after the fix:

| route | effective cap avg | cap reduced total | blocked by effective cap | ACK p95 ms | hard-window stall ms | effective-cap wait ms |
| --- | --- | --- | --- | --- | --- | --- |
| 1-hop | 63.4 | 2 | 2 | 340.2 | 199.4 | 6.0 |
| 2-hop | 63.8 | 1 | 1 | 263.6 | 105.8 | 0.8 |
| 3-hop | 61.2 | 3 | 189 | 269.8 | 24.2 | 198.0 |

The controller engaged most strongly on `3-hop`. That is exactly where the hard-window pressure was most expensive in the pacing-era baseline.

## Honest Outcome

This step did not produce a uniform end-to-end win on every path.

What it did prove:

- the exit no longer has only one response mode: “run all the way to `64`”
- effective-cap control engaged on real WAN traffic
- `3-hop` showed the clearest intended behavior: lower effective cap, much lower hard-window stall, and lower ACK p95

What it did not prove:

- a route-independent latency improvement across `1-hop`, `2-hop`, and `3-hop`

That is expected for a baseline pressure-response layer. The next remaining performance problem is not burst-send or lifecycle semantics. It is the lack of a fuller RTT-aware congestion response and retransmit/backoff coupling on top of this baseline controller.
