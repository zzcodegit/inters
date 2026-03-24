# Congestion Response Corrected Policy

## Goal

Make congestion response useful on real long-path loss without throttling good paths by default.

## Current Policy

### Entry

Congestion still enters only from sustained pressure, not from one stray sample.

Inputs remain:

- ACK growth relative to the route-local baseline
- retransmit rate
- retransmit burst depth
- near-window in-flight occupancy

### Active Congestion Hold

Once the state enters congestion, it no longer starts restoring after only a few fast loop samples.

The controller now keeps the state for an RTT-aware minimum hold:

- `hold_ms = clamp(baseline_rtt_ms / 2, 100, 400)`

That prevents enter/relieve oscillation while the retransmit burst is still playing out.

### Severe Response

`SEVERE` now uses a deeper cap:

- old severe cap: `56`
- corrected severe cap: `52`

The hard window remains `64`.

### Restore

Restore still requires a clean streak, but it is now gated by the hold window first.

So the controller does not instantly walk back the reduction just because in-flight briefly dips under the hard-window headroom.

## Intention

This is still not full congestion control.

It is a small correction to make the lightweight response:

- more stable on lossy long paths
- less decorative
- less likely to oscillate
