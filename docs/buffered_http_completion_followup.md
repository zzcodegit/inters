# Buffered HTTP Completion Follow-Up

## Question

How much of the remaining response tail was actually caused by `buffer_cap_reached` after the lifecycle fix?

Short answer: it was a real correctness and completion-contract defect, but it was not the main latency bottleneck.

## Inputs

Before:

- [`docs/artifacts/remote_perf_matrix_2026-03-21_response_fix.jsonl`](artifacts/remote_perf_matrix_2026-03-21_response_fix.jsonl)
- [`docs/artifacts/remote_perf_stage_matrix_2026-03-21_response_fix.md`](artifacts/remote_perf_stage_matrix_2026-03-21_response_fix.md)
- [`docs/artifacts/remote_perf_stage_matrix_2026-03-21_response_fix.client.jsonl`](artifacts/remote_perf_stage_matrix_2026-03-21_response_fix.client.jsonl)

After:

- [`docs/artifacts/remote_perf_matrix_2026-03-21_buffered_fix.jsonl`](artifacts/remote_perf_matrix_2026-03-21_buffered_fix.jsonl)
- [`docs/artifacts/remote_perf_stage_matrix_2026-03-21_buffered_fix.md`](artifacts/remote_perf_stage_matrix_2026-03-21_buffered_fix.md)
- [`docs/artifacts/remote_perf_stage_matrix_2026-03-21_buffered_fix.client.jsonl`](artifacts/remote_perf_stage_matrix_2026-03-21_buffered_fix.client.jsonl)
- [`docs/artifacts/remote_perf_matrix_2026-03-21_buffered_fix.client.jsonl`](artifacts/remote_perf_matrix_2026-03-21_buffered_fix.client.jsonl)

## Completion Reason Shift

Buffered perf completions after the fix:

- `21 / 21` new perf buffered completion decisions end with `content_length_reached`
- `0 / 21` end with `buffer_cap_reached`

Stage perf completions after the fix:

- `20 / 20` successful remote perf-stage runs end with `content_length_reached`
- `0 / 20` end with `buffer_cap_reached`

That is the important contract change.

## Perf Interpretation

Cross-run wall-clock numbers moved between the older and newer remote samples, including the direct path:

| scenario | avg total before ms | avg total after ms | delta ms |
| --- | ---: | ---: | ---: |
| direct | `1244.14` | `1548.86` | `+304.72` |
| remote-1hop | `6588.14` | `9195.18` | `+2607.04` |
| remote-2hop | `6824.92` | `8324.90` | `+1499.98` |
| remote-3hop | `8837.30` | `11284.50` | `+2447.21` |

I do not treat those raw before and after totals as proof that buffered completion itself slowed the dataplane, because even `direct` got slower. That means the samples include network and server variance outside this code change.

## What The Stage Breakdown Says

The more stable signal is stage composition.

Before:

- client body completion tail: `1.15 ms`
- overlay pre-first-byte gap: `7339.83 ms`
- exit/body tail: `7175.20 ms`

After:

- client body completion tail: `3.81 ms`
- overlay pre-first-byte gap: `12636.96 ms`
- exit/body tail: `12304.47 ms`

Interpretation:

- buffered completion tail stays tiny in both samples
- the dominant cost remains earlier in the overlay response path
- waiting for the exact `Content-Length` did not become a new large tail source

## Conclusion

`buffer_cap_reached` was a real bug and a bad acceptance signal because it let buffered HTTP complete for the wrong reason.

But it was not the main reason WAN total time stays high.

The main bottlenecks remain:

1. `overlay pre-first-byte gap`
2. `exit/body delivery tail`
3. retransmit and ACK behavior on the response path

So the right takeaway is:

- keep this buffered completion fix because it restores protocol-correct completion semantics
- do not expect it alone to solve the remaining WAN perf tail
- continue optimization work on response-path delivery, ACK, and retransmit behavior
