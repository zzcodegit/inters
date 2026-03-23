# Response Inflight Corrective Remote Evidence

## Exact IP Mapping

- exit: `31.192.232.26:30000`
- relay-1: `45.197.133.115:30001`
- relay-2: `185.144.28.95:30002`
- direct public target: `31.192.232.26:18080`

## WAN Route Proof

Source: [stage5_inflight_corrected_final_remote_wan_matrix_2026-03-23.log](./stage5_inflight_corrected_final_remote_wan_matrix_2026-03-23.log)

- `1-hop`: `client -> 31.192.232.26:30000 -> target`, `observed_status=200`, `duration_ms=1131`
- `2-hop`: `client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`, `observed_status=200`, `duration_ms=1266`
- `3-hop`: `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`, `observed_status=200`, `duration_ms=1401`

## Remote Perf Proof

Source: [stage5_inflight_corrected_final_remote_perf_matrix_2026-03-23.md](./stage5_inflight_corrected_final_remote_perf_matrix_2026-03-23.md)

- `1-hop`: avg total `1555.92 ms`, avg TTFB `1554.75 ms`
- `2-hop`: avg total `1212.74 ms`, avg TTFB `1209.77 ms`
- `3-hop`: avg total `1407.29 ms`, avg TTFB `1405.83 ms`

## Remote Stage Proof

Source: [stage5_inflight_corrected_final_remote_perf_stage_2026-03-23.md](./stage5_inflight_corrected_final_remote_perf_stage_2026-03-23.md)

- `1-hop`: `ack avg=244.00`, `ack p95=397.60`, `hard stall=201.60`, `effective cap avg=64`, `blocked by cap=0`
- `2-hop`: `ack avg=185.60`, `ack p95=203.60`, `hard stall=54.80`, `effective cap avg=64`, `blocked by cap=0`
- `3-hop`: `ack avg=239.20`, `ack p95=251.80`, `hard stall=41.40`, `effective cap avg=64`, `blocked by cap=0`

Exit peer proof from [stage5_inflight_corrected_final_remote_perf_stage_2026-03-23.exit.jsonl](./stage5_inflight_corrected_final_remote_perf_stage_2026-03-23.exit.jsonl):

- route `1`: peer `Udp://217.173.65.131:54664`
- route `2`: peer `Udp://45.197.133.115:30001`
- route `3`: peer `Udp://185.144.28.95:30002`

## Selective Policy Proof

Final corrected exit trace:

- no `effective_inflight_cap_reduced`
- no `send_blocked_by_effective_cap`
- `max_pressure_streak=0` on `2-hop`
- `max_pressure_streak=0/1` on the live routes overall

That proves the controller did not choke the live WAN routes in this final sample.

## Cleanup Proof

- remote cleanup verify: [stage5_inflight_corrected_final_remote_cleanup_verify_2026-03-23.log](./stage5_inflight_corrected_final_remote_cleanup_verify_2026-03-23.log)
  - `exit_service=active`
  - `vpnnode-target-http.service=active`
  - `public_service=inactive`
- stage cleanup: [stage5_inflight_corrected_final_remote_perf_stage_cleanup_2026-03-23.log](./stage5_inflight_corrected_final_remote_perf_stage_cleanup_2026-03-23.log)
  - `cleanup_status=done`
- local cleanup verify: [stage5_inflight_corrected_final_local_cleanup_verify_2026-03-23.log](./stage5_inflight_corrected_final_local_cleanup_verify_2026-03-23.log)
  - `processes_present: none`
  - `listening_ports: none`
