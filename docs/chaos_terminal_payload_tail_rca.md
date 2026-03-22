# Chaos RCA: Residual Terminal Payload Tail

## Summary

После `ACK monotonic` fix следующий remaining limiter under `combined` chaos был не в ACK/gap и не в duplicate-packet accounting.

Следующей failure-looking сигнатурой оставался только `terminal_payload_after_local_completion` в client completed/tombstone path.

## Symptom

До этого фикса:

- `combined` summary в [docs/artifacts/chaos_matrix_2026-03-22_terminal_tail_ack_gap_fix.summary.json](/C:/neinternet/vpnnode/docs/artifacts/chaos_matrix_2026-03-22_terminal_tail_ack_gap_fix.summary.json) показывал:
  - `terminal_payload_after_local_completion = 2`
  - `ack_gap_detected = 0`
  - `ack_regression_ignored = 0`
  - `duplicate_terminal_payload_after_local_completion = 0`
- targeted regression before-fix в [docs/artifacts/chaos_terminal_payload_tail_before_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_terminal_payload_tail_before_2026-03-22.log) воспроизводил:
  - `terminal_payload_after_local_completion = 4`
  - `duplicate_terminal_payload_after_local_completion = 4`
  - `payload_after_local_completion_during_settlement = 0`

Это означало, что body tail уже не ломался, ACK monotonic path уже был чистым, а residual noise оставался только в terminal payload classification.

## Root Cause

Одна причина:

completed/tombstone dispatch в [src/roles/client.rs](/C:/neinternet/vpnnode/src/roles/client.rs) классифицировал любой пустой post-completion frame с `late_bytes == 0` как terminal payload, если stream был в terminal-settlement mode.

Это было слишком широко. В residual combined-chaos traces post-completion frame был:

- пустым
- уже полностью ack-covered
- не нёс новых body bytes
- `end_of_stream = false`

То есть это был harmless duplicate empty DATA frame after local completion, а не настоящий transport terminal payload.

Проверка до фикса в [docs/artifacts/chaos_terminal_payload_tail_before_2026-03-22.stage.jsonl](/C:/neinternet/vpnnode/docs/artifacts/chaos_terminal_payload_tail_before_2026-03-22.stage.jsonl) показывала типичный residual event:

- `stage=terminal_payload_after_local_completion`
- `frame_seq=37`
- `ack_seq=38`
- `end_of_stream=false`

Это и есть точный root cause: terminal payload classifier смешивал duplicate empty DATA after completion с реальным EOS marker.

## Structural Fix

Фикс в [src/roles/client.rs](/C:/neinternet/vpnnode/src/roles/client.rs):

- `terminal_payload_after_local_completion` теперь возможен только если:
  - `frame.payload.is_empty()`
  - `late_bytes == 0`
  - `end_of_stream == true`

Иначе post-completion empty duplicate frame больше не проходит через terminal-payload anomaly path.

Это не меняет:

- replay-window policy
- ACK monotonic policy
- timeout thresholds
- route scoring

## Why This Was The Next Limiter

После предыдущего шага:

- `ack_gap_detected = 0`
- `ack_regression_ignored = 0`
- `response_transport_terminal_close_duplicate = 0`

Значит terminal payload classification действительно оставался следующим и последним failure-looking residual в terminal tail.

## Result

После фикса:

- local targeted regression: `terminal_payload_after_local_completion = 0`
- full local chaos matrix: `terminal_payload_after_local_completion = 0` для `none`, `mild-loss`, `mild-reorder`, `mild-delay`, `combined`
- remote client stage trace: `terminal_payload_after_local_completion = 0`

Подробное before/after сравнение: [docs/artifacts/chaos_terminal_payload_tail_compare_2026-03-22.md](/C:/neinternet/vpnnode/docs/artifacts/chaos_terminal_payload_tail_compare_2026-03-22.md)

Remote proof с точными IP и counters: [docs/artifacts/chaos_terminal_payload_tail_remote_evidence_2026-03-22.md](/C:/neinternet/vpnnode/docs/artifacts/chaos_terminal_payload_tail_remote_evidence_2026-03-22.md)
