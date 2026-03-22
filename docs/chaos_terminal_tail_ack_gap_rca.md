# Chaos Terminal-Tail / ACK-Gap RCA

## Summary

After the duplicate-noise classification fix, the next failure-looking signature under `combined` chaos was no longer duplicate packet reject noise and no longer terminal close duplication inside the buffered settlement loop.

The dominant remaining anomaly was `ack_regression_ignored` on the exit response path while `response_transport_terminal_close_duplicate` was already zero. The transport still delivered complete `200 OK` responses, but the ACK trace looked non-monotonic and therefore still resembled a transport-risk candidate.

## Symptom

In the pre-fix targeted combined-chaos run:

- `response_transport_terminal_close_duplicate = 0`
- `duplicate_close_stream_suppressed = 32`
- `response_transport_terminal_close_observed = 16`
- `ack_regression_ignored = 233`
- `stale_ack_ignored = 358`
- `ack_gap_detected = 0`

This means the terminal-close path was already idempotent enough to stop looking like the primary failure mode, while the ACK tail still produced a large volume of regression-shaped events.

## Trigger Profile

The trigger is the existing `combined` deterministic chaos profile:

- loss `1000 ppm`
- duplication `2000 ppm`
- reordering `15000 ppm`
- base delay `8 ms`
- jitter `6 ms`
- reorder extra delay `30 ms`

Under that profile, older cumulative ACKs can arrive after the ACK floor has already advanced.

## Root Cause

One concrete root cause:

`ReliableStream::apply_ack_with_latency_details()` classified any cumulative ACK lower than the highest ACK seen so far as `Regression`.

That is too strict for the transport we actually have under combined chaos. Once a cumulative ACK floor has advanced, a later lower ACK is not a new transport failure. It is an old ACK arriving late because the network duplicated or reordered it.

So the code was mixing two different things:

- a harmless stale duplicate ACK
- a real monotonicity violation

Because those were both routed through the regression branch, the exit stage trace produced a failure-looking tail even though inflight cleanup remained correct and the body lifecycle had already settled normally.

## Fix

The fix is a monotonic cumulative-ACK policy in `src/stream_reliable.rs`:

- `ack_seq > highest_ack_seq_seen` => `Advanced`
- `ack_seq <= highest_ack_seq_seen` => `Stale`

This keeps one ACK floor per stream and makes any reordered or duplicated older ACK explicitly harmless. It also keeps inflight cleanup monotonic because only `Advanced` ACKs remove new frame ranges.

In the same transport-tail path, the client continues to treat duplicate `CloseStream` markers as idempotent settlement traffic instead of lifecycle failure. That preserves the previously accepted duplicate-noise and delay-tail fixes while the new ACK policy removes the next misleading signal.

## Why This Was The Next Limiter

The targeted before/after regression shows the ordering clearly:

- terminal close duplicates were already not re-entering settlement (`response_transport_terminal_close_duplicate = 0` both before and after)
- ACK gaps were not showing up (`ack_gap_detected = 0` both before and after)
- the only remaining high-volume failure-looking counter was `ack_regression_ignored`
- after the monotonic ACK-floor fix, that counter drops to `0` and the same traffic is counted as `stale_ack_ignored`

That is why the limiter for this step is ACK monotonic accounting, not replay-window policy, not duplicate packet classification, and not response completion timing.

## Validation Inputs

- Targeted regression log: `docs/artifacts/chaos_terminal_tail_ack_gap_after_2026-03-22.log`
- Before stage trace: `docs/artifacts/chaos_terminal_tail_ack_gap_before_2026-03-22.stage.jsonl`
- After stage trace: `docs/artifacts/chaos_terminal_tail_ack_gap_after_2026-03-22.stage.jsonl`
- Full chaos matrix summary: `docs/artifacts/chaos_matrix_2026-03-22_terminal_tail_ack_gap_fix.summary.json`
- Remote stage summary: `docs/artifacts/chaos_terminal_tail_ack_gap_remote_perf_stage_2026-03-22.md`
