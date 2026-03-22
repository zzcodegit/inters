# Stage 4 Closeout / Acceptance

Stage 4 closes the transport-semantics hardening loop for Chaos / Fault Injection. This closeout is based on one fresh freeze-quality rerun on March 22, 2026, plus the accepted fix lineage captured in [docs/artifacts/stage4_lineage_summary_2026-03-22.md](docs/artifacts/stage4_lineage_summary_2026-03-22.md).

## Freeze rerun bundle

- Readiness: [docs/artifacts/stage4_freeze_http_probe_ready_2026-03-22.log](artifacts/stage4_freeze_http_probe_ready_2026-03-22.log)
- Local baseline: [docs/artifacts/stage4_freeze_local_baseline_2026-03-22.log](artifacts/stage4_freeze_local_baseline_2026-03-22.log)
- Targeted critical regressions: [docs/artifacts/stage4_freeze_targeted_regressions_2026-03-22.log](artifacts/stage4_freeze_targeted_regressions_2026-03-22.log), [docs/artifacts/stage4_freeze_targeted_regressions_2026-03-22.md](artifacts/stage4_freeze_targeted_regressions_2026-03-22.md)
- Full local chaos matrix: [docs/artifacts/chaos_matrix_stage4_freeze_2026-03-22.log](artifacts/chaos_matrix_stage4_freeze_2026-03-22.log), [docs/artifacts/chaos_matrix_stage4_freeze_2026-03-22.md](artifacts/chaos_matrix_stage4_freeze_2026-03-22.md), [docs/artifacts/chaos_matrix_stage4_freeze_2026-03-22.summary.json](artifacts/chaos_matrix_stage4_freeze_2026-03-22.summary.json)
- Remote WAN matrix: [docs/artifacts/stage4_freeze_remote_wan_matrix_2026-03-22.log](artifacts/stage4_freeze_remote_wan_matrix_2026-03-22.log)
- Remote perf: [docs/artifacts/stage4_freeze_remote_perf_2026-03-22.log](artifacts/stage4_freeze_remote_perf_2026-03-22.log), [docs/artifacts/stage5_entry_perf_matrix_2026-03-22.md](artifacts/stage5_entry_perf_matrix_2026-03-22.md)
- Remote stage: [docs/artifacts/stage4_freeze_remote_stage_2026-03-22.log](artifacts/stage4_freeze_remote_stage_2026-03-22.log), [docs/artifacts/stage5_entry_perf_stage_2026-03-22.md](artifacts/stage5_entry_perf_stage_2026-03-22.md)
- Remote topology proof: [docs/artifacts/stage4_freeze_remote_evidence_2026-03-22.md](artifacts/stage4_freeze_remote_evidence_2026-03-22.md)
- Cleanup proof: [docs/artifacts/stage4_freeze_local_cleanup_verify_2026-03-22.log](artifacts/stage4_freeze_local_cleanup_verify_2026-03-22.log), [docs/artifacts/stage4_freeze_remote_cleanup_verify_2026-03-22.log](artifacts/stage4_freeze_remote_cleanup_verify_2026-03-22.log)

## Accepted fix set

The accepted fix sequence is:

1. Response truncation completion
2. Lifecycle race / missing response channel
3. Replay-window false rejects
4. Delay-tail false late payload
5. Duplicate-noise false open failures
6. ACK false regression
7. Terminal payload tail
8. Duplicate payload tail false anomaly

The exact commit lineage and proof artifact for each item is captured in [docs/artifacts/stage4_lineage_summary_2026-03-22.md](artifacts/stage4_lineage_summary_2026-03-22.md).

## Current invariants

Stage 4 acceptance now requires all of the following:

- `tests/http_probe_ready.rs` stays green and readiness does not accept `504` as usable.
- Local baseline stays green (`9/9`).
- The critical chaos regressions stay green:
  - truncation against `Content-Length`
  - replay-window reorder tolerance
  - delayed-tail settlement
  - duplicate packet classification
  - terminal-close settlement
  - terminal-payload tail
  - duplicate-payload tail
- Full local chaos matrix stays green on `none`, `mild-loss`, `mild-reorder`, `mild-delay`, and `combined`.
- Remote WAN matrix still proves `1-hop`, `2-hop`, and `3-hop` HTTP `200` on the real London/Warsaw/Los Angeles topology.
- Remote perf and remote stage runners stay green.

## Counters that must stay zero

These are no longer allowed to show up as active failure signatures in Stage 4 acceptance:

- `late_payload_after_completion`
- `late_close_after_completion`
- `packet too old for replay window`
- `open_message_failed: duplicate packet detected`
- `ack_gap_detected`
- `ack_regression_ignored`
- `terminal_payload_after_local_completion`
- `duplicate_terminal_payload_after_local_completion`
- `duplicate_payload_after_local_completion`
- `duplicate_payload_after_local_completion_repeat`
- `payload_after_local_completion_during_settlement`
- `MISSING response channel`

The local freeze matrix summary already shows the semantic tail counters at zero, and the targeted regressions plus remote stage trace keep the rest pinned there.

## Non-zero counters that are now explicit harmless policy

These counters can remain non-zero without blocking Stage 4 because they now represent explicit, idempotent policy rather than ambiguous transport failure:

- `duplicate_close_stream_suppressed`
- `response_transport_terminal_payload_observed`
- `response_transport_terminal_close_observed`
- `response_transport_local_completion`
- `response_transport_settlement_started`
- `response_transport_settlement_completed`
- `duplicate_payload_after_local_completion_absorbed`
- `duplicate_packet_dropped`
- `stale_ack_ignored`
- `cumulative_ack_advanced`
- `inflight_cleanup_by_ack_range`

For the freeze local chaos matrix, the combined profile ends with:

- no late events
- no open-message failures
- no replay false rejects
- `duplicate_close_stream_suppressed = 32`
- `stale_ack_ignored = 632`
- `duplicate_packet_dropped = 2` on client and `9` on exit

Those are explicit policy buckets, not unresolved semantics bugs.

## Explicitly not blockers anymore

The following are explicitly out of blocker status for Stage 4:

- delayed `CloseStream` after local completion, as long as it is absorbed by settlement
- delayed duplicate payload after local completion, as long as it is absorbed and not counted as anomaly
- stale cumulative ACK after a newer ACK floor, as long as it is counted as stale and not as regression
- duplicate ciphertext under combined chaos, as long as it is counted as `duplicate_packet_dropped` and not as open failure

## Out of scope for Stage 4

Stage 4 does not claim to solve:

- congestion control
- pacing
- adaptive congestion window discipline
- route-quality performance tuning beyond already accepted scoring stability
- residual log-noise such as `SUSPICIOUSLY_SMALL_RESPONSE`

Those belong to the Stage 5 performance/control workstream, not to transport-chaos semantics.

## Acceptance statement

Based on the March 22, 2026 freeze rerun, Stage 4 is accepted as transport-semantic hardening:

- the previously accepted chaos fixes remain green together
- the freeze matrix does not surface a remaining semantics blocker
- the remaining limiter is no longer a chaos-lifecycle correctness issue; it is a performance/control issue documented in [docs/stage5_entry_baseline.md](stage5_entry_baseline.md)
