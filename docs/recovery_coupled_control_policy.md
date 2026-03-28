# Recovery-Coupled Control Policy

## Scope

This policy replaces the rejected RTT-first congestion reaction.

It does **not**:

- change the hard response window (`64`)
- change routing
- change Stage 4 transport semantics
- add full congestion control (`BBR`, `CUBIC`, dynamic cwnd)

It only changes how exit-side send shaping reacts to **active recovery pressure**.

## States

### `none`

Default state.

- effective cap stays at `64`
- no congestion pacing extra is applied

### `recovery_guard`

Entered immediately when a retransmit burst starts under near-window occupancy.

Purpose:

- avoid overshooting while recovery starts
- react in the same time domain as retransmit recovery
- avoid the old lagging RTT-only verdict

Behavior:

- short hold
- small cap reduction to `62`
- no extra pacing bump by default
- quick clear once active recovery disappears

### `sustained_recovery_pressure`

Entered only if recovery pressure keeps going.

Purpose:

- apply bounded shaping only while burst recovery is still active
- keep the response coupled to current loss recovery rather than stale RTT samples

Behavior:

- stronger cap reserve under the hard window
- minimum cap bounded at `48`
- pacing bump allowed only when ACK lag is meaningfully above the current clean RTT reference and repeated loss is present
- hold longer than `recovery_guard`, but still clears back to `none` quickly after recovery disappears

## Entry Signals

The state machine uses:

- `new_retransmits`
- `retransmit_burst`
- `repeated_loss_frames`
- `max_retransmit_count`
- `recovery_active_windows`
- `ack_lag_ms` relative to the current clean RTT reference
- near-window inflight occupancy

It does **not** elevate state based only on a stale high RTT sample.

## Restore Logic

Restore is explicit and state-dependent.

- `recovery_guard` clears after a short clean streak and a short hold
- `sustained_recovery_pressure` clears only after active recovery disappears and a longer hold elapses

The model does not use the old slow, sticky restore staircase.

## Metrics

The model emits:

- `congestion_state`
- `congestion_events`
- `congestion_duration_ms`
- `cap_reduction_due_to_congestion`
- `pacing_increase_due_to_congestion`
- `ack_lag_ms`
- `retransmit_burst`
- `repeated_loss_frames`
- `new_retransmits`
- `recovery_active_windows`

## Operational Reading

Good route or clean run:

- `congestion_events = 0`
- `blocked_by_cap = 0`
- `effective_inflight_cap_avg = 64`

Active burst:

- a `response_congestion_state` event should appear at burst start
- `level` should move to `recovery_guard` immediately
- the state should clear once recovery pressure vanishes

## Current Status

The policy is structurally closer to the desired Stage 5 behavior than the rejected congestion model, but it is not accepted yet because:

- the latest measured `5-hop` bad stream still completed without active congestion state in the measured stream
- the moderate transient-loss regression is still unstable in the full local chaos matrix
