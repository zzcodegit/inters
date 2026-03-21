# Residual Response-Path Follow-up

## Validation Set

Local checks:

- `docs/artifacts/residual_response_http_probe_ready_2026-03-21.log`
- `docs/artifacts/residual_response_local_baseline_2026-03-21.log`

Remote checks:

- `docs/artifacts/remote_wan_matrix_2026-03-21_residual_fix.log`
- `docs/artifacts/remote_perf_matrix_2026-03-21_residual_fix.log`
- `docs/artifacts/remote_perf_matrix_2026-03-21_residual_fix.jsonl`
- `docs/artifacts/remote_perf_matrix_2026-03-21_residual_fix.md`
- `docs/artifacts/remote_perf_stage_matrix_2026-03-21_residual_fix.log`
- `docs/artifacts/remote_perf_stage_matrix_2026-03-21_residual_fix.local.jsonl`
- `docs/artifacts/remote_perf_stage_matrix_2026-03-21_residual_fix.client.jsonl`
- `docs/artifacts/remote_perf_stage_matrix_2026-03-21_residual_fix.exit.jsonl`
- `docs/artifacts/remote_perf_stage_matrix_2026-03-21_residual_fix.direct.jsonl`
- `docs/artifacts/remote_perf_stage_matrix_2026-03-21_residual_fix.joined.jsonl`
- `docs/artifacts/remote_perf_stage_matrix_2026-03-21_residual_fix.md`

Cleanup proof:

- `docs/artifacts/residual_response_path_cleanup_2026-03-21.log`

## Status

- local readiness: pass
- local baseline: pass
- remote WAN matrix: pass
- remote perf runner: pass
- remote stage runner: pass

## Short take

The most important residual instability is gone: the old probe-stream `MISSING response channel` noise no longer appears in the reproduced perf/stage scenario.

The transport-side tuning also helped, but only where path quality was good enough for it to matter. The clearest win is on `remote-2hop`, where retransmit rate dropped to zero in the new stage sample. The weakest path in the new sample is now `remote-3hop`, and its bottleneck is better explained by very high live ACK latency than by the old lifecycle bug.

## Recommended next step

If this line of work continues, the next highest-value optimization is not another client cleanup change. It is route-quality handling:

1. prefer empirically lower-ACK-latency paths more aggressively
2. reduce time spent on poor `3-hop` routes that now show `ACK p95 > 1s`
3. keep the adaptive retransmit interval, but do not expect it to rescue a genuinely bad WAN route by itself
