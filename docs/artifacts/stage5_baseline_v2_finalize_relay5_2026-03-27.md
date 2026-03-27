# Stage 5 Baseline v2 Finalize Relay5

## Summary

- Baseline v2 exact-route fleet is reproducible on `exit`, `relay1`, `relay2`, `relay3`, `relay4`, and `relay6`.
- `relay5` (`155.212.143.112`) is not part of the required `1-hop/2-hop/3-hop/5-hop` acceptance routes, but its missing startup proof previously blocked baseline acceptance.
- On March 27, 2026, `relay5` was re-probed directly:
  - TCP `22` is reachable
  - SSH password auth for `root` is rejected
  - the blocker is external access/auth, not vpnnode relay startup logic
- Fresh isolated exact-route reruns with full reset before each route proved correct `1-hop`, `2-hop`, `3-hop`, and `5-hop` behavior on the reduced fleet.

## Relay5 Status

- TCP probe: [stage5_baseline_v2_finalize_relay5_tcp_probe_2026-03-27.log](/C:/neinternet/vpnnode_stage5_baseline_v2_exact_route/docs/artifacts/stage5_baseline_v2_finalize_relay5_tcp_probe_2026-03-27.log)
  - `RemotePort = 22`
  - `TcpTestSucceeded = True`
- SSH auth probe: [stage5_baseline_v2_finalize_relay5_auth_probe_2026-03-27.log](/C:/neinternet/vpnnode_stage5_baseline_v2_exact_route/docs/artifacts/stage5_baseline_v2_finalize_relay5_auth_probe_2026-03-27.log)
  - `Connected to 155.212.143.112`
  - `Using username "root"`
  - `Password authentication failed`
  - `Configured password was not accepted`
- Secondary PuTTY proof: [stage5_baseline_v2_finalize_relay5_plink_2026-03-27.log](/C:/neinternet/vpnnode_stage5_baseline_v2_exact_route/docs/artifacts/stage5_baseline_v2_finalize_relay5_plink_2026-03-27.log)

Result:

- `relay5 = unreachable for administration with provided credential`
- exclusion reason is external auth failure
- no code workaround or fake peer bootstrap was used

## Startup Proof

Accessible nodes:

- `exit`
  - status: [stage5_baseline_v2_finalize_exit_status_2026-03-27.log](/C:/neinternet/vpnnode_stage5_baseline_v2_exact_route/docs/artifacts/stage5_baseline_v2_finalize_exit_status_2026-03-27.log)
  - startup proof with `peers=0`: [stage5_baseline_v2_finalize_routeproof_1hop_exit_journal_2026-03-27.log](/C:/neinternet/vpnnode_stage5_baseline_v2_exact_route/docs/artifacts/stage5_baseline_v2_finalize_routeproof_1hop_exit_journal_2026-03-27.log)
- `relay1`
  - status: [stage5_baseline_v2_finalize_relay1_status_2026-03-27.log](/C:/neinternet/vpnnode_stage5_baseline_v2_exact_route/docs/artifacts/stage5_baseline_v2_finalize_relay1_status_2026-03-27.log)
  - startup proof with `peers=0`: [stage5_baseline_v2_finalize_routeproof_1hop_relay1_journal_2026-03-27.log](/C:/neinternet/vpnnode_stage5_baseline_v2_exact_route/docs/artifacts/stage5_baseline_v2_finalize_routeproof_1hop_relay1_journal_2026-03-27.log)
- `relay2`
  - status: [stage5_baseline_v2_finalize_relay2_status_2026-03-27.log](/C:/neinternet/vpnnode_stage5_baseline_v2_exact_route/docs/artifacts/stage5_baseline_v2_finalize_relay2_status_2026-03-27.log)
  - startup proof with `peers=0`: [stage5_baseline_v2_finalize_routeproof_1hop_relay2_journal_2026-03-27.log](/C:/neinternet/vpnnode_stage5_baseline_v2_exact_route/docs/artifacts/stage5_baseline_v2_finalize_routeproof_1hop_relay2_journal_2026-03-27.log)
