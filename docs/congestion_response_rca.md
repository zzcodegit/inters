# Lightweight Congestion Response RCA

## Scope

This step adds a lightweight RTT/loss-aware congestion response on top of the existing:

- response pacing
- RTT-aware inflight discipline
- RTT-aware retransmit backoff

It does **not** introduce full congestion control. The hard response window stays `64`.

## Problem Statement

After the retransmit-backoff step, the long overlay path still showed a clear control-plane gap:

- `remote-5hop` had `retransmit_rate = 0.2810`
- `ack_latency_ms_avg = 271.20`
- `ack_latency_ms_p95 = 293.80`
- `effective_inflight_cap_avg = 64`
- `send_blocked_by_effective_cap = 0`
- `congestion_events = 0` because no sustained congestion state existed yet

Source: [docs/artifacts/retransmit_backoff_remote_perf_stage_2026-03-23.md](/C:/neinternet/vpnnode/docs/artifacts/retransmit_backoff_remote_perf_stage_2026-03-23.md)

That combination means the exit continued to drive the path almost normally while the path already had:

- elevated RTT
- repeated retransmits
- near-max in-flight occupancy

## Bottleneck Proof

The current bottleneck is not burst-send anymore:

- `max_send_burst_frames = 4` on `1-hop/2-hop/3-hop/5-hop`
- pacing is already active

The remaining issue is missing coordinated pressure response under sustained congestion:

- `remote-1hop`: `ack p95 = 186.80`, `retransmit_rate = 0.0000`, `blocked_by_cap = 0`
- `remote-2hop`: `ack p95 = 186.60`, `retransmit_rate = 0.0000`, `blocked_by_cap = 0`
- `remote-3hop`: `ack p95 = 243.80`, `retransmit_rate = 0.0000`, `blocked_by_cap = 0`
- `remote-5hop`: `ack p95 = 349.60`, `retransmit_rate = 0.0000` in the post-step run, but `congestion_events = 2`, `congestion_duration_ms = 180.80`, `send_blocked_by_effective_cap = 5`

Source: [docs/artifacts/congestion_response_remote_perf_stage_2026-03-24.md](/C:/neinternet/vpnnode/docs/artifacts/congestion_response_remote_perf_stage_2026-03-24.md)

The before/after pair shows the gap directly:

- before congestion response: `remote-5hop` had heavy retransmit pressure and no coordinated response
- after congestion response: `remote-5hop` enters explicit congestion state, adds mild backpressure, and keeps `1-hop/2-hop/3-hop` untouched

## Root Cause

One root cause:

The response path had pacing and retransmit tuning, but it still lacked a sustained congestion signal that combined:

- RTT growth relative to a route-local baseline
- retransmit rate
- retransmit burst pressure
- near-window in-flight occupancy

So the exit could react to individual events, but it could not shift into a coordinated response mode when the path stayed congested across multiple samples.

## Why This Step Is Selective

The new signal is intentionally gated so that good routes stay at the default behavior:

- enough frames must be sent before congestion is considered
- in-flight occupancy must be close to the hard window
- the signal must persist for more than one sample
- restore requires a clean streak

That is why the current run shows:

- `1-hop/2-hop/3-hop`: `congestion_events = 0`
- `5-hop`: `congestion_events = 2`

## Out Of Scope

This step does not do:

- CUBIC
- BBR
- dynamic hard-window resizing
- route scoring changes
- retransmit policy changes

Those remain separate Stage 5 follow-up areas.
