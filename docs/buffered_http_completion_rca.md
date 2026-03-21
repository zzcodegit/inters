# Buffered HTTP Completion RCA

## Goal

Explain why the client buffered HTTP path was frequently completing on `buffer_cap_reached` instead of `Content-Length`, fix that behavior structurally, and check whether it is a meaningful contributor to the remaining response tail.

## Buffered Path Scope

The buffered response path is active in [`src/roles/client.rs`](../src/roles/client.rs) when `write_response_to_tcp == false`.

That is the normal plain-HTTP path for:

- local baseline probe and request scenarios
- remote WAN smoke and matrix probes
- remote perf and remote perf-stage scenarios

The TLS / streaming path (`write_response_to_tcp == true`) is intentionally not changed here.

## Root Cause

The old client logic did the checks in the wrong order:

1. Append the next chunk into `full`.
2. Check a soft cap of `request.len() + 256 KiB`.
3. Only after that, try to finish by `Content-Length`.

For the perf payload this made the cap part of the normal path:

- request size: about `82` bytes
- old soft cap: `262226` bytes
- actual full HTTP response size: `262349` bytes

So the cap was smaller than the correct full response. `buffer_cap_reached` became a normal completion reason even though the response had a valid `Content-Length`.

## Structural Fix

In [`src/roles/client.rs`](../src/roles/client.rs):

- added a pure helper `inspect_buffered_http_response()`
- completion now prefers `Content-Length` before any cap-based stop
- when `Content-Length` is present, the active cap expands to `max(request_len + 256 KiB, expected_total + 4 KiB)`
- added a hard safety cap of `16 MiB`
- `buffer_cap_reached` stays as a safety fallback, not a normal success path

New buffered diagnostics:

- `buffered_headers_parsed`
- `buffered_content_length_detected`
- `buffered_completion_decision`

They log:

- `stream_id`
- `route_len`
- `route_chain`
- `response_bytes_total`
- `header_end`
- `content_length`
- `expected_total`
- `fallback_cap`
- `active_cap`
- `hard_cap`
- `completion_reason`

## Regression Coverage

Added unit and regression coverage in [`src/roles/client.rs`](../src/roles/client.rs):

- small response with `Content-Length`
- medium response with `Content-Length`
- no premature stop at the old soft cap before the `Content-Length` total
- intentional cap fallback for an oversized response

## Reproduction Before vs After

Before-fix raw client stage log:

- [`docs/artifacts/remote_perf_stage_matrix_2026-03-21_response_fix.client.jsonl`](artifacts/remote_perf_stage_matrix_2026-03-21_response_fix.client.jsonl)

After-fix raw client stage log:

- [`docs/artifacts/remote_perf_stage_matrix_2026-03-21_buffered_fix.client.jsonl`](artifacts/remote_perf_stage_matrix_2026-03-21_buffered_fix.client.jsonl)

Observed completion reason counts in stage client traces:

| sample | buffered completion by content length | buffered completion by cap | timeout |
| --- | ---: | ---: | ---: |
| before | `10` (`content_length_satisfied`) | `81` | `8` |
| after | `20` (`content_length_reached`) | `0` | `1` |

Observed completion reason counts in the new perf client trace:

- [`docs/artifacts/remote_perf_matrix_2026-03-21_buffered_fix.client.jsonl`](artifacts/remote_perf_matrix_2026-03-21_buffered_fix.client.jsonl)
- `21` buffered completion decisions
- `21 / 21` completed by `content_length_reached`
- `0 / 21` completed by `buffer_cap_reached`

## Validation

Local:

- readiness log: [`docs/artifacts/buffered_completion_http_probe_ready_2026-03-21.log`](artifacts/buffered_completion_http_probe_ready_2026-03-21.log)
- baseline log: [`docs/artifacts/buffered_completion_local_baseline_2026-03-21.log`](artifacts/buffered_completion_local_baseline_2026-03-21.log)

Remote:

- WAN matrix log: [`docs/artifacts/remote_wan_matrix_2026-03-21_buffered_fix.log`](artifacts/remote_wan_matrix_2026-03-21_buffered_fix.log)
- WAN matrix client trace: [`docs/artifacts/remote_wan_matrix_2026-03-21_buffered_fix.client.jsonl`](artifacts/remote_wan_matrix_2026-03-21_buffered_fix.client.jsonl)
- perf log: [`docs/artifacts/remote_perf_matrix_2026-03-21_buffered_fix.log`](artifacts/remote_perf_matrix_2026-03-21_buffered_fix.log)
- perf raw: [`docs/artifacts/remote_perf_matrix_2026-03-21_buffered_fix.jsonl`](artifacts/remote_perf_matrix_2026-03-21_buffered_fix.jsonl)
- perf stage log: [`docs/artifacts/remote_perf_stage_matrix_2026-03-21_buffered_fix.log`](artifacts/remote_perf_stage_matrix_2026-03-21_buffered_fix.log)
- perf stage summary: [`docs/artifacts/remote_perf_stage_matrix_2026-03-21_buffered_fix.md`](artifacts/remote_perf_stage_matrix_2026-03-21_buffered_fix.md)

## What This Fix Did And Did Not Change

What it fixed:

- `buffer_cap_reached` stopped being the normal completion reason for valid `Content-Length` perf responses
- buffered completion is now protocol-driven when the response tells us the exact size
- diagnostics now show exactly why completion happened

What it did not show:

- this fix is not the main reason WAN total time stays high

Why:

- in the new stage summary, client body completion tail is only `3.81 ms`
- target connect stays tiny at about `1.33 ms`
- the dominant cost still sits in `overlay pre-first-byte gap` and `exit/body tail`

See the follow-up:

- [`docs/buffered_http_completion_followup.md`](buffered_http_completion_followup.md)

## Residual Risk

The stage-runner console log still contains `2` lines of:

- `client: MISSING response channel for response payloads stream_id=1000000000`
- artifact: [`docs/artifacts/remote_perf_stage_matrix_2026-03-21_buffered_fix.log`](artifacts/remote_perf_stage_matrix_2026-03-21_buffered_fix.log)

However, the corresponding client stage trace does not show a buffered-path regression:

- perf responses complete via `content_length_reached`
- the only probe timeout in the trace is recorded as `response_idle_timeout` with later `late_payload_after_completion`

So the buffered completion fix did not reproduce the old lifecycle race in the new raw stage traces, but the console-only anomaly remains a separate residual risk.