- `relay3`
  - status: [stage5_baseline_v2_finalize_relay3_status_2026-03-27.log](/C:/neinternet/vpnnode_stage5_baseline_v2_exact_route/docs/artifacts/stage5_baseline_v2_finalize_relay3_status_2026-03-27.log)
  - startup proof with `peers=0`: [stage5_baseline_v2_finalize_routeproof_1hop_relay3_journal_2026-03-27.log](/C:/neinternet/vpnnode_stage5_baseline_v2_exact_route/docs/artifacts/stage5_baseline_v2_finalize_routeproof_1hop_relay3_journal_2026-03-27.log)
- `relay4`
  - status: [stage5_baseline_v2_finalize_relay4_status_2026-03-27.log](/C:/neinternet/vpnnode_stage5_baseline_v2_exact_route/docs/artifacts/stage5_baseline_v2_finalize_relay4_status_2026-03-27.log)
  - startup proof with `peers=0`: [stage5_baseline_v2_finalize_routeproof_1hop_relay4_journal_2026-03-27.log](/C:/neinternet/vpnnode_stage5_baseline_v2_exact_route/docs/artifacts/stage5_baseline_v2_finalize_routeproof_1hop_relay4_journal_2026-03-27.log)
- `relay6`
  - status: [stage5_baseline_v2_finalize_relay6_status_2026-03-27.log](/C:/neinternet/vpnnode_stage5_baseline_v2_exact_route/docs/artifacts/stage5_baseline_v2_finalize_relay6_status_2026-03-27.log)
  - startup proof with `peers=0`: [stage5_baseline_v2_finalize_routeproof_1hop_relay6_journal_2026-03-27.log](/C:/neinternet/vpnnode_stage5_baseline_v2_exact_route/docs/artifacts/stage5_baseline_v2_finalize_routeproof_1hop_relay6_journal_2026-03-27.log)

Observed on each accessible node:

- `Active: active (running)`
- `loaded config role=... peers=0 ants=false`
- no restart loop
- no `relay requires at least one peer`

Reduced acceptance fleet:

- `31.192.232.26:30000` (`exit`)
- `45.197.133.115:30001` (`relay1`)
- `185.144.28.95:30002` (`relay2`)
- `155.212.135.200:30003` (`relay3`)
- `155.212.135.95:30004` (`relay4`)
- `155.212.135.95:30006` (`relay6`)

## Route Correctness

All reruns used full reset before the individual route:

- `1-hop` reset: [stage5_baseline_v2_finalize_routeproof_reset_1hop_2026-03-27.log](/C:/neinternet/vpnnode_stage5_baseline_v2_exact_route/docs/artifacts/stage5_baseline_v2_finalize_routeproof_reset_1hop_2026-03-27.log)
- `2-hop` reset: [stage5_baseline_v2_finalize_routeproof_reset_2hop_2026-03-27.log](/C:/neinternet/vpnnode_stage5_baseline_v2_exact_route/docs/artifacts/stage5_baseline_v2_finalize_routeproof_reset_2hop_2026-03-27.log)
- `3-hop` reset: [stage5_baseline_v2_finalize_routeproof_reset_3hop_2026-03-27.log](/C:/neinternet/vpnnode_stage5_baseline_v2_exact_route/docs/artifacts/stage5_baseline_v2_finalize_routeproof_reset_3hop_2026-03-27.log)
- `5-hop` reset: [stage5_baseline_v2_finalize_routeproof_reset_5hop_2026-03-27.log](/C:/neinternet/vpnnode_stage5_baseline_v2_exact_route/docs/artifacts/stage5_baseline_v2_finalize_routeproof_reset_5hop_2026-03-27.log)

Run results:

- `1-hop`: [stage5_baseline_v2_finalize_routeproof_run_1hop_2026-03-27.log](/C:/neinternet/vpnnode_stage5_baseline_v2_exact_route/docs/artifacts/stage5_baseline_v2_finalize_routeproof_run_1hop_2026-03-27.log)
  - declared route: `client -> 31.192.232.26:30000 -> target`
  - observed status: `200`
  - exit peer: client direct in [stage5_baseline_v2_finalize_routeproof_1hop_exit_journal_2026-03-27.log](/C:/neinternet/vpnnode_stage5_baseline_v2_exact_route/docs/artifacts/stage5_baseline_v2_finalize_routeproof_1hop_exit_journal_2026-03-27.log)
  - activity summary: `exit=29`, `relay1=0`, `relay2=0`, `relay3=0`, `relay4=0`, `relay6=0`
