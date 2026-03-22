# Remote Evidence: Terminal Payload Tail Fix

## Exact Topology

- exit: `31.192.232.26:30000`
- relay-1: `45.197.133.115:30001`
- relay-2: `185.144.28.95:30002`

## Route Proof

WAN matrix log:

- [chaos_terminal_payload_tail_remote_wan_matrix_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_terminal_payload_tail_remote_wan_matrix_2026-03-22.log)

Observed chains in that log:

- `1-hop`: `client -> 31.192.232.26:30000 -> target`, `observed_status=200`, `duration_ms=1285`
- `2-hop`: `client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`, `observed_status=200`, `duration_ms=1399`
- `3-hop`: `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`, `observed_status=200`, `duration_ms=8402`

Perf matrix log:

- [chaos_terminal_payload_tail_remote_perf_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_terminal_payload_tail_remote_perf_2026-03-22.log)

That log shows the same route chains for `remote-1hop`, `remote-2hop`, `remote-3hop`, each with `success_count=5 error_count=0`.

## Exit Peer Proof

Exit stage trace:

- [chaos_terminal_payload_tail_remote_perf_stage_2026-03-22.exit.jsonl](/C:/neinternet/vpnnode/docs/artifacts/chaos_terminal_payload_tail_remote_perf_stage_2026-03-22.exit.jsonl)

Observed peers from `stream_complete`:

- route length `1`: `peer=Udp://46.242.13.60:2070`
- route length `2`: `peer=Udp://45.197.133.115:30001`
- route length `3`: `peer=Udp://185.144.28.95:30002`

That proves the exit really saw:

- direct client ingress on `1-hop`
- relay-1 as upstream on `2-hop`
- relay-2 as upstream on `3-hop`

## Client Counters After Fix

Client stage trace:

- [chaos_terminal_payload_tail_remote_perf_stage_2026-03-22.client.jsonl](/C:/neinternet/vpnnode/docs/artifacts/chaos_terminal_payload_tail_remote_perf_stage_2026-03-22.client.jsonl)

Relevant counts after fix:

| counter | value |
| --- | ---: |
| `terminal_payload_after_local_completion` | 0 |
| `duplicate_terminal_payload_after_local_completion` | 0 |
| `payload_after_local_completion_during_settlement` | 0 |
| `response_transport_terminal_payload_observed` | 19 |
| `response_transport_terminal_close_observed` | 18 |
| `duplicate_close_stream_suppressed` | 36 |
| `response_transport_local_completion` | 21 |
| `response_transport_settlement_completed` | 18 |

Meaning:

- terminal payload markers still exist on live remote traffic
- close duplicates still arrive under real WAN conditions
- the residual post-local-completion terminal payload path is now `0`

## Remote Retest Files

- perf setup: [chaos_terminal_payload_tail_remote_perf_setup_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_terminal_payload_tail_remote_perf_setup_2026-03-22.log)
- perf report: [chaos_terminal_payload_tail_remote_perf_2026-03-22.md](/C:/neinternet/vpnnode/docs/artifacts/chaos_terminal_payload_tail_remote_perf_2026-03-22.md)
- stage setup: [chaos_terminal_payload_tail_remote_perf_stage_setup_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_terminal_payload_tail_remote_perf_stage_setup_2026-03-22.log)
- stage collect: [chaos_terminal_payload_tail_remote_perf_stage_collect_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_terminal_payload_tail_remote_perf_stage_collect_2026-03-22.log)
- stage summarize: [chaos_terminal_payload_tail_remote_perf_stage_summarize_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_terminal_payload_tail_remote_perf_stage_summarize_2026-03-22.log)
- stage summary: [chaos_terminal_payload_tail_remote_perf_stage_2026-03-22.md](/C:/neinternet/vpnnode/docs/artifacts/chaos_terminal_payload_tail_remote_perf_stage_2026-03-22.md)

## Cleanup Proof

- remote cleanup: [chaos_terminal_payload_tail_remote_cleanup_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_terminal_payload_tail_remote_cleanup_2026-03-22.log)
- remote cleanup verify: [chaos_terminal_payload_tail_remote_cleanup_verify_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_terminal_payload_tail_remote_cleanup_verify_2026-03-22.log)

Verified end state:

- `exit_service=active`
- `public_service=inactive`
