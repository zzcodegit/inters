# Response Path Reliability RCA

## Summary

The client-side `MISSING response channel for response payloads` error was a real transport/session lifecycle bug, not just a noisy log line.

Before the fix, `tunnel_http_roundtrip()` removed the per-stream response sender, route, and reliable state as soon as the local consumer considered the response complete. Late response `Data` and `CloseStream` messages still arrived on the UDP receive path after that cleanup window. Once the active `ReliableStream` state had been removed, the receive path recreated a fresh empty reliable stream for the same `stream_id`, which destroyed cumulative ACK state and could turn a late duplicate into another retransmit tail.

That race explains both symptoms:

- noisy `client: MISSING response channel...` errors in successful remote perf runs
- a longer overlay response tail caused by avoidable retransmit / ACK churn after logical completion

## Exact Lifecycle Before The Fix

Active TCP-mode response state was split across four in-memory maps in [src/roles/client.rs](/C:/neinternet/vpnnode/src/roles/client.rs):

- response channel created in `tunnel_http_roundtrip()` and inserted into `response_senders`
- per-stream reliable state stored in `reliable_streams`
- per-stream route stored in `stream_routes`
- first-response bookkeeping stored in `response_started`

The same function removed the active stream immediately after its local receive loop exited. The UDP receive path processed response `Data` for the same `stream_id` independently, so late packets could arrive after active cleanup.

## Reproduction

The issue reproduced honestly on the accepted WAN topology through the existing remote runners:

- `bash scripts/run_perf_matrix.sh`
- `bash scripts/run_perf_stage_matrix.sh`

Before-fix evidence is already committed:

- [remote_perf_matrix_2026-03-21_post_stage.log](/C:/neinternet/vpnnode/docs/artifacts/remote_perf_matrix_2026-03-21_post_stage.log)
- [remote_perf_stage_matrix_2026-03-21.log](/C:/neinternet/vpnnode/docs/artifacts/remote_perf_stage_matrix_2026-03-21.log)

Count summary:

- before perf log: `31` matches
- before stage log: `12` matches
- after perf log: `0` matches
- after stage log: `0` matches

See [response_path_fix_missing_check_2026-03-21.txt](/C:/neinternet/vpnnode/docs/artifacts/response_path_fix_missing_check_2026-03-21.txt).

## Root Cause

The established root cause is **premature response-stream cleanup combined with late response traffic**.

What happened before the fix:

1. Client finished the local response loop and removed active state.
2. Exit or relays still delivered late response frames or `CloseStream`.
3. Client receive path no longer found a response sender.
4. Client also no longer had the original reliable receive state, so a fresh `ReliableStream` could be created for the same `stream_id`.
5. That new reliable state had `recv_next = 0`, which lost cumulative ACK progress for the already-completed stream.
6. Late traffic could therefore extend response-path churn instead of being cleanly recognized as post-completion duplicates.

This is why the bug was linked to overlay tail latency instead of being “just logging”.

## Fix

The fix is structural and keeps the transport semantics honest:

- Added a completed-response tombstone table in [src/roles/client.rs](/C:/neinternet/vpnnode/src/roles/client.rs).
- On completion, the client now keeps the original route and `ReliableStream` state for a short linger window instead of forgetting them immediately.
- Active response channels are still removed, but late payloads and late `CloseStream` are now handled via completed-state lookup.
- Late traffic is traced explicitly instead of being misreported as a missing live channel.
- The receive path now reuses retained cumulative ACK state for late duplicates instead of recreating an empty reliable stream for the same `stream_id`.

New lifecycle diagnostics added in [src/roles/client.rs](/C:/neinternet/vpnnode/src/roles/client.rs):

- `response_channel_created`
- `response_payload_received`
- `response_channel_removed`
- `response_timeout`
- `late_payload_after_completion`
- `late_close_after_completion`

## Regression Test

A focused unit regression was added in [src/roles/client.rs](/C:/neinternet/vpnnode/src/roles/client.rs) as `completed_response_stream_retains_ack_state_for_late_duplicates`.

It proves the class of bug directly:

- a stream is completed
- active channel is gone
- a late duplicate frame arrives
- the retained reliable state still returns the correct cumulative ACK instead of resetting to `0`

## Validation After The Fix

Green checks:

- local baseline: [response_path_fix_baseline_2026-03-21.log](/C:/neinternet/vpnnode/docs/artifacts/response_path_fix_baseline_2026-03-21.log)
- readiness test: [response_path_fix_http_probe_ready_2026-03-21.log](/C:/neinternet/vpnnode/docs/artifacts/response_path_fix_http_probe_ready_2026-03-21.log)
- remote WAN matrix: [remote_wan_matrix_2026-03-21_response_fix_retry.log](/C:/neinternet/vpnnode/docs/artifacts/remote_wan_matrix_2026-03-21_response_fix_retry.log)
- remote perf runner: [remote_perf_matrix_2026-03-21_response_fix.log](/C:/neinternet/vpnnode/docs/artifacts/remote_perf_matrix_2026-03-21_response_fix.log)
- remote stage runner: [remote_perf_stage_matrix_2026-03-21_response_fix.log](/C:/neinternet/vpnnode/docs/artifacts/remote_perf_stage_matrix_2026-03-21_response_fix.log)

After-fix client stage evidence:

- lifecycle trace: [remote_perf_stage_matrix_2026-03-21_response_fix.client.jsonl](/C:/neinternet/vpnnode/docs/artifacts/remote_perf_stage_matrix_2026-03-21_response_fix.client.jsonl)
- stage summary: [remote_perf_stage_matrix_2026-03-21_response_fix.md](/C:/neinternet/vpnnode/docs/artifacts/remote_perf_stage_matrix_2026-03-21_response_fix.md)

Observed after-fix lifecycle counts:

- `response_channel_created=21`
- `response_payload_received=20`
- `response_channel_removed=21`
- `late_payload_after_completion=4`
- `late_close_after_completion=54`
- `response_timeout=1`

The important behavioral change is that these late events are now classified and tolerated instead of surfacing as `MISSING response channel`.

## Residual Notes

- Successful remote perf runs still often finish with `completion_reason=buffer_cap_reached`. That is separate from this fix and points to a remaining response-buffer heuristic issue in the buffered HTTP path.
- The accepted WAN environment still needs topology resets between remote scenarios because relay/exit runtime is effectively single-session.
