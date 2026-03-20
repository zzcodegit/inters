# Remote WAN performance matrix

## Purpose

This perf layer sits on top of the accepted remote WAN matrix and answers a different question:

- remote matrix: "does the route work and return `200`?"
- perf matrix: "what is the real cost of `1-hop`, `2-hop`, and `3-hop` relative to a direct path to the same payload?"

## Scenarios

The perf runner measures four scenarios against the same static HTTP payload:

- `direct`
- `remote-1hop`
- `remote-2hop`
- `remote-3hop`

Remote scenarios still use `VPNNODE_BASELINE_REMOTE_EXACT_ROUTE_ONLY=true`.

## Metrics

Each successful run records:

- `connect_time_ms`
- `ttfb_ms`
- `total_time_ms`
- `response_bytes`
- `body_bytes`
- `effective_throughput_bps`

Definitions:

- `connect_time_ms`: TCP connect completion for the measured endpoint
- `ttfb_ms`: time from request flush to the first response byte
- `total_time_ms`: time from connect start to EOF
- `effective_throughput_bps`: response body bytes divided by total request time

Interpretation note:

- `direct` connect time is a public TCP connect to `31.192.232.26:18080`
- `remote-*` connect time is only the local TCP connect into the client ingress listener on `127.0.0.1`

That means the overlay scenarios are directly comparable on `TTFB`, `total_time_ms`, and throughput, but `connect_time_ms` is mostly useful as a sanity metric, not as a fair WAN latency comparison against `direct`.

## Direct path contract

The perf setup uses the same private HTTP target service as the WAN matrix:

- private target: `127.0.0.1:8080` on the exit host
- direct path: public TCP forward `31.192.232.26:18080 -> 127.0.0.1:8080`

That keeps the payload identical while providing an honest public direct baseline.

## Payload contract

- file path: `/perf-262144.bin`
- payload size: `262144` bytes
- generated into `/opt/vpnnode/target-http`

The remote perf runner uses scenario-specific `Host:` labels only to make overlay logs attributable. The payload file and target content are identical across all scenarios.

## Running

```bash
set -a
source examples/config/remote-baseline.env.example
set +a
bash scripts/run_perf_matrix.sh
```

Required supporting env:

- `VPNNODE_REMOTE_EXIT_HOST`
- `VPNNODE_REMOTE_EXIT_PASSWORD`
- `VPNNODE_PERF_DIRECT_ADDR`

Typical values for the current public topology:

- `VPNNODE_PERF_DIRECT_ADDR=31.192.232.26:18080`
- `VPNNODE_BASELINE_REMOTE_RELAY1_ADDR=45.197.133.115:30001`
- `VPNNODE_BASELINE_REMOTE_RELAY2_ADDR=185.144.28.95:30002`
- `VPNNODE_BASELINE_REMOTE_EXIT_ADDR=31.192.232.26:30000`

## Artifacts

The default artifact paths are:

- `docs/artifacts/remote_perf_matrix_2026-03-21.log`
- `docs/artifacts/remote_perf_matrix_2026-03-21.jsonl`
- `docs/artifacts/remote_perf_matrix_2026-03-21.md`
- `docs/artifacts/exit_perf_matrix_2026-03-21.log`

The perf report is intended to be read together with the accepted route proof in `docs/artifacts/wan_matrix_2026-03-20_evidence.md`.
