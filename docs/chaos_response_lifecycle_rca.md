# Chaos Response Lifecycle RCA

## Symptom

Under local mixed chaos, the client could log:

- `client: MISSING response channel for response payloads stream_id=...`

even when the request still completed successfully.

This was most visible in:

- `docs/artifacts/chaos_truncation_targeted_regression_2026-03-22.log`
- `docs/artifacts/chaos_matrix_2026-03-22_response_fix.log`

## Root Cause

The missing-channel symptom came from a real lifecycle race, not from harmless logging noise.

Two mechanisms could create the same bad state:

1. Non-atomic response-state lookup

   The UDP receive path looked up:

   - `response_senders`
   - `reliable_streams`
   - `completed_response_streams`

   separately.

   Cleanup stores a completed tombstone first and then removes live maps. Because the receive path only consulted the completed tombstone when both the live channel and the live reliable state were already absent, it could still hit an inconsistent read such as:

   - `response_senders`: already removed
   - `reliable_streams`: still present or seen from a stale snapshot

   That path fell into "active reliable state but no consumer" and emitted `MISSING response channel`.

2. Late ACK recreated active stream state after completion

   The client `Ping/Ack` path previously did:

   - `reliable_streams.entry(stream_id).or_insert(new ReliableStream())`

   even after a response stream had already completed and been cleaned up.

   Under duplicate/reordered traffic, a late ACK could therefore recreate an empty active `ReliableStream` for a completed stream. A later late payload then saw:

   - `active_rs = Some(...)`
   - `response channel = None`
   - completed tombstone ignored because `active_rs` existed

   and again emitted `MISSING response channel`.

## Structural Fix

The client now uses a stricter response lifecycle model:

- `Payload`
- `EndOfStream`
- `CloseStream`
- completed tombstone / lingering state

The important behavioral changes are:

1. Response payload dispatch now prioritizes the completed tombstone whenever the live response channel is absent.

2. Late ACKs are no longer allowed to recreate active response state.

   Instead the client now routes ACKs to:

   - the live stream state, if a live consumer exists
   - the completed tombstone reliable state, if the stream already completed
   - otherwise the ACK is handled as an unknown late control packet without creating new active state

3. Any stale active response state found without a live consumer is pruned explicitly.

4. Late payloads are either:

   - delivered to the live consumer
   - processed against the completed tombstone and recorded as `late_payload_after_completion`
   - or dropped through an explicit handled lifecycle path, not through an error

## What Changed in Practice

Before the fix:

- targeted mixed-chaos regression: `252` occurrences of `MISSING response channel`
- full local chaos matrix: `1368` occurrences

After the fix:

- targeted mixed-chaos regression: `0`
- full local chaos matrix: `0`
- remote WAN matrix / perf / stage logs: `0`

See:

- `docs/artifacts/chaos_lifecycle_missing_channel_compare_2026-03-22.md`

## Why This Is a Root-Cause Fix

The fix does not hide the log and does not ignore late traffic blindly.

Instead it removes the invalid state transition:

- a response stream is no longer allowed to re-enter "active reliable state without consumer"

Late traffic is now accounted for explicitly:

- `late_payload_after_completion` remains as the expected path for duplicated/reordered payloads after stream completion
- `late_ack_after_completion` now captures late ACKs against the completed tombstone

In the new combined chaos stage trace:

- `orphaned_response_payload = 0`
- `response_payload_unknown_stream = 0`
- `late_ack_after_completion > 0`

This is the expected shape: late control/data still happens under chaos, but it is routed through a valid completed-stream lifecycle instead of the old broken state.

## Post-Fix Evidence

Local:

- `docs/artifacts/chaos_lifecycle_client_unit_2026-03-22.log`
- `docs/artifacts/chaos_lifecycle_targeted_regression_2026-03-22.log`
- `docs/artifacts/chaos_matrix_2026-03-22_lifecycle_fix.log`
- `docs/artifacts/chaos_matrix_2026-03-22_lifecycle_fix.summary.json`

Remote:

- `docs/artifacts/chaos_lifecycle_remote_wan_matrix_2026-03-22.log`
- `docs/artifacts/chaos_lifecycle_remote_perf_2026-03-22.log`
- `docs/artifacts/chaos_lifecycle_remote_perf_stage_2026-03-22.log`

Cleanup:

- `docs/artifacts/chaos_lifecycle_cleanup_2026-03-22.log`
- remote stage cleanup is embedded in `docs/artifacts/chaos_lifecycle_remote_perf_stage_2026-03-22.log`
