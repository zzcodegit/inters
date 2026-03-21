# Remote Perf Stage Breakdown

## Purpose

This layer sits above the accepted remote WAN perf matrix and answers a narrower question:

- not just "how slow is `1-hop / 2-hop / 3-hop`"
- but "which stage is actually burning the time"

## Runner

Use:

```bash
bash scripts/run_perf_stage_matrix.sh
```

The runner keeps the same public topology and payload contract as the remote perf matrix:

- direct: `31.192.232.26:18080 -> 127.0.0.1:8080`
- 1-hop: `client -> 31.192.232.26:30000 -> target`
- 2-hop: `client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- 3-hop: `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`

The payload remains:

- path: `/perf-262144.bin`
- size: `262144` bytes

## Captured Stages

For remote scenarios the artifacts combine:

- local measurement timings from the perf stage test
- client-side stage JSONL via `VPNNODE_STAGE_TRACE_PATH`
- exit-side stage JSONL via remote `VPNNODE_STAGE_TRACE_PATH`
- direct-forward stage JSONL from the public TCP forwarder

The breakdown focuses on:

- route ready
- session established
- request accepted by client ingress
- first hop send
- target connect
- first byte from target
- first overlay response send from exit
- first byte delivered to client
- final byte delivered
- retransmit / ACK context
- response window stall and configured window size
- ACK latency min/p50/p95/max

## Artifacts

The main outputs are:

- `docs/artifacts/remote_perf_stage_matrix_2026-03-21.local.jsonl`
- `docs/artifacts/remote_perf_stage_matrix_2026-03-21.client.jsonl`
- `docs/artifacts/remote_perf_stage_matrix_2026-03-21.exit.jsonl`
- `docs/artifacts/remote_perf_stage_matrix_2026-03-21.direct.jsonl`
- `docs/artifacts/remote_perf_stage_matrix_2026-03-21.joined.jsonl`
- `docs/artifacts/remote_perf_stage_matrix_2026-03-21.md`
- `docs/artifacts/remote_perf_stage_matrix_2026-03-21.log`
- `docs/artifacts/remote_perf_stage_matrix_2026-03-21.deploy.log`

## Remote Binary Requirement

Because exit-side stage timing is emitted by production code on the remote nodes, the remote `vpnnode` binary must include the current instrumentation changes.

For the current workflow that means:

- build the Linux binary under WSL into `target-wsl/release/vpnnode`
- deploy it with `scripts/deploy_remote_binary.ps1`
- then run `scripts/run_perf_stage_matrix.sh`

## Current Conclusion

The current RCA result is in `docs/artifacts/remote_perf_stage_matrix_2026-03-21.md`.

In the current public topology, the dominant steady-state cost is not target connect and not target first-byte generation. The main loss sits in overlay delivery itself:

- large overlay first-send to client first-byte gap
- large window/backpressure stall on the exit response path
- strong coupling between total time, retransmit rate, and ACK latency

The latest response-window RCA and follow-up are:

- `docs/overlay_response_window_rca.md`
- `docs/overlay_response_window_optimization_note.md`
