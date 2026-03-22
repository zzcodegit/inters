# Remote Evidence: Duplicate Noise Fix

## Exact Topology

- relay-1: `45.197.133.115:30001`
- relay-2: `185.144.28.95:30002`
- exit: `31.192.232.26:30000`
- direct public target path: `31.192.232.26:18080`

## Route Proof

Remote WAN logs after the fix:

- 1-hop: `docs/artifacts/chaos_duplicate_noise_remote_wan_1hop_2026-03-22.log`
- 2-hop: `docs/artifacts/chaos_duplicate_noise_remote_wan_2hop_2026-03-22.log`
- 3-hop: `docs/artifacts/chaos_duplicate_noise_remote_wan_3hop_2026-03-22.log`

Observed route chains from those logs:

- `client -> 31.192.232.26:30000 -> target`
- `client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`

Reset logs proving the exact hosts involved:

- `docs/artifacts/chaos_duplicate_noise_remote_reset_1hop_2026-03-22.log`
- `docs/artifacts/chaos_duplicate_noise_remote_reset_2hop_2026-03-22.log`
- `docs/artifacts/chaos_duplicate_noise_remote_reset_3hop_2026-03-22.log`

Each reset log shows:

- `relay-1 host=45.197.133.115`
- `relay-2 host=185.144.28.95`
- `exit host=31.192.232.26`

## Remote Perf / Stage Proof

Remote perf matrix:

- log: `docs/artifacts/chaos_duplicate_noise_remote_perf_2026-03-22.log`
- raw: `docs/artifacts/chaos_duplicate_noise_remote_perf_2026-03-22.jsonl`
- report: `docs/artifacts/chaos_duplicate_noise_remote_perf_2026-03-22.md`

The perf log prints:

- `direct route_chain=client -> 31.192.232.26:18080 -> target`
- `remote-1hop route_chain=client -> 31.192.232.26:30000 -> target`
- `remote-2hop route_chain=client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `remote-3hop route_chain=client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`

Remote stage run:

- log: `docs/artifacts/chaos_duplicate_noise_remote_perf_stage_2026-03-22.log`
- client stage trace: `docs/artifacts/chaos_duplicate_noise_remote_perf_stage_2026-03-22.client.jsonl`
- exit stage trace: `docs/artifacts/chaos_duplicate_noise_remote_perf_stage_2026-03-22.exit.jsonl`
- summary: `docs/artifacts/chaos_duplicate_noise_remote_perf_stage_2026-03-22.md`

## Duplicate / Terminal Counters After Fix

Remote stage traces after the fix show:

| component | counter | count |
| --- | --- | ---: |
| client | `duplicate_packet_dropped` | 0 |
| exit | `duplicate_packet_dropped` | 0 |
| client | `open_message_failed:*duplicate packet detected*` | 0 |
| exit | `open_message_failed:*duplicate packet detected*` | 0 |
| client | `response_transport_terminal_close_duplicate` | 36 |

Interpretation:

- the duplicate-noise fix did not introduce duplicate-failure accounting on real WAN paths
- idempotent terminal close duplicates still exist as terminal-settlement observations
- those terminal duplicates do not appear as `open_message_failed`

## Cleanup Proof

Remote cleanup logs:

- raw cleanup log: `docs/artifacts/chaos_duplicate_noise_remote_cleanup_2026-03-22.log`
- verification log: `docs/artifacts/chaos_duplicate_noise_remote_cleanup_verify_2026-03-22.log`

Verified final state:

- `exit_service=active`
- `public_service=inactive`

Local cleanup proof:

- `docs/artifacts/chaos_duplicate_noise_local_cleanup_2026-03-22.log`

Verified local final state:

- `processes: none`
- `listening_ports: none`
