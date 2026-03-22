# Remote Evidence: Terminal-Tail / ACK-Gap Fix (2026-03-22)

## Exact WAN Topology

- relay-1: `45.197.133.115:30001`
- relay-2: `185.144.28.95:30002`
- exit: `31.192.232.26:30000`
- direct public HTTP target on exit host: `31.192.232.26:18080`

## Route Proof From Remote Logs

WAN matrix logs:

- `docs/artifacts/chaos_terminal_tail_ack_gap_remote_wan_matrix_2026-03-22.log`
  - `remote_scenario=1hop ... route_chain=client -> 31.192.232.26:30000 -> target`
  - `remote_scenario=1hop observed_status=200 ...`
  - `remote_scenario=2hop ... route_chain=client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
  - `remote_scenario=2hop observed_status=200 ...`
- `docs/artifacts/chaos_terminal_tail_ack_gap_remote_wan_3hop_2026-03-22.log`
  - `remote_scenario=3hop ... route_chain=client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`
  - `remote_scenario=3hop observed_status=200 ...`

Remote perf log:

- `docs/artifacts/chaos_terminal_tail_ack_gap_remote_perf_2026-03-22.log`
  - `perf_scenario=direct ... route_chain=client -> 31.192.232.26:18080 -> target`
  - `perf_scenario=remote-1hop ... route_chain=client -> 31.192.232.26:30000 -> target`
  - `perf_scenario=remote-2hop ... route_chain=client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
  - `perf_scenario=remote-3hop ... route_chain=client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`

Remote stage traces:

- client trace `docs/artifacts/chaos_terminal_tail_ack_gap_remote_perf_stage_2026-03-22.client.jsonl`
  - route_len `1`: `Udp://31.192.232.26:30000`
  - route_len `2`: `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000`
  - route_len `3`: `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000`
- exit trace `docs/artifacts/chaos_terminal_tail_ack_gap_remote_perf_stage_2026-03-22.exit.jsonl`
  - route_len `2` peer: `Udp://45.197.133.115:30001`
  - route_len `3` peer: `Udp://185.144.28.95:30002`

## Counters After Fix

Client stage counters from `docs/artifacts/chaos_terminal_tail_ack_gap_remote_perf_stage_2026-03-22.client.jsonl`:

- `duplicate_close_stream_suppressed = 38`
- `response_transport_terminal_close_observed = 18`
- `response_transport_terminal_close_duplicate = 0`
- `terminal_close_before_local_completion_queued = 0`
- `terminal_payload_after_local_completion = 0`

Exit-stage stream summaries from `docs/artifacts/chaos_terminal_tail_ack_gap_remote_perf_stage_2026-03-22.exit.jsonl`:

- route_len `2` warmup: peer `45.197.133.115:30001`, `ack_latency_ms_p95 = 308`, `total_retransmits = 0`, `window_wait_total_ms = 796`
- route_len `3` warmup: peer `185.144.28.95:30002`, `ack_latency_ms_p95 = 323`, `total_retransmits = 0`, `window_wait_total_ms = 1077`

Remote perf/stage test results:

- perf matrix: `docs/artifacts/chaos_terminal_tail_ack_gap_remote_perf_2026-03-22.log`
  - `1 passed; 0 failed`
- stage matrix: `docs/artifacts/chaos_terminal_tail_ack_gap_remote_perf_stage_2026-03-22.log`
  - `1 passed; 0 failed`

## Cleanup Proof

- remote reset logs:
  - `docs/artifacts/chaos_terminal_tail_ack_gap_remote_reset_3hop_2026-03-22.log`
- remote stage setup log:
  - `docs/artifacts/chaos_terminal_tail_ack_gap_remote_perf_stage_setup_2026-03-22.log`
- remote stage collect log:
  - `docs/artifacts/chaos_terminal_tail_ack_gap_remote_perf_stage_collect_2026-03-22.log`
- remote cleanup log:
  - `docs/artifacts/chaos_terminal_tail_ack_gap_remote_cleanup_2026-03-22.log`
  - final state there: `exit_service=active`, `public_service=inactive`, `cleanup_status=done`
