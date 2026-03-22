# Remote Evidence: Duplicate Payload Tail Fix

## Exact Topology

- exit: `31.192.232.26:30000`
- relay-1: `45.197.133.115:30001`
- relay-2: `185.144.28.95:30002`

## Route Proof

WAN matrix log:

- [chaos_duplicate_payload_tail_remote_wan_matrix_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_duplicate_payload_tail_remote_wan_matrix_2026-03-22.log)

Observed routes and statuses:

- `1-hop`: `client -> 31.192.232.26:30000 -> target`, `observed_status=200`, `duration_ms=2414`
- `2-hop`: `client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`, `observed_status=200`, `duration_ms=5563`
- `3-hop`: `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`, `observed_status=200`, `duration_ms=1973`

Perf runner log:

- [chaos_duplicate_payload_tail_remote_perf_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_duplicate_payload_tail_remote_perf_2026-03-22.log)

That log shows the same route chains and `success_count=5 error_count=0` for `remote-1hop`, `remote-2hop`, `remote-3hop`.

## Exit Peer Proof

Exit stage trace:

- [chaos_duplicate_payload_tail_remote_perf_stage_2026-03-22.exit.jsonl](/C:/neinternet/vpnnode/docs/artifacts/chaos_duplicate_payload_tail_remote_perf_stage_2026-03-22.exit.jsonl)

Observed peers:

- route length `1`: `peer=Udp://46.242.13.60:8347`
- route length `2`: `peer=Udp://45.197.133.115:30001`
- route length `3`: `peer=Udp://185.144.28.95:30002`

That proves the exit saw:

- direct client ingress on `1-hop`
- relay-1 as upstream on `2-hop`
- relay-2 as upstream on `3-hop`

## Client Counters After Fix

Client stage trace:

- [chaos_duplicate_payload_tail_remote_perf_stage_2026-03-22.client.jsonl](/C:/neinternet/vpnnode/docs/artifacts/chaos_duplicate_payload_tail_remote_perf_stage_2026-03-22.client.jsonl)

Relevant counters:

| counter | value |
| --- | ---: |
| `duplicate_payload_after_local_completion` | 0 |
| `duplicate_payload_after_local_completion_repeat` | 0 |
| `duplicate_payload_after_local_completion_absorbed` | 0 |
| `payload_after_local_completion_during_settlement` | 0 |
| `response_transport_local_completion` | 21 |
| `response_transport_settlement_completed` | 18 |
| `duplicate_close_stream_suppressed` | 36 |
| `response_transport_terminal_payload_observed` | 21 |
| `response_transport_terminal_close_observed` | 18 |

Interpretation:

- remote run still exercises buffered local completion and settlement
- duplicate close suppression still happens
- old duplicate payload tail anomaly path is now `0`
- this particular live WAN sample did not need the absorbtion bucket; the local targeted regression already proves the bucket is used when duplicate covered DATA actually arrives after cleanup

## Retest Files

- perf setup: [chaos_duplicate_payload_tail_remote_perf_setup_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_duplicate_payload_tail_remote_perf_setup_2026-03-22.log)
- perf report: [chaos_duplicate_payload_tail_remote_perf_2026-03-22.md](/C:/neinternet/vpnnode/docs/artifacts/chaos_duplicate_payload_tail_remote_perf_2026-03-22.md)
- stage setup: [chaos_duplicate_payload_tail_remote_perf_stage_setup_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_duplicate_payload_tail_remote_perf_stage_setup_2026-03-22.log)
- stage collect: [chaos_duplicate_payload_tail_remote_perf_stage_collect_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_duplicate_payload_tail_remote_perf_stage_collect_2026-03-22.log)
- stage summarize: [chaos_duplicate_payload_tail_remote_perf_stage_summarize_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_duplicate_payload_tail_remote_perf_stage_summarize_2026-03-22.log)
- stage summary: [chaos_duplicate_payload_tail_remote_perf_stage_2026-03-22.md](/C:/neinternet/vpnnode/docs/artifacts/chaos_duplicate_payload_tail_remote_perf_stage_2026-03-22.md)

## Cleanup Proof

- remote cleanup: [chaos_duplicate_payload_tail_remote_cleanup_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_duplicate_payload_tail_remote_cleanup_2026-03-22.log)
- remote cleanup verify: [chaos_duplicate_payload_tail_remote_cleanup_verify_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_duplicate_payload_tail_remote_cleanup_verify_2026-03-22.log)

Verified end state:

- `exit_service=active`
- `public_service=inactive`
