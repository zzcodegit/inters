# Chaos Terminal-Tail / ACK-Gap Compare (2026-03-22)

## Targeted Combined-Chaos Regression

| signal | before | after |
| --- | ---: | ---: |
| `response_transport_terminal_close_duplicate` | 0 | 0 |
| `duplicate_close_stream_suppressed` | 32 | 32 |
| `response_transport_terminal_close_observed` | 16 | 16 |
| `terminal_close_before_local_completion_queued` | 1 | 2 |
| `terminal_payload_after_local_completion` | 3 | 1 |
| `cumulative_ack_advanced` | 48 | 40 |
| `inflight_cleanup_by_ack_range` | 48 | 40 |
| `ack_gap_detected` | 0 | 0 |
| `ack_regression_ignored` | 233 | 0 |
| `stale_ack_ignored` | 358 | 631 |
| `late_close_after_completion` | 0 | 0 |
| `duplicate_terminal_payload_after_local_completion` | 0 | 0 |

## Interpretation

- The failure-looking ACK tail moved from `ack_regression_ignored = 233` to `0`.
- Older ACKs did not disappear; they were reclassified into the explicit harmless policy bucket `stale_ack_ignored`.
- Terminal close duplication was already clean enough before this fix and stayed clean after it.
- `ack_gap_detected` stayed at `0`, so the issue was not gap cleanup. It was the classification of reordered older cumulative ACKs.

## Full Chaos Matrix After Fix

Combined profile from `docs/artifacts/chaos_matrix_2026-03-22_terminal_tail_ack_gap_fix.summary.json`:

| signal | combined after fix |
| --- | ---: |
| `duplicate_close_stream_suppressed` | 32 |
| `response_transport_terminal_close_observed` | 16 |
| `response_transport_terminal_close_duplicate` | 0 |
| `terminal_close_before_local_completion_queued` | 1 |
| `terminal_payload_after_local_completion` | 2 |
| `cumulative_ack_advanced` | 44 |
| `inflight_cleanup_by_ack_range` | 44 |
| `ack_gap_detected` | 0 |
| `ack_regression_ignored` | 0 |
| `stale_ack_ignored` | 695 |

## Evidence Files

- before log: `docs/artifacts/chaos_terminal_tail_ack_gap_before_2026-03-22.log`
- before stage trace: `docs/artifacts/chaos_terminal_tail_ack_gap_before_2026-03-22.stage.jsonl`
- after log: `docs/artifacts/chaos_terminal_tail_ack_gap_after_2026-03-22.log`
- after stage trace: `docs/artifacts/chaos_terminal_tail_ack_gap_after_2026-03-22.stage.jsonl`
- full matrix log: `docs/artifacts/chaos_matrix_2026-03-22_terminal_tail_ack_gap_fix.log`
- full matrix summary: `docs/artifacts/chaos_matrix_2026-03-22_terminal_tail_ack_gap_fix.md`
