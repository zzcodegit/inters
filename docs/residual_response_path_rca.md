# Residual Response-Path RCA

## Scope

This follow-up targets the residual response-path instability that remained after the accepted fixes for:

- response lifecycle cleanup
- buffered HTTP completion
- exit response window sizing

The two concrete goals were:

1. explain the probe-stream `stream_id=1000000000` lifecycle and the old `MISSING response channel` noise
2. try one focused ACK/retransmit fix without breaking the accepted local baseline, remote matrix, perf runner, or stage runner

## Probe Stream `1000000000`

`stream_id=1000000000` is not a user request stream. It is the client-side warmup RTT probe that is created in `run_client_tcp_mode()` before local TCP accept-loop traffic begins:

- `next_probe_stream_id` starts at `1_000_000_000`
- the probe request comes from `build_probe_request()`
- the probe runs `tunnel_http_roundtrip()` over each candidate route to seed route metrics

That means probe traffic is expected in both local and remote logs. What was *not* expected was the old console error:

`client: MISSING response channel for response payloads stream_id=1000000000`

### Established root cause

The old error was reproduced only in the remote perf/stage harness, not in the per-stream stage trace for a healthy probe roundtrip.

The harness used to:

- start the remote client in-process with `tokio::spawn(...)`
- abort only the outer task at scenario end

`run_client()` internally spawns detached background tasks, including the probe loop and UDP receive path. Aborting the outer join handle could leave those detached tasks alive briefly while the next scenario started. Because the probe stream counter restarts from `1_000_000_000` in each fresh client, late packets could hit a half-torn local state and produce `MISSING response channel`.

### Fix

Remote baseline/perf/stage runners now spawn a managed child `vpnnode client ...` process and stop it explicitly at scenario end.

Changed files:

- `tests/baseline/support.rs`
- `tests/remote_baseline.rs`
- `tests/remote_perf.rs`
- `tests/remote_perf_stages.rs`

### Evidence

Before:

- `remote_perf_matrix_2026-03-21_overlay_window_fix.log`: `4` matches
- `remote_perf_stage_matrix_2026-03-21_overlay_window_fix.log`: `6` matches

After:

- `remote_perf_matrix_2026-03-21_residual_fix.log`: `0` matches
- `remote_perf_stage_matrix_2026-03-21_residual_fix.log`: `0` matches

The probe stream still appears in stage trace, but now it completes cleanly and is classified correctly:

- `roundtrip_started`
- `response_channel_created`
- `response_payload_received`
- `response_channel_removed`
- `stream_complete`

Late follow-up packets are now classified as `late_close_after_completion` or `late_payload_after_completion`, which is the expected tombstone behavior rather than a missing-channel failure.

## ACK / Retransmit Tuning

## Why this was the next candidate

After the exit response-window fix, the remaining stage breakdown still showed the main steady-state cost on the overlay response path:

- `overlay pre-first-byte gap`
- `overlay first-send -> client first-byte gap`
- `window/backpressure stall`
- `exit/body tail`

The strongest signal was that exit retransmit still used a fixed `300 ms` interval while the measured WAN ACK `p95` had already reached roughly `311-394 ms` on the accepted post-window snapshot. That is aggressive enough to trigger premature retransmits on moderate paths.

### Fix

Exit response retransmit interval is now adaptive per stream:

- base interval: `350 ms`
- safety margin: `+75 ms`
- source of truth: stream ACK latency summary (`p95`, then `avg`, then `p50`)
- clamp: `<= 1500 ms`

Code:

- `src/roles/exit.rs`

Tests:

- `adaptive_retransmit_interval_uses_base_without_ack_samples`
- `adaptive_retransmit_interval_tracks_high_ack_p95`
- `adaptive_retransmit_interval_is_clamped`

## Result

The effect is real, but it is not uniform across all WAN paths.

### Stable improvement

`remote-2hop` improved in the new stage sample:

- retransmit rate: `0.1058 -> 0.0000`
- ACK `p95`: `390.6 ms -> 371.2 ms`
- window stall: `1051.8 ms -> 933.6 ms`
- exit/body tail: `1177.2 ms -> 1056.2 ms`

This is the cleanest evidence that the adaptive interval removed premature retransmit pressure on a middle-quality path.

### What did not magically improve

`remote-3hop` got worse in the new WAN sample:

- total: `1590.06 ms -> 3955.00 ms`
- retransmit rate: `0.1820 -> 0.3388`
- ACK `p95`: `394.0 ms -> 1164.8 ms`
- window stall: `1223.4 ms -> 2697.4 ms`

That is too large to explain as a local lifecycle bug. The data points to a materially worse live route sample: ACK latency itself exploded, and backpressure followed it.

`remote-1hop` also moved in the wrong direction in this sample:

- total: `1205.89 ms -> 1451.97 ms`
- ACK `p95`: `311.2 ms -> 415.2 ms`

This means the adaptive retransmit fix is not enough to dominate ambient WAN variance by itself.

## Ranking After This Fix

What is no longer the main limiter:

- probe-stream lifecycle noise
- fixed `300 ms` retransmit policy as a universal issue

What still dominates:

1. route-quality driven ACK latency on weaker paths
2. window/backpressure stalls that follow that ACK latency
3. remaining overlay first-send -> client first-byte gap

In other words: the accepted lifecycle race is closed, and retransmit policy is better behaved, but the worst remaining tail now tracks route quality much more than local client cleanup.

## Honest conclusion

This step should be accepted as a stability fix first and a transport tuning step second.

What is established:

- the probe-stream `MISSING response channel` issue was a real harness/lifecycle artifact
- it is now eliminated in the reproduced perf/stage scenario
- the adaptive retransmit interval measurably improved the `2-hop` path and removed premature retransmit noise there

What is *not* established:

- that the new retransmit tuning makes every live WAN path faster
- that `3-hop` is now transport-clean in absolute terms

The remaining top bottleneck is now best described as: **route-quality induced ACK latency and the backpressure tail that follows it**.
