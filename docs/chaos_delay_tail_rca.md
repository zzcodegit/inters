# Delay-Driven Response Tail RCA

## Scope

This document closes the next chaos limiter after the replay-window fix: delayed response-tail traffic that previously surfaced as `late_payload_after_completion`.

The investigation was intentionally limited to one bottleneck:

- buffered HTTP response path after `content_length_reached`
- mixed/mild delay chaos profiles
- no replay-window changes
- no ACK / route-scoring tuning in this step

## Root Cause

The root cause was a lifecycle mismatch in the client response model:

1. Buffered HTTP considered the response locally complete as soon as `Content-Length` bytes were buffered.
2. The client then moved the stream toward completed cleanup too early relative to transport-tail behavior.
3. Under delay, the exit could still deliver already-ACKed response DATA duplicates after local body completion.
4. Those packets carried no new bytes (`delivered_bytes=0`) and were older than the current cumulative ACK horizon, but the client still classified them as `late_payload_after_completion`.

So the failure mode was not "missing body bytes after 200 OK" anymore. After the truncation fix, the remaining issue was that post-completion duplicate response frames were still being interpreted as late payload instead of being handled as transport-tail duplicates.

## Code Path

- Buffered body completion happens in [`src/roles/client.rs`](/C:/neinternet/vpnnode/src/roles/client.rs) inside `tunnel_http_roundtrip()`.
- Post-completion receive-path dispatch also lives in [`src/roles/client.rs`](/C:/neinternet/vpnnode/src/roles/client.rs), in the `MsgType::Data` handling branch.

The structural gap was that the client had only two meaningful states for buffered responses:

- active delivery
- completed cleanup

It did not model the transport-tail phase explicitly enough.

## Structural Fix

The fix has two parts, both in [`src/roles/client.rs`](/C:/neinternet/vpnnode/src/roles/client.rs):

1. Two-phase completion for buffered `Content-Length` responses.
   - `response_transport_local_completion`
   - `response_transport_settlement_started`
   - cleanup deferred into `settle_buffered_response_transport()`
   - cleanup only after terminal signals plus a quiet period, not immediately at local body completion

2. Separate classification for post-completion duplicate DATA frames.
   - true late payload remains `late_payload_after_completion`
   - already-delivered duplicate body frames become:
     - `duplicate_payload_after_local_completion`
     - `duplicate_payload_after_local_completion_repeat`

This means:

- new bytes after completion are still treated as a real problem
- duplicate retransmit tail is no longer mislabeled as late payload
- response cleanup remains bounded and does not rely on inflating request/response timeouts

## Why This Is The Right Fix

This step does not "hide" the transport tail.

The client still records post-completion activity, but it now distinguishes three cases:

- local body complete
- transport settlement in progress
- duplicate transport tail after local completion

That matches the actual behavior under delay much better than the old binary active/completed model.

## Key Artifacts

- Targeted regression log:
  - [chaos_delay_tail_targeted_regression_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_delay_tail_targeted_regression_2026-03-22.log)
- Before/after local chaos compare:
  - [chaos_delay_tail_compare_2026-03-22.md](/C:/neinternet/vpnnode/docs/artifacts/chaos_delay_tail_compare_2026-03-22.md)
- Before stage traces:
  - [chaos_matrix_2026-03-22_replay_fix_clean_mild-delay.stage.jsonl](/C:/neinternet/vpnnode/docs/artifacts/chaos_matrix_2026-03-22_replay_fix_clean_mild-delay.stage.jsonl)
  - [chaos_matrix_2026-03-22_replay_fix_clean_combined.stage.jsonl](/C:/neinternet/vpnnode/docs/artifacts/chaos_matrix_2026-03-22_replay_fix_clean_combined.stage.jsonl)
- After stage traces:
  - [chaos_matrix_2026-03-22_delay_tail_fix_mild-delay.stage.jsonl](/C:/neinternet/vpnnode/docs/artifacts/chaos_matrix_2026-03-22_delay_tail_fix_mild-delay.stage.jsonl)
  - [chaos_matrix_2026-03-22_delay_tail_fix_combined.stage.jsonl](/C:/neinternet/vpnnode/docs/artifacts/chaos_matrix_2026-03-22_delay_tail_fix_combined.stage.jsonl)

## Remaining Risk

This change removes `late_payload_after_completion` as the dominant delay-tail symptom in the current chaos matrix.

A small number of ultra-late terminal markers can still arrive after cleanup and are now tracked separately as terminal duplicates. They are not new body bytes, they do not corrupt `Content-Length`, and they are not counted as late payload anymore.
