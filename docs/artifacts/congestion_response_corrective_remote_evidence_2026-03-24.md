# Congestion Response Corrective Remote Evidence

## Proven Routes

Current WAN proof on the corrected binary:

- `1-hop`: `client -> 31.192.232.26:30000 -> target`
- `2-hop`: `client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `3-hop`: `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`
- `5-hop`: `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 155.212.135.200:30003 -> 155.212.135.95:30004 -> 31.192.232.26:30000 -> target`

Proof log:

- [docs/artifacts/congestion_response_corrective_remote_wan_matrix_2026-03-24.log](/C:/neinternet/vpnnode/docs/artifacts/congestion_response_corrective_remote_wan_matrix_2026-03-24.log)

## 7-Hop Probe

The probe was re-run on the corrected binary and is still blocked.

Intended route:

- `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 155.212.135.200:30003 -> 155.212.135.95:30004 -> 155.212.135.95:30006 -> 155.212.143.112:30005 -> 31.192.232.26:30000 -> target`

Observed result:

- local client listener did not come up
- route remains blocked at the same external point

Proof:

- [docs/artifacts/congestion_response_corrective_remote_reset_7hop_probe_2026-03-24.log](/C:/neinternet/vpnnode/docs/artifacts/congestion_response_corrective_remote_reset_7hop_probe_2026-03-24.log)
- [docs/artifacts/congestion_response_corrective_remote_wan_7hop_probe_2026-03-24.log](/C:/neinternet/vpnnode/docs/artifacts/congestion_response_corrective_remote_wan_7hop_probe_2026-03-24.log)

## Current Remote Perf / Stage Artifacts

- perf: [docs/artifacts/congestion_response_corrective_remote_perf_matrix_2026-03-24.md](/C:/neinternet/vpnnode/docs/artifacts/congestion_response_corrective_remote_perf_matrix_2026-03-24.md)
- stage: [docs/artifacts/congestion_response_corrective_remote_perf_stage_2026-03-24.md](/C:/neinternet/vpnnode/docs/artifacts/congestion_response_corrective_remote_perf_stage_2026-03-24.md)
- stage exit trace: [docs/artifacts/congestion_response_corrective_remote_perf_stage_2026-03-24.exit.jsonl](/C:/neinternet/vpnnode/docs/artifacts/congestion_response_corrective_remote_perf_stage_2026-03-24.exit.jsonl)

## Cleanup

- local cleanup: [docs/artifacts/congestion_response_corrective_local_cleanup_verify_2026-03-24.log](/C:/neinternet/vpnnode/docs/artifacts/congestion_response_corrective_local_cleanup_verify_2026-03-24.log)
- remote cleanup: [docs/artifacts/congestion_response_corrective_remote_cleanup_verify_2026-03-24.log](/C:/neinternet/vpnnode/docs/artifacts/congestion_response_corrective_remote_cleanup_verify_2026-03-24.log)
