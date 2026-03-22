# Chaos Duplicate Noise RCA

## Symptom

After the replay-window and delay-tail fixes, the remaining failure-looking signature in the `combined` chaos profile was no longer truncation or late cleanup. The dominant residual signal was duplicate traffic surfacing as generic transport failure:

- client `open_message_failed: duplicate packet detected` = `4`
- exit `open_message_failed: duplicate packet detected` = `6`
- client `response_transport_terminal_close_duplicate` = `32`

Those counts are visible in the pre-fix combined artifacts:

- `docs/artifacts/chaos_matrix_2026-03-22_delay_tail_fix.summary.json`
- `docs/artifacts/chaos_matrix_2026-03-22_delay_tail_fix_combined.stage.jsonl`

The important point is that the network was still delivering complete `200 OK` responses. What remained was duplicate tail traffic being accounted as if it were a decode/open failure candidate.

## Trigger Profile

The issue reproduced under deterministic local `combined` chaos only:

- `loss_ppm=1000`
- `duplicate_ppm=2000`
- `reorder_ppm=15000`
- `base_delay_ms=8`
- `jitter_ms=6`
- `reorder_extra_delay_ms=30`

This is the same profile used by:

- `combined_chaos_duplicate_packets_are_classified_without_open_failure`
- `docs/artifacts/chaos_matrix_2026-03-22_duplicate_noise_fix_combined.profile.json`

## Root Cause

One concrete root cause:

`SessionCrypto::open_message()` already knew when an incoming packet was a replay-window duplicate, but client and exit receive loops were routing that duplicate-reject through the same generic `open_message_failed` path used for true transport/decode failures.

That made expected duplicate ciphertext under `combined` chaos look like the next transport failure mode, even though the correct policy was “drop as duplicate and continue”.

Code location before the fix:

- client UDP receive path in `src/roles/client.rs`
- exit UDP receive path in `src/roles/exit.rs`

The duplicate terminal close markers themselves were not the broken part. They were already reaching the settlement path as idempotent terminal observations. The misleading part was the duplicate ciphertext reject path being reported as a transport failure.

## Structural Fix

The fix is classification, not log suppression:

1. `src/session.rs` now exposes typed duplicate classification through:
   - `SessionOpenRejectKind`
   - `SessionOpenRejectInfo`
   - `classify_open_message_error(...)`
2. `src/roles/client.rs` and `src/roles/exit.rs` now treat `Duplicate` rejects as explicit duplicate-drop policy:
   - emit `duplicate_packet_dropped`
   - keep `open_message_failed` for non-duplicate failures only
   - continue the receive loop without touching response lifecycle
3. `tests/chaos_matrix.rs` now enforces this invariant:
   - duplicate packets must not reappear as `open_message_failed`
   - duplicate traffic still has to be observed as `duplicate_packet_dropped`

This is a policy fix on the receive/classification path. It does not change replay-window sizing, timeouts, route scoring, or terminal-settlement semantics.

## Result

Post-fix `combined` chaos still injects duplicate traffic:

- `chaos_duplicate = 12`

But duplicate traffic is no longer accounted as a failure:

- client `open_message_failed: duplicate packet detected` = `0`
- exit `open_message_failed: duplicate packet detected` = `0`
- client `duplicate_packet_dropped` = `5`
- exit `duplicate_packet_dropped` = `7`
- client `response_transport_terminal_close_duplicate` = `32`

That means duplicate terminal tail is still observable, but it is now isolated as idempotent terminal noise plus explicit duplicate-drop policy, not a transport failure candidate.

## Evidence

- local matrix after fix: `docs/artifacts/chaos_matrix_2026-03-22_duplicate_noise_fix.md`
- local matrix raw summary: `docs/artifacts/chaos_matrix_2026-03-22_duplicate_noise_fix.summary.json`
- targeted regression log: `docs/artifacts/chaos_duplicate_noise_targeted_regression_2026-03-22.log`
- targeted regression stage trace: `docs/artifacts/chaos_duplicate_noise_targeted_regression_2026-03-22.stage.jsonl`
- before/after compare: `docs/artifacts/chaos_duplicate_noise_compare_2026-03-22.md`
