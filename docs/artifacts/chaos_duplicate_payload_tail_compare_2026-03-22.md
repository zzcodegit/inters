# Duplicate Payload Tail Before/After

## Before

Targeted regression before-fix:

- [chaos_duplicate_payload_tail_before_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_duplicate_payload_tail_before_2026-03-22.log)
- [chaos_duplicate_payload_tail_before_2026-03-22.jsonl](/C:/neinternet/vpnnode/docs/artifacts/chaos_duplicate_payload_tail_before_2026-03-22.jsonl)
- [chaos_duplicate_payload_tail_before_2026-03-22.stage.jsonl](/C:/neinternet/vpnnode/docs/artifacts/chaos_duplicate_payload_tail_before_2026-03-22.stage.jsonl)

Failure signature:

| counter | before |
| --- | ---: |
| `duplicate_payload_after_local_completion` | 9 |
| `duplicate_payload_after_local_completion_repeat` | 713 |
| `duplicate_payload_after_local_completion_absorbed` | 0 |
| `payload_after_local_completion_during_settlement` | 0 |
| `response_transport_local_completion` | 16 |
| `response_transport_settlement_completed` | 16 |
| `duplicate_close_stream_suppressed` | 32 |

Representative before events were already fully ACK-covered:

- `frame_seq=20`, `ack_seq=38`
- `frame_seq=16`, `ack_seq=38`
- `frame_seq=17`, `ack_seq=38`

## After

Targeted regression after-fix:

- [chaos_duplicate_payload_tail_after_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_duplicate_payload_tail_after_2026-03-22.log)
- [chaos_duplicate_payload_tail_after_2026-03-22.jsonl](/C:/neinternet/vpnnode/docs/artifacts/chaos_duplicate_payload_tail_after_2026-03-22.jsonl)
- [chaos_duplicate_payload_tail_after_2026-03-22.stage.jsonl](/C:/neinternet/vpnnode/docs/artifacts/chaos_duplicate_payload_tail_after_2026-03-22.stage.jsonl)

After-fix counts:

| counter | after |
| --- | ---: |
| `duplicate_payload_after_local_completion` | 0 |
| `duplicate_payload_after_local_completion_repeat` | 0 |
| `duplicate_payload_after_local_completion_absorbed` | 659 |
| `payload_after_local_completion_during_settlement` | 0 |
| `response_transport_local_completion` | 16 |
| `response_transport_settlement_completed` | 16 |
| `duplicate_close_stream_suppressed` | 32 |

Representative after events keep the same ACK-covered shape, but now go through explicit absorbtion policy:

- `frame_seq=4`, `ack_seq=38`
- `frame_seq=5`, `ack_seq=38`
- `frame_seq=7`, `ack_seq=38`
- stage: `duplicate_payload_after_local_completion_absorbed`
- `dispatch_state=completed_tombstone`
- `ack_covered=true`

## Full Local Chaos Matrix

Current matrix:

- [chaos_matrix_2026-03-22_duplicate_payload_tail_fix.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_matrix_2026-03-22_duplicate_payload_tail_fix.log)
- [chaos_matrix_2026-03-22_duplicate_payload_tail_fix.md](/C:/neinternet/vpnnode/docs/artifacts/chaos_matrix_2026-03-22_duplicate_payload_tail_fix.md)
- [chaos_matrix_2026-03-22_duplicate_payload_tail_fix.summary.json](/C:/neinternet/vpnnode/docs/artifacts/chaos_matrix_2026-03-22_duplicate_payload_tail_fix.summary.json)

Current local profile totals:

| profile | `duplicate_payload_after_local_completion` | `duplicate_payload_after_local_completion_repeat` | `payload_after_local_completion_during_settlement` | `ack_gap_detected` | `ack_regression_ignored` |
| --- | ---: | ---: | ---: | ---: | ---: |
| `none` | 0 | 0 | 0 | 0 | 0 |
| `mild-loss` | 0 | 0 | 0 | 0 | 0 |
| `mild-reorder` | 0 | 0 | 0 | 0 | 0 |
| `mild-delay` | 0 | 0 | 0 | 0 | 0 |
| `combined` | 0 | 0 | 0 | 0 | 0 |

## Remote Before/After

Before remote client trace:

- [chaos_terminal_payload_tail_remote_perf_stage_2026-03-22.client.jsonl](/C:/neinternet/vpnnode/docs/artifacts/chaos_terminal_payload_tail_remote_perf_stage_2026-03-22.client.jsonl)
  - `duplicate_payload_after_local_completion = 1`
  - `duplicate_payload_after_local_completion_repeat = 3`

After remote client trace:

- [chaos_duplicate_payload_tail_remote_perf_stage_2026-03-22.client.jsonl](/C:/neinternet/vpnnode/docs/artifacts/chaos_duplicate_payload_tail_remote_perf_stage_2026-03-22.client.jsonl)
  - `duplicate_payload_after_local_completion = 0`
  - `duplicate_payload_after_local_completion_repeat = 0`
  - `payload_after_local_completion_during_settlement = 0`
