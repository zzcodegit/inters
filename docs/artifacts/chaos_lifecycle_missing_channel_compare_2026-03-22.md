# Missing Response Channel Compare

## Counts

| scope | before | after |
| --- | ---: | ---: |
| targeted mixed-chaos regression | 252 | 0 |
| full local chaos matrix | 1368 | 0 |
| remote WAN matrix log | n/a | 0 |
| remote perf runner log | n/a | 0 |
| remote stage runner log | n/a | 0 |

## Sources

Before:

- `docs/artifacts/chaos_truncation_targeted_regression_2026-03-22.log`
- `docs/artifacts/chaos_matrix_2026-03-22_response_fix.log`

After:

- `docs/artifacts/chaos_lifecycle_targeted_regression_2026-03-22.log`
- `docs/artifacts/chaos_matrix_2026-03-22_lifecycle_fix.log`
- `docs/artifacts/chaos_lifecycle_remote_wan_matrix_2026-03-22.log`
- `docs/artifacts/chaos_lifecycle_remote_perf_2026-03-22.log`
- `docs/artifacts/chaos_lifecycle_remote_perf_stage_2026-03-22.log`

## Stage-Trace Follow-Up

Combined chaos after the fix:

- `orphaned_response_payload = 0`
- `response_payload_unknown_stream = 0`
- `late_ack_after_completion = 169`
- `stale_active_response_state_pruned = 0`

This is the intended shape: late ACKs still exist under chaos, but they are handled against completed-stream tombstones instead of recreating broken active stream state.
