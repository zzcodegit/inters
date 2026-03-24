# Retransmit Backoff Remote Evidence

## Exact IP Topology

- exit: `31.192.232.26:30000`
- relay-1: `45.197.133.115:30001`
- relay-2: `185.144.28.95:30002`
- relay-3: `155.212.135.200:30003`
- relay-4: `155.212.135.95:30004`
- stored but not active in this chain: `155.212.143.112:30005`

## Route Proof

WAN matrix log:

- [retransmit_backoff_remote_wan_matrix_direct2_2026-03-23.log](/C:/neinternet/vpnnode/docs/artifacts/retransmit_backoff_remote_wan_matrix_direct2_2026-03-23.log)

Observed routes:

- `1-hop`: `client -> 31.192.232.26:30000 -> target`
- `2-hop`: `client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `3-hop`: `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`
- `5-hop`: `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 155.212.135.200:30003 -> 155.212.135.95:30004 -> 31.192.232.26:30000 -> target`

All four scenarios returned `observed_status=200`.

## Perf Evidence

- raw: [retransmit_backoff_remote_perf_matrix_2026-03-23.jsonl](/C:/neinternet/vpnnode/docs/artifacts/retransmit_backoff_remote_perf_matrix_2026-03-23.jsonl)
- report: [retransmit_backoff_remote_perf_matrix_2026-03-23.md](/C:/neinternet/vpnnode/docs/artifacts/retransmit_backoff_remote_perf_matrix_2026-03-23.md)
- console log: [retransmit_backoff_remote_perf_matrix_2026-03-23.log](/C:/neinternet/vpnnode/docs/artifacts/retransmit_backoff_remote_perf_matrix_2026-03-23.log)

Current avg totals:

- `1-hop`: `1143.17 ms`
- `2-hop`: `1097.70 ms`
- `3-hop`: `1386.50 ms`
- `5-hop`: `1780.34 ms`

## Stage Evidence

- summary: [retransmit_backoff_remote_perf_stage_2026-03-23.md](/C:/neinternet/vpnnode/docs/artifacts/retransmit_backoff_remote_perf_stage_2026-03-23.md)
- joined: [retransmit_backoff_remote_perf_stage_2026-03-23.joined.jsonl](/C:/neinternet/vpnnode/docs/artifacts/retransmit_backoff_remote_perf_stage_2026-03-23.joined.jsonl)
- client trace: [retransmit_backoff_remote_perf_stage_2026-03-23.client.jsonl](/C:/neinternet/vpnnode/docs/artifacts/retransmit_backoff_remote_perf_stage_2026-03-23.client.jsonl)
- exit trace: [retransmit_backoff_remote_perf_stage_2026-03-23.exit.jsonl](/C:/neinternet/vpnnode/docs/artifacts/retransmit_backoff_remote_perf_stage_2026-03-23.exit.jsonl)
- direct trace: [retransmit_backoff_remote_perf_stage_2026-03-23.direct.jsonl](/C:/neinternet/vpnnode/docs/artifacts/retransmit_backoff_remote_perf_stage_2026-03-23.direct.jsonl)
- console log: [retransmit_backoff_remote_perf_stage_2026-03-23.log](/C:/neinternet/vpnnode/docs/artifacts/retransmit_backoff_remote_perf_stage_2026-03-23.log)

Exit-side peer proof from the stage trace:

- route `1`: peer `Udp://217.173.65.131:57190`
- route `2`: peer `Udp://45.197.133.115:30001`
- route `3`: peer `Udp://185.144.28.95:30002`
- route `5`: peer `Udp://155.212.135.95:30004`

Key retransmit fields in the current stage summary:

- `remote-1hop`: `retransmit_trigger_count=0`, `retransmit_rate=0.0000`
- `remote-2hop`: `retransmit_timeout_ms_avg=273`, `retransmit_rtt_ratio=1.50`
- `remote-3hop`: `retransmit_timeout_ms_avg=369`, `retransmit_rtt_ratio=1.50`
- `remote-5hop`: `retransmit_timeout_ms_avg=231`, `retransmit_timeout_ms_p95=242`, `retransmit_rtt_ratio=1.63`

## Cleanup Proof

- remote verify: [retransmit_backoff_remote_cleanup_verify_2026-03-23.log](/C:/neinternet/vpnnode/docs/artifacts/retransmit_backoff_remote_cleanup_verify_2026-03-23.log)
- local verify: [retransmit_backoff_local_cleanup_verify_2026-03-23.log](/C:/neinternet/vpnnode/docs/artifacts/retransmit_backoff_local_cleanup_verify_2026-03-23.log)

Verified state after the remote stage run:

- `relay1 active`
- `relay2 active`
- `relay3 active`
- `relay4 active`
- `exit active`
- `vpnnode-target-http-public.service inactive`

## Credential Location

The shared remote credentials and active topology bootstrap are committed in:

- [examples/config/remote_test_servers.ps1](/C:/neinternet/vpnnode/examples/config/remote_test_servers.ps1)

And documented in:

- [docs/remote_test_servers.md](/C:/neinternet/vpnnode/docs/remote_test_servers.md)
