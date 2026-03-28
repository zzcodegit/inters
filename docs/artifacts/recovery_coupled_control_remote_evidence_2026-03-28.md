# Recovery-Coupled Control Remote Evidence

## Exact IP Fleet

- exit: `31.192.232.26:30000`
- relay1: `45.197.133.115:30001`
- relay2: `185.144.28.95:30002`
- relay3: `155.212.135.200:30003`
- relay4: `155.212.135.95:30004`
- relay6: `155.212.135.95:30006`

## Proved Routes

- `1-hop`: `client -> 31.192.232.26:30000 -> target`
- `2-hop`: `client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `3-hop`: `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`
- `5-hop`: `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 155.212.135.200:30003 -> 155.212.135.95:30004 -> 31.192.232.26:30000 -> target`

Topology proof:

- `docs/artifacts/recovery_coupled_control_remote_perf_matrix_2026-03-28.md`
- `docs/artifacts/recovery_coupled_control_route_proof_2026-03-28.md`

## Route-Peer Proof

- `1-hop` exit peer: `Udp://217.173.65.131:*`
- `2-hop` exit peer: `Udp://45.197.133.115:30001`
- `3-hop` exit peer: `Udp://185.144.28.95:30002`
- `5-hop` exit peer: `Udp://155.212.135.95:30004`

Supporting logs:

- `docs/artifacts/recovery_coupled_control_routeproof_1hop_exit_journal_2026-03-28.log`
- `docs/artifacts/recovery_coupled_control_routeproof_2hop_exit_journal_2026-03-28.log`
- `docs/artifacts/recovery_coupled_control_routeproof_3hop_exit_journal_2026-03-28.log`
- `docs/artifacts/recovery_coupled_control_routeproof_5hop_exit_journal_2026-03-28.log`

## Perf Artifacts

- remote WAN / perf matrix:
  - `docs/artifacts/recovery_coupled_control_remote_perf_matrix_2026-03-28.log`
  - `docs/artifacts/recovery_coupled_control_remote_perf_matrix_2026-03-28.md`
  - `docs/artifacts/recovery_coupled_control_remote_perf_matrix_2026-03-28.jsonl`
- remote stage:
  - `docs/artifacts/recovery_coupled_control_remote_perf_stage_2026-03-28.log`
  - `docs/artifacts/recovery_coupled_control_remote_perf_stage_2026-03-28.md`
  - `docs/artifacts/recovery_coupled_control_remote_perf_stage_2026-03-28.exit.jsonl`
  - `docs/artifacts/recovery_coupled_control_remote_perf_stage_2026-03-28.client.jsonl`
  - `docs/artifacts/recovery_coupled_control_remote_perf_stage_2026-03-28.local.jsonl`
  - `docs/artifacts/recovery_coupled_control_remote_perf_stage_2026-03-28.joined.jsonl`

## Cleanup

Final reduced-fleet reset:

- `docs/artifacts/recovery_coupled_control_remote_reset_final_2026-03-28.log`

Final exit/public service state:

- `docs/artifacts/recovery_coupled_control_remote_cleanup_final_verify_2026-03-28.log`

Observed final state:

- `vpnnode-exit.service = active`
- `vpnnode-target-http.service = active`
- `vpnnode-target-http-public.service = inactive`

Local cleanup:

- `docs/artifacts/recovery_coupled_control_local_cleanup_2026-03-28.log`
