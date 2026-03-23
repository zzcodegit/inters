# RTT-Aware Inflight Discipline Remote Evidence

## Topology

- exit: `31.192.232.26:30000`
- relay-1: `45.197.133.115:30001`
- relay-2: `185.144.28.95:30002`
- direct public target path: `31.192.232.26:18080`

## Route Proof

Source: `docs/artifacts/stage5_inflight_remote_wan_matrix_final_2026-03-23.log`

- `1-hop`
  - route chain: `client -> 31.192.232.26:30000 -> target`
  - observed status: `200`
  - duration: `1745 ms`
- `2-hop`
  - route chain: `client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
  - observed status: `200`
  - duration: `1412 ms`
- `3-hop`
  - route chain: `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`
  - observed status: `200`
  - duration: `2026 ms`

## Exit Peer Proof

Source: `docs/artifacts/stage5_inflight_remote_perf_stage_final_2026-03-23.exit.jsonl`

- route `1`, sample `stage-remote-1hop-run1`
  - peer: `Udp://217.173.65.131:61814`
- route `2`, sample `stage-remote-2hop-run1`
  - peer: `Udp://45.197.133.115:30001`
- route `3`, sample `stage-remote-3hop-run1`
  - peer: `Udp://185.144.28.95:30002`

This matches the intended relay/exit hop mapping:

- direct client peer at exit for `1-hop`
- relay-1 as the exit peer for `2-hop`
- relay-2 as the exit peer for `3-hop`

## Remote Perf After Inflight Discipline

Sources:

- `docs/artifacts/stage5_inflight_remote_perf_matrix_final_2026-03-23.md`
- `docs/artifacts/stage5_inflight_remote_perf_stage_final_2026-03-23.md`

### End-to-End

| route | total ms avg | TTFB ms avg | throughput avg Bps |
| --- | --- | --- | --- |
| 1-hop | 1116.22 | 1114.95 | 234930 |
| 2-hop | 1811.82 | 1810.92 | 184787 |
| 3-hop | 1487.11 | 1483.18 | 178457 |

### Stage Metrics

| route | ACK avg ms | ACK p95 ms | hard stall ms | effective cap wait ms | effective cap avg | cap reduced total | blocked by cap total |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1-hop | 216.2 | 340.2 | 199.4 | 6.0 | 63.4 | 2 | 2 |
| 2-hop | 193.2 | 263.6 | 105.8 | 0.8 | 63.8 | 1 | 1 |
| 3-hop | 239.4 | 269.8 | 24.2 | 198.0 | 61.2 | 3 | 189 |

## Client Lifecycle Counters

Source: `docs/artifacts/stage5_inflight_remote_perf_stage_final_2026-03-23.client.jsonl`

- `response_transport_local_completion = 21`
- `response_transport_settlement_completed = 18`
- `duplicate_payload_after_local_completion = 0`
- `terminal_payload_after_local_completion = 0`

These counters confirm that the accepted Stage 4 lifecycle hardening stayed intact while the new control layer was exercised on real WAN routes.

## Cleanup Proof

Local cleanup:

- `docs/artifacts/stage5_inflight_local_cleanup_verify_2026-03-23.log`
  - `listening_ports=none`
  - `vpnnode_processes=none`

Remote cleanup:

- `docs/artifacts/stage5_inflight_remote_perf_stage_final_2026-03-23.log`
  - `exit_service=active`
  - `public_service=inactive`
  - `cleanup_status=done`
