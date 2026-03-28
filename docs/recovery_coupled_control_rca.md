# Recovery-Coupled Control RCA

## Why The Old Model Was Abandoned

The rejected lightweight congestion model used delayed RTT growth as the main trigger for cap and pacing reactions.

That created two failures:

1. The signal often arrived after the retransmit burst had already started.
2. The response reduced throughput without helping active loss recovery.

This was already visible in the rejected validation bundle:

- `1-hop`: total `1105.14 -> 1256.81 ms`, retransmit `0.0000 -> 0.0374`, blocked-by-cap `0 -> 6`
- `3-hop`: total `1386.36 -> 1475.55 ms`, retransmit `0.0000 -> 0.1550`, blocked-by-cap `0 -> 11`
- `5-hop`: total `1772.98 -> 2282.38 ms`, retransmit `0.0152 -> 0.2796`, blocked-by-cap `0 -> 173`

Those rejected-control reference values are reproduced again in:

- `docs/artifacts/recovery_coupled_control_compare_2026-03-28.md`

## New Model Goal

The replacement model couples control state to active recovery instead of stale RTT samples.

The sender now looks at:

- retransmit burst start
- retransmit burst density
- consecutive recovery windows
- ACK lag relative to the current clean RTT reference
- near-window inflight occupancy

It does **not** issue a congestion verdict just because RTT was bad some time earlier.

## What The New Model Demonstrated

On the exact-route reduced fleet rerun from `2026-03-28`, the current candidate no longer harms the short and medium routes the way the rejected model did:

- `1-hop`: total `1156.76 ms`, `+4.67%` vs baseline v2, retransmit `0.0000`, blocked-by-cap `0`
- `2-hop`: total `1170.98 ms`, `-2.74%` vs baseline v2, retransmit `0.0000`, blocked-by-cap `0`
- `3-hop`: total `1394.20 ms`, `+0.57%` vs baseline v2, retransmit `0.0000`, blocked-by-cap `0`
- `5-hop`: total `1957.21 ms`, retransmit `0.0374`, blocked-by-cap `0`

Compared with the rejected model:

- `5-hop` total `2282.38 -> 1957.21 ms`
- `5-hop` retransmit `0.2796 -> 0.0374`
- `5-hop` blocked-by-cap `173 -> 0`

## Temporal Alignment Proof

The raw exit stage trace shows the new state machine entering `recovery_guard` at retransmit burst start rather than after an RTT-only verdict.

Observed event on the exact `5-hop` path:

- site: `stage-remote-5hop-warmup`
- transition: `none -> recovery_guard`
- reason: `retransmit_burst_start`
- `new_retransmits = 5`
- `retransmit_burst = 3`
- `ack_lag_ms = 0`

Then it cleared immediately once recovery pressure disappeared:

- transition: `recovery_guard -> none`
- reason: `no_active_recovery_pressure`

Artifacts:

- `docs/artifacts/recovery_coupled_control_remote_perf_stage_2026-03-28.exit.jsonl`
- `docs/artifacts/recovery_coupled_control_remote_perf_stage_2026-03-28.md`

## Why This Step Is Still Not Accepted

The model is structurally better than the rejected control, but the acceptance bar is still not met.

Two facts keep this step out of acceptance:

1. The latest measured `5-hop` bad stream (`stage-remote-5hop-run2`) still completed with:
   - `retransmit_rate = 0.1869`
   - `retransmit_trigger_count = 54`
   - `ack_latency_ms_p95 = 354`
   - `congestion_events = 0`

   So the measured long-path improvement in this WAN sample cannot be attributed to sustained active control on the measured bad stream itself.

2. The local targeted regression for moderate transient loss is not stable enough inside the full chaos matrix run:
   - `response_recovery_guard_avoids_harmful_throttling_on_moderate_route`
   - latest full-matrix log still fails with `blocked on effective cap = 23`

Artifacts:

- `docs/artifacts/recovery_coupled_control_local_chaos_matrix_2026-03-28.log`
- `docs/artifacts/recovery_coupled_control_targeted_moderate_2026-03-28.log`

## Current Conclusion

The RTT-first congestion model was the wrong control basis and had to be replaced.

The recovery-coupled model is directionally correct because it is temporally aligned with retransmit pressure and it stops the harmful route-agnostic throttling seen in the rejected control.

But the current implementation is not yet accepted as a Stage 5 milestone because the benefit on measured long-path pressure is not yet causally complete and the local moderate-loss guard remains unstable in the full chaos matrix.
