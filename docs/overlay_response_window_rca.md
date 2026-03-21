# Overlay Response Window RCA

## Scope

This note explains the remaining WAN bottleneck investigation after the accepted fixes for:

- response lifecycle cleanup races
- buffered HTTP completion

The question for this step was narrower:

- why `overlay pre-first-byte gap` stayed large
- why `exit/body tail` stayed large
- which single mechanism should be fixed first

## Artifacts

Before the fix:

- perf matrix: `docs/artifacts/remote_perf_matrix_2026-03-21_buffered_fix.md`
- stage matrix: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_buffered_fix.md`

After the fix:

- remote matrix log: `docs/artifacts/remote_wan_matrix_2026-03-21_overlay_window_fix_clean.log`
- perf matrix raw: `docs/artifacts/remote_perf_matrix_2026-03-21_overlay_window_fix.jsonl`
- perf matrix summary: `docs/artifacts/remote_perf_matrix_2026-03-21_overlay_window_fix.md`
- stage matrix raw local: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_overlay_window_fix.local.jsonl`
- stage matrix raw client: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_overlay_window_fix.client.jsonl`
- stage matrix raw exit: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_overlay_window_fix.exit.jsonl`
- stage matrix joined view: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_overlay_window_fix.joined.jsonl`
- stage matrix summary: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_overlay_window_fix.md`

## Root Cause

The dominant steady-state limiter was the hardcoded exit response window:

- response chunk size on exit: `1000` bytes
- old exit response window: `8` frames
- measured ACK latency before this fix: about `315-394 ms`

That gives a rough bandwidth-delay ceiling of:

- `8 * 1000 bytes / 0.315-0.394 s`
- about `20-25 KB/s`

That matches the old observed overlay throughput almost exactly:

- old `1-hop`: `29235 Bps`
- old `2-hop`: `31515 Bps`
- old `3-hop`: `23279 Bps`

So the response path was not mainly blocked by:

- target connect
- target first-byte generation
- buffered response completion
- delay before the exit starts sending

It was mainly constrained by too little response data being allowed in flight relative to WAN ACK latency.

## Proof From New Instrumentation

New exit-side stage trace adds:

- `first_overlay_send`
- `window_wait_events`
- `window_wait_total_ms`
- `window_wait_max_ms`
- ACK latency min/p50/p95/max
- `window_frames`

The post-fix stage summary shows:

- `exit first send delay ms`: `0.40 / 0.00 / 0.20` for `1-hop / 2-hop / 3-hop`
- `window stall ms`: `893.80 / 1051.80 / 1223.40`
- `overlay first-send -> client ms`: `1202.01 / 1396.09 / 1579.78`

That means:

1. The exit is not sitting on the first target byte.
2. The main remaining delay is after the first overlay send.
3. Window/backpressure time explains most of the remaining exit-side tail.

## Structural Fix

The fix was intentionally narrow:

- add exit config `response_window_frames`
- validate it in `8..=256`
- default it to `64`
- replace the hardcoded exit response window `8` with the configured value

Code:

- `src/config.rs`
- `src/node_config.rs`
- `src/main.rs`
- `src/roles/exit.rs`

This keeps the change focused on the proven bottleneck instead of mixing in retransmit or chunk-size changes at the same time.

## Before/After

### Perf Matrix

| scenario | total ms before | total ms after | delta | throughput before | throughput after | delta |
| --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | 9195.18 | 2089.22 | -77.28% | 29235 | 148745 | +408.93% |
| remote-2hop | 8324.90 | 1157.11 | -86.10% | 31515 | 227101 | +620.62% |
| remote-3hop | 11284.50 | 1492.93 | -86.77% | 23279 | 175935 | +655.76% |

### Stage Matrix

| scenario | overlay gap before | overlay gap after | delta | exit tail before | exit tail after | delta |
| --- | --- | --- | --- | --- | --- | --- |
| remote-1hop | 11553.99 | 1202.01 | -89.60% | 11278.40 | 1026.80 | -90.90% |
| remote-2hop | 11873.63 | 1396.09 | -88.24% | 11667.80 | 1177.20 | -89.91% |
| remote-3hop | 14483.25 | 1579.78 | -89.09% | 13967.20 | 1328.00 | -90.49% |

## Why This Fix Worked

After the change:

- response window is `64` frames instead of `8`
- measured ACK p95 is about `311-394 ms`

That gives a rough ceiling of:

- `64 * 1000 bytes / 0.311-0.394 s`
- about `162-206 KB/s`

That lands in the same range as the new measured throughput:

- `1-hop`: `148745 Bps`
- `2-hop`: `227101 Bps`
- `3-hop`: `175935 Bps`

This is the strongest evidence that the old response window was the main limiter.

## Regressions Checked

- local readiness: `docs/artifacts/overlay_response_window_http_probe_ready_2026-03-21.log`
- local baseline: `docs/artifacts/overlay_response_window_local_baseline_2026-03-21.log`
- remote matrix: `docs/artifacts/remote_wan_matrix_2026-03-21_overlay_window_fix_clean.log`
- perf runner: `docs/artifacts/remote_perf_matrix_2026-03-21_overlay_window_fix.log`
- stage runner: `docs/artifacts/remote_perf_stage_matrix_2026-03-21_overlay_window_fix.log`
- cleanup proof: `docs/artifacts/overlay_response_window_cleanup_2026-03-21.log`

## Remaining Bottleneck

The response window fix removed the dominant artificial cap, but the response path is not fully "free" now.

The remaining steady-state cost is mostly:

- overlay first-send to client first-byte transit
- ACK pacing / retransmit behavior on worse routes
- route quality effects as hop-count grows

Target connect and first target byte remain low and are not the next optimization target.
