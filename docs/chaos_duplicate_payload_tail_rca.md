# Chaos RCA: Duplicate Payload Tail After Local Completion

## Summary

После `terminal payload tail` fix следующий remaining lifecycle limiter был не в ACK/gap, не в replay-window и не в terminal-close path.

Следующей residual сигнатурой оставался только `duplicate_payload_after_local_completion` на completed/tombstone path.

## Symptom

До этого фикса:

- local combined trace from [docs/artifacts/chaos_matrix_2026-03-22_terminal_payload_tail_fix_combined.stage.jsonl](/C:/neinternet/vpnnode/docs/artifacts/chaos_matrix_2026-03-22_terminal_payload_tail_fix_combined.stage.jsonl):
  - `duplicate_payload_after_local_completion = 4`
  - `duplicate_payload_after_local_completion_repeat = 213`
  - `payload_after_local_completion_during_settlement = 0`
- remote client stage trace from [docs/artifacts/chaos_terminal_payload_tail_remote_perf_stage_2026-03-22.client.jsonl](/C:/neinternet/vpnnode/docs/artifacts/chaos_terminal_payload_tail_remote_perf_stage_2026-03-22.client.jsonl):
  - `duplicate_payload_after_local_completion = 1`
  - `duplicate_payload_after_local_completion_repeat = 3`
  - `payload_after_local_completion_during_settlement = 0`
- at the same time:
  - `ack_gap_detected = 0`
  - `ack_regression_ignored = 0`
  - `terminal_payload_after_local_completion = 0`

Это и доказывало, что next limiter сидел именно в non-terminal duplicate payload tail after local completion.

## Root Cause

Одна причина:

completed/tombstone branch в [src/roles/client.rs](/C:/neinternet/vpnnode/src/roles/client.rs) продолжал отправлять fully ACK-covered duplicate DATA frames в anomaly bucket `duplicate_payload_after_local_completion`.

Классификация основывалась на:

- `late_bytes == 0`
- `!end_of_stream`
- `frame_seq < ack_seq`

Это не аномалия доставки, а уже покрытый cumulative ACK duplicate DATA tail после cleanup.

До фикса targeted before-trace в [docs/artifacts/chaos_duplicate_payload_tail_before_2026-03-22.stage.jsonl](/C:/neinternet/vpnnode/docs/artifacts/chaos_duplicate_payload_tail_before_2026-03-22.stage.jsonl) показывал типичный residual event:

- `frame_seq=20`
- `ack_seq=38`
- `completion_reason=content_length_reached`

То есть duplicate frame уже был полностью за ACK floor, но код всё ещё учитывал его как post-completion duplicate anomaly.

## Structural Fix

Фикс в [src/roles/client.rs](/C:/neinternet/vpnnode/src/roles/client.rs):

- введена явная policy `is_ack_covered_duplicate_payload_tail(...)`
- такие frames больше не идут в `duplicate_payload_after_local_completion`
- вместо этого completed/tombstone path поглощает их через explicit settlement policy event:
  - `duplicate_payload_after_local_completion_absorbed`

Это именно completed/tombstone dispatch fix.

Он не меняет:

- replay-window
- stale ACK policy
- timeouts
- route scoring
- congestion behavior

## Result

После фикса:

- targeted regression:
  - `duplicate_payload_after_local_completion: 9 -> 0`
  - `duplicate_payload_after_local_completion_repeat: 713 -> 0`
  - `duplicate_payload_after_local_completion_absorbed: 0 -> 659`
- current full local chaos matrix:
  - old duplicate counters = `0`
- current remote client stage trace:
  - old duplicate counters = `0`

Before/after compare: [docs/artifacts/chaos_duplicate_payload_tail_compare_2026-03-22.md](/C:/neinternet/vpnnode/docs/artifacts/chaos_duplicate_payload_tail_compare_2026-03-22.md)

Remote evidence with exact hops: [docs/artifacts/chaos_duplicate_payload_tail_remote_evidence_2026-03-22.md](/C:/neinternet/vpnnode/docs/artifacts/chaos_duplicate_payload_tail_remote_evidence_2026-03-22.md)
