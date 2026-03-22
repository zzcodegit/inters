# Remote Evidence: Delay-Tail Fix Retest

## Exact WAN Topology

- relay-1: `45.197.133.115:30001`
- relay-2: `185.144.28.95:30002`
- exit: `31.192.232.26:30000`
- direct public target path for perf: `31.192.232.26:18080`

## Route Proof

### Remote WAN Matrix

- 1-hop:
  - [chaos_delay_tail_remote_wan_matrix_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_delay_tail_remote_wan_matrix_2026-03-22.log)
  - route evidence:
    - `client -> 31.192.232.26:30000 -> target`
    - `observed_status=200`
- 2-hop:
  - [chaos_delay_tail_remote_wan_matrix_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_delay_tail_remote_wan_matrix_2026-03-22.log)
  - route evidence:
    - `client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
    - `observed_status=200`
- 3-hop:
  - reset:
    - [chaos_delay_tail_remote_reset_3hop_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_delay_tail_remote_reset_3hop_2026-03-22.log)
  - scenario log:
    - [chaos_delay_tail_remote_wan_3hop_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_delay_tail_remote_wan_3hop_2026-03-22.log)
  - route evidence:
    - `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`
    - `observed_status=200`

## Remote Perf Proof

- Runner log:
  - [chaos_delay_tail_remote_perf_2026-03-22_clean.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_delay_tail_remote_perf_2026-03-22_clean.log)
- Summary:
  - [chaos_delay_tail_remote_perf_2026-03-22_clean.md](/C:/neinternet/vpnnode/docs/artifacts/chaos_delay_tail_remote_perf_2026-03-22_clean.md)
- Raw:
  - [chaos_delay_tail_remote_perf_2026-03-22_clean.jsonl](/C:/neinternet/vpnnode/docs/artifacts/chaos_delay_tail_remote_perf_2026-03-22_clean.jsonl)

The summary carries exact route chains for all scenarios:

- `direct`: `client -> 31.192.232.26:18080 -> target`
- `remote-1hop`: `client -> 31.192.232.26:30000 -> target`
- `remote-2hop`: `client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `remote-3hop`: `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`

## Remote Stage Proof

- Stage runner log:
  - [chaos_delay_tail_remote_perf_stage_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_delay_tail_remote_perf_stage_2026-03-22.log)
- Stage summary:
  - [chaos_delay_tail_remote_perf_stage_2026-03-22.md](/C:/neinternet/vpnnode/docs/artifacts/chaos_delay_tail_remote_perf_stage_2026-03-22.md)
- Client stage trace:
  - [chaos_delay_tail_remote_perf_stage_2026-03-22.client.jsonl](/C:/neinternet/vpnnode/docs/artifacts/chaos_delay_tail_remote_perf_stage_2026-03-22.client.jsonl)
- Exit stage trace:
  - [chaos_delay_tail_remote_perf_stage_2026-03-22.exit.jsonl](/C:/neinternet/vpnnode/docs/artifacts/chaos_delay_tail_remote_perf_stage_2026-03-22.exit.jsonl)
- Joined per-run stage view:
  - [chaos_delay_tail_remote_perf_stage_2026-03-22.joined.jsonl](/C:/neinternet/vpnnode/docs/artifacts/chaos_delay_tail_remote_perf_stage_2026-03-22.joined.jsonl)

## Cleanup Proof

- Local cleanup:
  - [chaos_delay_tail_cleanup_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_delay_tail_cleanup_2026-03-22.log)
  - shows:
    - `processes_present: none`
    - `listening_ports: none`
- Remote cleanup:
  - [chaos_delay_tail_remote_perf_stage_cleanup_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_delay_tail_remote_perf_stage_cleanup_2026-03-22.log)
  - shows:
    - `exit_service=active`
    - `public_service=inactive`
    - `cleanup_status=done`

## Delay / Chaos Profile Evidence

- Delay-only profile:
  - [chaos_matrix_2026-03-22_delay_tail_fix_mild-delay.profile.json](/C:/neinternet/vpnnode/docs/artifacts/chaos_matrix_2026-03-22_delay_tail_fix_mild-delay.profile.json)
- Combined profile:
  - [chaos_matrix_2026-03-22_delay_tail_fix_combined.profile.json](/C:/neinternet/vpnnode/docs/artifacts/chaos_matrix_2026-03-22_delay_tail_fix_combined.profile.json)

These files capture the exact deterministic fault-injection settings used for the post-fix chaos rerun.
