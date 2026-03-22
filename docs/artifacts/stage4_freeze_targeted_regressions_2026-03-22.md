# Stage 4 Freeze Targeted Regressions

Raw log: [docs/artifacts/stage4_freeze_targeted_regressions_2026-03-22.log](stage4_freeze_targeted_regressions_2026-03-22.log)

Critical targeted regressions covered by the freeze rerun:

| regression | status |
| --- | --- |
| `completed_response_stream_retains_ack_state_for_late_duplicates` | pass |
| `response_data_dispatch_prefers_completed_stream_over_stale_active_state` | pass |
| `late_ack_after_completion_uses_completed_stream_state` | pass |
| `orphaned_active_state_without_consumer_is_pruned` | pass |
| `combined_chaos_preserves_full_body_against_content_length` | pass |
| `reorder_chaos_does_not_reject_packets_inside_replay_window` | pass |
| `mild_delay_content_length_completion_settles_terminal_signals_cleanly` | pass |
| `combined_chaos_duplicate_packets_are_classified_without_open_failure` | pass after refreshing one stale test assertion to the already-accepted `duplicate_close_stream_suppressed` policy |
| `combined_chaos_terminal_close_duplicates_are_suppressed_after_first_signal` | pass |
| `combined_chaos_terminal_payload_tail_is_absorbed_before_completed_tombstone` | pass |
| `combined_chaos_duplicate_payload_tail_is_absorbed_before_completed_tombstone` | pass |

This summary exists so that Stage 4 closeout can point to one human-readable green set while still preserving the raw rerun log.
