# Stage 4 Freeze Remote Evidence

## Topology

- relay-1: `45.197.133.115:30001` (London)
- relay-2: `185.144.28.95:30002` (Warsaw)
- exit: `31.192.232.26:30000` (Los Angeles)
- direct perf endpoint: `31.192.232.26:18080`

## WAN matrix proof

From [docs/artifacts/stage4_freeze_remote_wan_matrix_2026-03-22.log](stage4_freeze_remote_wan_matrix_2026-03-22.log):

- `1-hop`
  - route chain: `client -> 31.192.232.26:30000 -> target`
  - observed status: `200`
  - duration: `1186 ms`
- `2-hop`
  - route chain: `client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
  - observed status: `200`
  - duration: `1693 ms`
- `3-hop`
  - route chain: `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`
  - observed status: `200`
  - duration: `1759 ms`

The reset hook in the same log proves the shared WAN topology was explicitly restarted before each scenario.

## Remote perf proof

From [docs/artifacts/stage4_freeze_remote_perf_2026-03-22.log](stage4_freeze_remote_perf_2026-03-22.log):

- `direct`: `success_count=5 error_count=0`
- `remote-1hop`: `success_count=5 error_count=0`
- `remote-2hop`: `success_count=5 error_count=0`
- `remote-3hop`: `success_count=5 error_count=0`

The matching route chains are also printed there:

- `direct`: `client -> 31.192.232.26:18080 -> target`
- `remote-1hop`: `client -> 31.192.232.26:30000 -> target`
- `remote-2hop`: `client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `remote-3hop`: `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`

## Remote stage proof

From [docs/artifacts/stage5_entry_perf_stage_2026-03-22.exit.jsonl](stage5_entry_perf_stage_2026-03-22.exit.jsonl):

- route `1` peer: `Udp://46.242.13.60:2077`
- route `2` peer: `Udp://45.197.133.115:30001`
- route `3` peer: `Udp://185.144.28.95:30002`

That confirms the overlay hops seen by the exit during the freeze stage run.

From [docs/artifacts/stage5_entry_perf_stage_2026-03-22.client.jsonl](stage5_entry_perf_stage_2026-03-22.client.jsonl):

- `duplicate_payload_after_local_completion = 0`
- `duplicate_payload_after_local_completion_repeat = 0`
- `terminal_payload_after_local_completion = 0`
- `duplicate_terminal_payload_after_local_completion = 0`
- `payload_after_local_completion_during_settlement = 0`
- `duplicate_close_stream_suppressed = 36`
- `duplicate_payload_after_local_completion_absorbed = 2`
- `response_transport_local_completion = 21`
- `response_transport_settlement_completed = 18`
- `response_transport_terminal_payload_observed = 20`
- `response_transport_terminal_close_observed = 18`

This is the freeze-quality proof that the accepted Stage 4 lifecycle policy is active on the real WAN path.

## Cleanup proof

From [docs/artifacts/stage4_freeze_remote_cleanup_verify_2026-03-22.log](stage4_freeze_remote_cleanup_verify_2026-03-22.log):

- relay-1 service: `active`
- relay-2 service: `active`
- exit service: `active`
- target-http service: `active`
- public perf forwarder: `inactive`
- exit stage-trace drop-in: `absent`

That is the final remote cleanup state after the freeze rerun.
