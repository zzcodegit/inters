# Response Pacing Remote Evidence

## Exact Topology

- exit: `31.192.232.26:30000`
- relay-1: `45.197.133.115:30001`
- relay-2: `185.144.28.95:30002`

## Route Proof

From [stage5_pacing_remote_wan_matrix_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/stage5_pacing_remote_wan_matrix_2026-03-22.log):

- `1-hop`: `client -> 31.192.232.26:30000 -> target`
- `2-hop`: `client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `3-hop`: `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`

Observed HTTP status in that same log:

- `1-hop`: `200`
- `2-hop`: `200`
- `3-hop`: `200`

## Exit Peer Proof

From [stage5_pacing_remote_perf_stage_2026-03-22.exit.jsonl](/C:/neinternet/vpnnode/docs/artifacts/stage5_pacing_remote_perf_stage_2026-03-22.exit.jsonl):

- route `1`: peer `Udp://217.173.65.131:53615`
- route `2`: peer `Udp://45.197.133.115:30001`
- route `3`: peer `Udp://185.144.28.95:30002`

That proves the exit really saw:

- direct client public ingress on `1-hop`
- London relay on `2-hop`
- Warsaw relay on `3-hop`

## Final Pacing Counters

From [stage5_pacing_remote_perf_stage_2026-03-22.md](/C:/neinternet/vpnnode/docs/artifacts/stage5_pacing_remote_perf_stage_2026-03-22.md):

- `remote-1hop`
  - `max burst=4`
  - `pacing interval=3 ms`
  - `pacing delay=729.0 ms`
  - `burst prevented=273`
  - `ack p95=190.0 ms`
  - `window stall=32.4 ms`
- `remote-2hop`
  - `max burst=4`
  - `pacing interval=3 ms`
  - `pacing delay=749.6 ms`
  - `burst prevented=273`
  - `ack p95=200.4 ms`
  - `window stall=44.0 ms`
- `remote-3hop`
  - `max burst=4`
  - `pacing interval=4 ms`
  - `pacing delay=963.2 ms`
  - `burst prevented=257`
  - `ack p95=288.2 ms`
  - `window stall=155.0 ms`

## Cleanup Proof

From [stage5_pacing_remote_cleanup_verify_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/stage5_pacing_remote_cleanup_verify_2026-03-22.log):

- `vpnnode-exit.service = active`
- `vpnnode-target-http.service = active`
- `vpnnode-target-http-public.service = inactive`

From [stage5_pacing_local_cleanup_verify_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/stage5_pacing_local_cleanup_verify_2026-03-22.log):

- local worker processes: `none`
- listening test ports: `none`
