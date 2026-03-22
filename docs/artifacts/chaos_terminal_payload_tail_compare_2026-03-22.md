# Terminal Payload Tail Before/After

## Targeted Regression

Files:

- before log: [chaos_terminal_payload_tail_before_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_terminal_payload_tail_before_2026-03-22.log)
- before raw: [chaos_terminal_payload_tail_before_2026-03-22.jsonl](/C:/neinternet/vpnnode/docs/artifacts/chaos_terminal_payload_tail_before_2026-03-22.jsonl)
- before stage: [chaos_terminal_payload_tail_before_2026-03-22.stage.jsonl](/C:/neinternet/vpnnode/docs/artifacts/chaos_terminal_payload_tail_before_2026-03-22.stage.jsonl)
- after log: [chaos_terminal_payload_tail_after_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_terminal_payload_tail_after_2026-03-22.log)
- after raw: [chaos_terminal_payload_tail_after_2026-03-22.jsonl](/C:/neinternet/vpnnode/docs/artifacts/chaos_terminal_payload_tail_after_2026-03-22.jsonl)
- after stage: [chaos_terminal_payload_tail_after_2026-03-22.stage.jsonl](/C:/neinternet/vpnnode/docs/artifacts/chaos_terminal_payload_tail_after_2026-03-22.stage.jsonl)

Counts from targeted regression:

| counter | before | after |
| --- | ---: | ---: |
| `terminal_payload_after_local_completion` | 4 | 0 |
| `duplicate_terminal_payload_after_local_completion` | 4 | 0 |
| `terminal_close_before_local_completion_queued` | 0 | 1 |
| `response_transport_terminal_payload_observed` | 16 | 16 |
| `response_transport_terminal_close_observed` | 16 | 16 |
| `duplicate_close_stream_suppressed` | 32 | 32 |
| `payload_after_local_completion_during_settlement` | 0 | 0 |

Meaning:

- terminal markers are still observed during active settlement
- duplicate close suppression still works
- extra body delivery after local completion stays at `0`
- only the residual false terminal-payload classification disappears

## Full Local Chaos Matrix

Current matrix bundle:

- log: [chaos_matrix_2026-03-22_terminal_payload_tail_fix.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_matrix_2026-03-22_terminal_payload_tail_fix.log)
- summary: [chaos_matrix_2026-03-22_terminal_payload_tail_fix.md](/C:/neinternet/vpnnode/docs/artifacts/chaos_matrix_2026-03-22_terminal_payload_tail_fix.md)
- machine summary: [chaos_matrix_2026-03-22_terminal_payload_tail_fix.summary.json](/C:/neinternet/vpnnode/docs/artifacts/chaos_matrix_2026-03-22_terminal_payload_tail_fix.summary.json)

Relevant profile totals after fix:

| profile | `terminal_payload_after_local_completion` | `duplicate_terminal_payload_after_local_completion` | `payload_after_local_completion_during_settlement` | `ack_gap_detected` | `ack_regression_ignored` |
| --- | ---: | ---: | ---: | ---: | ---: |
| `none` | 0 | 0 | 0 | 0 | 0 |
| `mild-loss` | 0 | 0 | 0 | 0 | 0 |
| `mild-reorder` | 0 | 0 | 0 | 0 | 0 |
| `mild-delay` | 0 | 0 | 0 | 0 | 0 |
| `combined` | 0 | 0 | 0 | 0 | 0 |

## Combined Profile vs Previous Step

Previous combined summary:

- [chaos_matrix_2026-03-22_terminal_tail_ack_gap_fix.summary.json](/C:/neinternet/vpnnode/docs/artifacts/chaos_matrix_2026-03-22_terminal_tail_ack_gap_fix.summary.json)

Current combined summary:

- [chaos_matrix_2026-03-22_terminal_payload_tail_fix.summary.json](/C:/neinternet/vpnnode/docs/artifacts/chaos_matrix_2026-03-22_terminal_payload_tail_fix.summary.json)

Combined counters:

| counter | previous | current |
| --- | ---: | ---: |
| `terminal_payload_after_local_completion` | 2 | 0 |
| `duplicate_terminal_payload_after_local_completion` | 0 | 0 |
| `terminal_close_before_local_completion_queued` | 1 | 1 |
| `response_transport_terminal_close_observed` | 16 | 16 |
| `response_transport_terminal_payload_observed` | 16 | 16 |
| `duplicate_close_stream_suppressed` | 32 | 32 |
| `payload_after_local_completion_during_settlement` | 0 | 0 |
| `ack_gap_detected` | 0 | 0 |
| `ack_regression_ignored` | 0 | 0 |

## Root-Cause Proof Snippet

Before fix, residual terminal-payload events came from post-completion frames with:

- `frame_seq=37`
- `ack_seq=38`
- `end_of_stream=false`

That proves the old path was classifying duplicate empty DATA as terminal payload, not absorbing a real terminal EOS.
