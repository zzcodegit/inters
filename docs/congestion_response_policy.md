# Lightweight Congestion Response Policy

## Purpose

Provide a small, explainable response when the path shows sustained congestion, without turning the transport into a full congestion controller.

## Inputs

The signal uses:

- ACK latency growth relative to a route-local baseline
- retransmit rate
- repeated retransmit burst pressure
- near-window in-flight occupancy

## Congestion Levels

### `NONE`

Default state.

- effective cap stays at the inflight-discipline result
- no extra pacing delay is added

### `MILD`

Triggered only after a sustained pressure streak.

- effective cap limit: `60`
- extra pacing delay: `+1 ms`

### `SEVERE`

Triggered only after a stronger sustained pressure streak.

- effective cap limit: `56`
- extra pacing delay: `+2 ms`

## Detection Rules

The signal is considered only when all of these are true:

- enough response frames have been sent
- current in-flight occupancy is near the hard window
- the signal persists across multiple samples

The pressure input is considered elevated when one of these is true:

- ACK growth is above the mild or severe threshold relative to baseline
- retransmit rate crosses the mild or severe threshold
- retransmit burst depth crosses the mild or severe threshold

## Restore Rules

The state does not drop immediately on one clean sample.

- a clean streak is required
- restore is stepwise
- cap and pacing return gradually

This prevents oscillation on routes with short transient spikes.

## Hard Constraints

This policy does **not**:

- change the hard response window `64`
- alter routing
- alter replay/lifecycle/chaos semantics
- modify retransmit backoff itself
- implement a full congestion controller
