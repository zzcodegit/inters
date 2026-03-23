# Short-Path Remote Evidence

## Exact IP Mapping

- exit: `31.192.232.26:30000`
- relay-1: `45.197.133.115:30001`
- relay-2: `185.144.28.95:30002`
- direct public target: `31.192.232.26:18080`

## Route Proof

- `1-hop`: [stage5_shortpath_remote_wan_1hop_2026-03-23.log](../docs/artifacts/stage5_shortpath_remote_wan_1hop_2026-03-23.log)
  - route chain: `client -> 31.192.232.26:30000 -> target`
  - observed status: `200`
- `2-hop`: [stage5_shortpath_remote_wan_matrix_2026-03-23.log](../docs/artifacts/stage5_shortpath_remote_wan_matrix_2026-03-23.log)
  - route chain: `client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
  - observed status: `200`
- `3-hop`: [stage5_shortpath_remote_wan_matrix_2026-03-23.log](../docs/artifacts/stage5_shortpath_remote_wan_matrix_2026-03-23.log)
  - route chain: `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`
  - observed status: `200`

## Exit-Side Stage Proof

From [stage5_shortpath_remote_perf_stage_2026-03-23.exit.jsonl](../docs/artifacts/stage5_shortpath_remote_perf_stage_2026-03-23.exit.jsonl):

- route `1`
  - peer: `Udp://46.242.13.60:2057`
  - route chain: `Udp://31.192.232.26:30000`
- route `2`
  - peer: `Udp://45.197.133.115:30001`
  - route chain: `Udp://45.197.133.115:30001 -> Udp://31.192.232.26:30000`
- route `3`
  - peer: `Udp://185.144.28.95:30002`
  - route chain: `Udp://45.197.133.115:30001 -> Udp://185.144.28.95:30002 -> Udp://31.192.232.26:30000`

## WAN Metrics After Fix

Fresh end-to-end perf from [stage5_shortpath_remote_perf_matrix_2026-03-23.md](../docs/artifacts/stage5_shortpath_remote_perf_matrix_2026-03-23.md):

- `1-hop`: avg total `1122.00 ms`, avg TTFB `1117.62 ms`
- `2-hop`: avg total `1196.78 ms`, avg TTFB `1195.63 ms`
- `3-hop`: avg total `1445.42 ms`, avg TTFB `1441.16 ms`

Fresh stage metrics from [stage5_shortpath_remote_perf_stage_2026-03-23.md](../docs/artifacts/stage5_shortpath_remote_perf_stage_2026-03-23.md):

- `1-hop`: retransmit `0.0366`, ACK avg `178.8 ms`, ACK p95 `191.0 ms`, hard stall `102.2 ms`, effective cap avg `64`, blocked by cap `0`
- `2-hop`: retransmit `0.0000`, ACK avg `183.8 ms`, ACK p95 `193.6 ms`, hard stall `46.2 ms`, effective cap avg `64`, blocked by cap `0`
- `3-hop`: retransmit `0.0000`, ACK avg `235.0 ms`, ACK p95 `246.6 ms`, hard stall `25.6 ms`, effective cap avg `64`, blocked by cap `0`

## Cleanup Proof

- stage cleanup: [stage5_shortpath_remote_perf_stage_cleanup_2026-03-23.log](../docs/artifacts/stage5_shortpath_remote_perf_stage_cleanup_2026-03-23.log)
  - `exit_service=active`
  - `public_service=inactive`
- explicit verify: [stage5_shortpath_remote_cleanup_verify_2026-03-23.log](../docs/artifacts/stage5_shortpath_remote_cleanup_verify_2026-03-23.log)
  - `exit_service=active`
  - `target_service=active`
  - `public_service=inactive`
