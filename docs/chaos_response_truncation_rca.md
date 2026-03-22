# Chaos Response Truncation RCA

## Symptom

Under the mixed local chaos profile (`loss + reorder + duplicate + delay`) the client could return `HTTP/1.1 200 OK`, but the buffered body ended early relative to `Content-Length`.

The failing pre-fix example is preserved in:

- `docs/artifacts/chaos_matrix_2026-03-22_combined.stage.jsonl`
- `docs/artifacts/chaos_matrix_2026-03-22.log`

For `stream_id=6` on `chaos-exact-3hop-run5`:

- exit completed with `http_code=200` and `resp_bytes=32924`
- client parsed `Content-Length=32863`, so expected total bytes were `32924`
- client still completed with `completion_reason=peer_closed`
- delivered bytes were only `23384`

That is the exact truncation that made the original combined chaos run fail.

## Root Cause

The truncation was not caused by `Content-Length` parsing and not by the earlier buffered-cap heuristic.

The actual failure mode was:

1. Exit sent response data and then sent `CloseStream`.
2. Under mixed chaos, `CloseStream` could reach the client before all response payload frames had been delivered.
3. The buffered client path treated that close signal as authoritative response completion even when `Content-Length` had already been parsed and the body was still incomplete.
4. The TCP roundtrip returned a syntactically valid `200 OK` response whose body stopped early.

Why `200` was already visible:

- the status line and headers arrived early enough to parse
- the failure happened later, during buffered body completion

Replay-window and duplicate rejects were present in the same runs, but for the preserved failing stream they were noise around the transport. The truncation itself came from premature completion on `CloseStream`.

## Structural Fix

The client response channel now distinguishes three cases explicitly:

- `Payload(Vec<u8>)`
- `EndOfStream`
- `CloseStream`

Buffered HTTP completion changed from "any close ends the response" to:

- if `Content-Length` is known and the buffered body is still short, `CloseStream` is treated as an early close signal and the client keeps waiting
- completion remains on a protocol-consistent condition:
  - `content_length_reached`
  - `peer_closed` only when length is unknown or already satisfied
  - existing safety fallbacks still apply for oversized/unbounded responses

The same change also deduplicates close-feedback application per stream, so repeated `CloseStream` packets under duplication do not repeatedly perturb route-quality feedback.

## Regression Coverage

Added/updated coverage in `src/roles/client.rs` and `tests/chaos_matrix.rs`:

- buffered close waits for remaining `Content-Length` bytes
- buffered close without `Content-Length` still finishes promptly
- close-feedback is applied once per stream
- `combined_chaos_preserves_full_body_against_content_length`

Focused regression log:

- `docs/artifacts/chaos_truncation_targeted_regression_2026-03-22.log`

Client unit log:

- `docs/artifacts/chaos_truncation_client_unit_2026-03-22.log`

## After-Fix Result

Post-fix combined chaos evidence is preserved in:

- `docs/artifacts/chaos_matrix_2026-03-22_response_fix.log`
- `docs/artifacts/chaos_matrix_2026-03-22_response_fix.summary.json`
- `docs/artifacts/chaos_matrix_2026-03-22_response_fix_combined.md`
- `docs/artifacts/chaos_matrix_2026-03-22_response_fix_combined.stage.jsonl`

Observed result:

- mild profiles remain green
- combined profile no longer truncates the buffered body
- combined exact 3-hop runs now complete by `content_length_reached`

## Residual Risk Kept Explicit

Local mixed-chaos logs still show `client: MISSING response channel for response payloads`.

For this stage I am treating that as a separate residual lifecycle/noise issue, not as the truncation root cause, because:

- the new combined regression passes
- the full mixed-chaos matrix passes
- remote WAN matrix, perf runner, and stage runner remain green after the truncation fix

That residual should be handled as a follow-up transport-core cleanup item, but it is no longer able to produce the preserved `200 + truncated body` failure that blocked this chaos stage.
