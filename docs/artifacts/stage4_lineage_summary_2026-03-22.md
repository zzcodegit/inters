# Stage 4 Fix Lineage Summary

| failure mode | closing commit | primary proof artifact | current invariant |
| --- | --- | --- | --- |
| response truncation against `Content-Length` | `aee7cc5` `Fix chaos response truncation completion` | [docs/chaos_response_truncation_rca.md](../chaos_response_truncation_rca.md) | combined chaos no longer ends with short body after `200 OK` |
| lifecycle race / `MISSING response channel` | `0f59c31` `Fix chaos response lifecycle races` | [docs/chaos_response_lifecycle_rca.md](../chaos_response_lifecycle_rca.md) | `MISSING response channel = 0` |
| replay-window false rejects under reorder | `be4666e` `Fix chaos replay window tolerance` | [docs/chaos_replay_window_rca.md](../chaos_replay_window_rca.md) | no `packet too old for replay window` under accepted chaos profiles |
| delay-tail false late payload | `e6c7087` `Fix delayed response tail settlement` | [docs/chaos_delay_tail_rca.md](../chaos_delay_tail_rca.md) | no `late_payload_after_completion` / `late_close_after_completion` |
| duplicate-noise false open failures | `294f320` `Classify chaos duplicate packets without failure noise` | [docs/chaos_duplicate_noise_rca.md](../chaos_duplicate_noise_rca.md) | duplicate packets no longer surface as `open_message_failed` |
| ACK false regression under reorder/dup | `3425b60` `Stabilize terminal ACK tail accounting under chaos` | [docs/chaos_terminal_tail_ack_gap_rca.md](../chaos_terminal_tail_ack_gap_rca.md) | `ack_regression_ignored = 0`, `ack_gap_detected = 0` |
| residual terminal payload tail | `826708f` `Fix terminal payload tail classification` | [docs/chaos_terminal_payload_tail_rca.md](../chaos_terminal_payload_tail_rca.md) | `terminal_payload_after_local_completion = 0` |
| residual duplicate payload tail | `9e88e3e` `Absorb duplicate payload tail after completion` | [docs/chaos_duplicate_payload_tail_rca.md](../chaos_duplicate_payload_tail_rca.md) | `duplicate_payload_after_local_completion = 0`, `duplicate_payload_after_local_completion_repeat = 0` |

The Stage 4 freeze rerun that validates the whole lineage together is:

- [docs/stage4_closeout_acceptance.md](../stage4_closeout_acceptance.md)
- [docs/artifacts/chaos_matrix_stage4_freeze_2026-03-22.md](chaos_matrix_stage4_freeze_2026-03-22.md)
- [docs/artifacts/stage4_freeze_targeted_regressions_2026-03-22.log](stage4_freeze_targeted_regressions_2026-03-22.log)
