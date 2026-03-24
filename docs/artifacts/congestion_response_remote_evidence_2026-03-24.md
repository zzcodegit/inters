# Congestion Response Remote Evidence

## Topology

- exit: `31.192.232.26:30000`
- relay-1: `45.197.133.115:30001`
- relay-2: `185.144.28.95:30002`
- relay-3: `155.212.135.200:30003`
- relay-4: `155.212.135.95:30004`
- relay-5: `155.212.143.112:30005`
- relay-6: `155.212.135.95:30006`

## Proven Routes

- `1-hop`: `client -> 31.192.232.26:30000 -> target`
- `2-hop`: `client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `3-hop`: `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`
- `5-hop`: `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 155.212.135.200:30003 -> 155.212.135.95:30004 -> 31.192.232.26:30000 -> target`

Proof log: [docs/artifacts/congestion_response_remote_wan_matrix_2026-03-24.log](/C:/neinternet/vpnnode/docs/artifacts/congestion_response_remote_wan_matrix_2026-03-24.log)

## 7-Hop Attempt

Intended route:

- `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 155.212.135.200:30003 -> 155.212.135.95:30004 -> 155.212.135.95:30006 -> 155.212.143.112:30005 -> 31.192.232.26:30000 -> target`

Observed result:

- the local TCP listener for the 7-hop client did not come up
- isolated probe ended with `os error 10065`

Proof:

- [docs/artifacts/congestion_response_remote_wan_7hop_probe_2026-03-24.log](/C:/neinternet/vpnnode/docs/artifacts/congestion_response_remote_wan_7hop_probe_2026-03-24.log)
- [docs/artifacts/congestion_response_remote_7hop_failure_probe_2026-03-24.log](/C:/neinternet/vpnnode/docs/artifacts/congestion_response_remote_7hop_failure_probe_2026-03-24.log)

## Stage Proof

The selective congestion response is visible in:

- [docs/artifacts/congestion_response_remote_perf_stage_2026-03-24.md](/C:/neinternet/vpnnode/docs/artifacts/congestion_response_remote_perf_stage_2026-03-24.md)
- [docs/artifacts/congestion_response_remote_perf_stage_2026-03-24.client.jsonl](/C:/neinternet/vpnnode/docs/artifacts/congestion_response_remote_perf_stage_2026-03-24.client.jsonl)
- [docs/artifacts/congestion_response_remote_perf_stage_2026-03-24.exit.jsonl](/C:/neinternet/vpnnode/docs/artifacts/congestion_response_remote_perf_stage_2026-03-24.exit.jsonl)

Observed route behavior:

- `1-hop/2-hop/3-hop`: `congestion_events = 0`
- `5-hop`: `congestion_events = 2`, `congestion_duration_ms = 180.80`, `send_blocked_by_effective_cap = 5`

## Remote Provision / Deploy / Reset

- provision: [docs/artifacts/congestion_response_remote_provision_2026-03-24.log](/C:/neinternet/vpnnode/docs/artifacts/congestion_response_remote_provision_2026-03-24.log)
- deploy: [docs/artifacts/congestion_response_remote_deploy_2026-03-24.log](/C:/neinternet/vpnnode/docs/artifacts/congestion_response_remote_deploy_2026-03-24.log)
- reset: [docs/artifacts/congestion_response_remote_reset_2026-03-24.log](/C:/neinternet/vpnnode/docs/artifacts/congestion_response_remote_reset_2026-03-24.log)

## Cleanup Proof

- local: [docs/artifacts/congestion_response_local_cleanup_verify_2026-03-24.log](/C:/neinternet/vpnnode/docs/artifacts/congestion_response_local_cleanup_verify_2026-03-24.log)
- remote: [docs/artifacts/congestion_response_remote_cleanup_verify_2026-03-24.log](/C:/neinternet/vpnnode/docs/artifacts/congestion_response_remote_cleanup_verify_2026-03-24.log)

Remote cleanup state from the verify log:

- exit service active
- public target service inactive
- no relay service left active except `relay6`, which is configured as a dedicated extra-hop service on `155.212.135.95`
