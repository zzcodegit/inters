# Stage 5.1 - RTT-Aware Response Pacing RCA

## Scope

This step adds a baseline RTT-aware response pacing layer on top of the fixed exit response window of `64` frames.

It does **not** add:

- dynamic congestion window sizing
- CUBIC/BBR-style control
- route scoring changes
- replay/lifecycle/chaos semantics changes

## Bottleneck Proof

The accepted Stage 5 baseline already showed that the dominant steady-state cost lived in the exit response path after the target was already reachable:

- before pacing, remote stage baseline in [stage5_entry_perf_stage_2026-03-22.md](/C:/neinternet/vpnnode/docs/artifacts/stage5_entry_perf_stage_2026-03-22.md) had:
- `remote-1hop`: `ack p95=306.4 ms`, `window stall=817.4 ms`, `avg inflight=58.96`, `max inflight=64`
- `remote-2hop`: `ack p95=293.0 ms`, `window stall=800.6 ms`, `avg inflight=60.78`, `max inflight=64`
- `remote-3hop`: `ack p95=325.8 ms`, `window stall=1135.2 ms`, `avg inflight=61.55`, `max inflight=64`
- baseline correlation there was `total_time_ms` vs `ack_latency_ms_avg = 0.976`

That evidence shows the old path was repeatedly filling the fixed window to its ceiling and then waiting on ACK release.

The final targeted local regression in:

- [stage5_pacing_targeted_before_2026-03-22.jsonl](/C:/neinternet/vpnnode/docs/artifacts/stage5_pacing_targeted_before_2026-03-22.jsonl)
- [stage5_pacing_targeted_before_2026-03-22.stage.jsonl](/C:/neinternet/vpnnode/docs/artifacts/stage5_pacing_targeted_before_2026-03-22.stage.jsonl)

reproduces the old burst path directly on the current codebase with pacing disabled:

- `max_send_burst_frames = 37.0`
- `avg TTFB = 1221.57 ms`
- `avg total = 1223.07 ms`

That is the formal proof that the old exit path still behaved as a burst sender, not a paced sender.

## Root Cause

One root cause:

- the exit response loop in [exit.rs](/C:/neinternet/vpnnode/src/roles/exit.rs) sent response DATA frames back-to-back until `ReliableStream::inflight()` hit the fixed window limit
- once inflight hit `64`, the loop could only progress when ACKs released the range
- this created ACK bunching, then window stall, then long overlay first-byte / body-tail latency

This was a transport send-discipline issue, not a route-quality issue and not a replay/lifecycle issue.

## Design

The implemented pacing policy is intentionally simple:

- observed RTT source: `ack_latency_summary().avg_ms`, then `p50_ms`, then `p95_ms`
- bootstrap RTT fallback: `200 ms`
- fixed window: `64`
- pacing interval:

`pacing_interval_ms = max(min_interval_ms, ceil(observed_rtt_ms / 64))`

- `min_interval_ms = 1`
- small token-paced microburst cap: `4` frames
- start budget: `4` frames

Implementation details:

- pacing is exit-only, on the response DATA send path
- the window size stays fixed at `64`
- terminal markers and retransmit policy stay intact
- no dynamic congestion window resizing is introduced

## Implementation

Files changed for the pacing step:

- [config.rs](/C:/neinternet/vpnnode/src/config.rs)
- [node_config.rs](/C:/neinternet/vpnnode/src/node_config.rs)
- [main.rs](/C:/neinternet/vpnnode/src/main.rs)
- [exit.rs](/C:/neinternet/vpnnode/src/roles/exit.rs)
- [chaos_matrix.rs](/C:/neinternet/vpnnode/tests/chaos_matrix.rs)
- [summarize_perf_stages.py](/C:/neinternet/vpnnode/scripts/summarize_perf_stages.py)

New exit metrics emitted into `stream_complete`:

- `pacing_delay_applied`
- `pacing_delay_applied_ms_total`
- `pacing_interval_ms_avg`
- `pacing_interval_ms_max`
- `burst_prevented_count`
- `paced_send_batches`
- `max_send_burst_frames`
- `pacing_enabled`

## What Changed In Metrics

Targeted local regression, before vs after:

- `max_send_burst_frames: 37.0 -> 5.0`
- `avg TTFB: 1221.57 ms -> 613.60 ms`
- `avg total: 1223.07 ms -> 615.84 ms`

Remote stage compare, before vs after:

- `remote-1hop`
  - `ack p95: 306.4 -> 190.0 ms`
  - `window stall: 817.4 -> 32.4 ms`
  - `total: 1099.25 -> 1113.37 ms`
- `remote-2hop`
  - `ack p95: 293.0 -> 200.4 ms`
  - `window stall: 800.6 -> 44.0 ms`
  - `total: 1125.36 -> 1133.96 ms`
- `remote-3hop`
  - `ack p95: 325.8 -> 288.2 ms`
  - `window stall: 1135.2 -> 155.0 ms`
  - `total: 1524.05 -> 1517.50 ms`

That is the honest result of this baseline pacing step:

- burst size and window stall are no longer the dominant failure-looking signature
- ACK tail is materially lower
- end-to-end total time moves only slightly on `1-hop/2-hop`, and slightly down on `3-hop`

## Conclusion

Baseline RTT-aware pacing is now present and observable.

The old `burst until inflight=64` path is gone from the response DATA sender. The next performance bottleneck is no longer raw burst-send; it is the remaining fixed-window / no-congestion-response discipline on top of the now-paced sender.
