# Chaos Truncation Compare

## Before vs After

| slice | before | after |
| --- | --- | --- |
| combined exact 3-hop body completion | failed on preserved `stream_id=6` with `23384 / 32924` bytes and `completion_reason=peer_closed` | full body delivered; combined profile passes |
| combined matrix status | matrix failed honestly on incomplete body | matrix exits green |
| combined adaptive selector | no completed adaptive decision set because combined run aborted | `5` decision events, `0` route changes, `current_still_best x5` |
| mild profiles | green | green |

## Preserved Failing Evidence

Pre-fix artifacts:

- `docs/artifacts/chaos_matrix_2026-03-22.log`
- `docs/artifacts/chaos_matrix_2026-03-22.summary.json`
- `docs/artifacts/chaos_matrix_2026-03-22_combined.stage.jsonl`

Key preserved failing facts:

- exit stream complete: `resp_bytes=32924`, `http_code=200`
- client parsed `Content-Length=32863`, so expected total `32924`
- client completed at `23384` bytes with `peer_closed`

## Post-Fix Chaos Evidence

Post-fix artifacts:

- `docs/artifacts/chaos_matrix_2026-03-22_response_fix.log`
- `docs/artifacts/chaos_matrix_2026-03-22_response_fix.md`
- `docs/artifacts/chaos_matrix_2026-03-22_response_fix.summary.json`
- `docs/artifacts/chaos_matrix_2026-03-22_response_fix_combined.md`
- `docs/artifacts/chaos_matrix_2026-03-22_response_fix_combined.stage.jsonl`
- `docs/artifacts/chaos_truncation_targeted_regression_2026-03-22.log`

Key post-fix facts:

- combined exact 3-hop: `5/5` successful runs
- combined adaptive path: `5/5` successful runs
- buffered completion now lands on `content_length_reached`
- selector still does not flap under mild or combined chaos in this local matrix sample

## Post-Fix Acceptance Rechecks

Local:

- readiness: `docs/artifacts/chaos_truncation_http_probe_ready_2026-03-22.log`
- baseline: `docs/artifacts/chaos_truncation_local_baseline_2026-03-22.log`
- client/unit coverage: `docs/artifacts/chaos_truncation_client_unit_2026-03-22.log`

Remote/WAN:

- matrix: `docs/artifacts/chaos_truncation_remote_wan_matrix_2026-03-22.log`
- perf runner: `docs/artifacts/chaos_truncation_remote_perf_2026-03-22.log`
- perf summary: `docs/artifacts/chaos_truncation_remote_perf_2026-03-22.md`
- stage runner: `docs/artifacts/chaos_truncation_remote_perf_stage_2026-03-22.log`
- stage summary: `docs/artifacts/chaos_truncation_remote_perf_stage_2026-03-22.md`

## Cleanup Proof

- local cleanup: `docs/artifacts/chaos_truncation_cleanup_2026-03-22.log`
- remote stage cleanup is embedded in `docs/artifacts/chaos_truncation_remote_perf_stage_2026-03-22.log`