- `2-hop`: [stage5_baseline_v2_finalize_routeproof_run_2hop_2026-03-27.log](/C:/neinternet/vpnnode_stage5_baseline_v2_exact_route/docs/artifacts/stage5_baseline_v2_finalize_routeproof_run_2hop_2026-03-27.log)
  - declared route: `client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
  - observed status: `200`
  - exit peer: `Udp://45.197.133.115:30001` in [stage5_baseline_v2_finalize_routeproof_2hop_exit_journal_2026-03-27.log](/C:/neinternet/vpnnode_stage5_baseline_v2_exact_route/docs/artifacts/stage5_baseline_v2_finalize_routeproof_2hop_exit_journal_2026-03-27.log)
  - activity summary: `exit=25`, `relay1=55`, `relay2=0`, `relay3=0`, `relay4=0`, `relay6=0`
- `3-hop`: [stage5_baseline_v2_finalize_routeproof_run_3hop_2026-03-27.log](/C:/neinternet/vpnnode_stage5_baseline_v2_exact_route/docs/artifacts/stage5_baseline_v2_finalize_routeproof_run_3hop_2026-03-27.log)
  - declared route: `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`
  - observed status: `200`
  - exit peer: `Udp://185.144.28.95:30002` in [stage5_baseline_v2_finalize_routeproof_3hop_exit_journal_2026-03-27.log](/C:/neinternet/vpnnode_stage5_baseline_v2_exact_route/docs/artifacts/stage5_baseline_v2_finalize_routeproof_3hop_exit_journal_2026-03-27.log)
  - activity summary: `exit=35`, `relay1=121`, `relay2=42`, `relay3=0`, `relay4=0`, `relay6=0`
- `5-hop`: [stage5_baseline_v2_finalize_routeproof_run_5hop_2026-03-27.log](/C:/neinternet/vpnnode_stage5_baseline_v2_exact_route/docs/artifacts/stage5_baseline_v2_finalize_routeproof_run_5hop_2026-03-27.log)
  - declared route: `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 155.212.135.200:30003 -> 155.212.135.95:30004 -> 31.192.232.26:30000 -> target`
  - observed status: `200`
  - exit peer: `Udp://155.212.135.95:30004` in [stage5_baseline_v2_finalize_routeproof_5hop_exit_journal_2026-03-27.log](/C:/neinternet/vpnnode_stage5_baseline_v2_exact_route/docs/artifacts/stage5_baseline_v2_finalize_routeproof_5hop_exit_journal_2026-03-27.log)
  - activity summary: `exit=35`, `relay1=46`, `relay2=46`, `relay3=46`, `relay4=46`, `relay6=0`

Compact activity artifact:

- [stage5_baseline_v2_finalize_routeproof_activity_summary_2026-03-27.txt](/C:/neinternet/vpnnode_stage5_baseline_v2_exact_route/docs/artifacts/stage5_baseline_v2_finalize_routeproof_activity_summary_2026-03-27.txt)

Route correctness result:

- no hidden relay hits in the reduced acceptance fleet
- every non-route relay stayed idle
- every exit peer matched the declared previous hop

## Cleanup Proof

- remote cleanup: [stage5_baseline_v2_finalize_remote_cleanup_2026-03-27.log](/C:/neinternet/vpnnode_stage5_baseline_v2_exact_route/docs/artifacts/stage5_baseline_v2_finalize_remote_cleanup_2026-03-27.log)
- local cleanup: [stage5_baseline_v2_finalize_local_cleanup_2026-03-27.log](/C:/neinternet/vpnnode_stage5_baseline_v2_exact_route/docs/artifacts/stage5_baseline_v2_finalize_local_cleanup_2026-03-27.log)

Observed cleanup state:

- remote reduced fleet reset completed and all required services were returned to active baseline state
- local cleanup ended with `processes: none`

## Final Verdict

`BASELINE v2 = VALID (reduced fleet, relay5 excluded)`

Reason:

- exact-route routing is reproducible on the required `1-hop`, `2-hop`, `3-hop`, and `5-hop` routes
- accessible nodes start cleanly with `peers=0`
- relay5 exclusion is proven external (`22/tcp reachable`, password rejected), not a vpnnode code failure
- no workaround changed routing, hard window, pacing, retransmit policy, or Stage 4 transport semantics
