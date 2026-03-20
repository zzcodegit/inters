# Remote-capable baseline

## What changed

The repository now has two explicit baseline modes:

- `local` mode keeps the existing localhost integration suite and still exercises the full 9-test baseline matrix.
- `remote` mode is a separate WAN verification matrix that takes topology from environment variables instead of hardcoded `127.0.0.1` addresses.

The split is intentional. The full local suite can deterministically control target behavior, port allocation, and failure modes. The remote suite is restricted to honest WAN-capable paths that can be exercised against real relays/exits without pretending that localhost-only scenarios are now WAN-safe.

## Local mode

Run the existing suite with:

```bash
bash scripts/run_baseline.sh
```

Notes:

- The script exports `VPNNODE_BASELINE_MODE=local`.
- It still runs the baseline suite 5 times in a row.
- It still uses `--test-threads=1` because the local suite intentionally uses fixed port blocks.

## Remote mode

Run the WAN matrix with:

```bash
set -a
source examples/config/remote-baseline.env.example
set +a
bash scripts/run_baseline_remote.sh
```

Environment contract:

- `VPNNODE_BASELINE_REMOTE_ROUTE_LENGTH`: `1`, `2`, or `3`
- `VPNNODE_BASELINE_REMOTE_LOCAL_LISTEN`: local TCP listener for the client process
- `VPNNODE_BASELINE_REMOTE_RELAY1_ADDR`: required for route length `>= 2`
- `VPNNODE_BASELINE_REMOTE_RELAY2_ADDR`: required for route length `>= 3`
- `VPNNODE_BASELINE_REMOTE_EXIT_ADDR`: required
- `VPNNODE_BASELINE_REMOTE_TARGET_SCHEME`: currently must be `http`
- `VPNNODE_BASELINE_REMOTE_EXPECT_READY_STATUS`: status the remote path must produce to count as usable; defaults to `200`
- `VPNNODE_BASELINE_REMOTE_HTTP_HOST`: `Host:` header for the smoke probe
- `VPNNODE_BASELINE_REMOTE_ROUTE_CACHE_PATH`: local cache file for the remote client run
- `VPNNODE_BASELINE_REMOTE_EXACT_ROUTE_ONLY`: defaults to `true`; disables adaptive shorter-route fallback so the requested hop-count is the hop-count that must actually work
- `VPNNODE_BASELINE_REMOTE_READY_TIMEOUT_SECS`: total readiness budget for each remote scenario
- `VPNNODE_BASELINE_REMOTE_PROBE_ATTEMPT_TIMEOUT_SECS`: timeout for one remote HTTP probe attempt
- `VPNNODE_BASELINE_REMOTE_RESET_CMD`: optional command run before each scenario to reset shared remote topology state

Remote safety checks:

- Remote topology values are loaded from environment, not test hardcode.
- `relay1`, `relay2`, and `exit` are rejected if they point to `127.0.0.1`, `localhost`, or `::1`.
- The remote runner prints the effective topology before it starts the test.
- The remote smoke harness rejects `VPNNODE_BASELINE_REMOTE_TARGET_SCHEME` values other than `http`, because the current client TCP-mode is an HTTP-shaped tunnel entrypoint, not a generic TLS forward proxy.
- With `VPNNODE_BASELINE_REMOTE_EXACT_ROUTE_ONLY=true`, the client is not allowed to silently downgrade a 3-hop request into a 2-hop or 1-hop path.

## Remote scenarios currently supported

The remote runner currently verifies these WAN-capable scenarios:

- `remote_http_1hop_returns_200`
- `remote_http_2hop_returns_200`
- `remote_http_3hop_returns_200`

The runner launches each scenario as a separate `cargo test` process. That isolation is deliberate: the current relay/exit runtime still keeps effectively single-session state, so process isolation avoids cross-scenario contamination while preserving a real WAN path.

For the current public topology, separate client processes are not enough by themselves. Relay and exit still retain effective single-session state across successive WAN scenarios. To run a 1-hop/2-hop/3-hop matrix honestly on shared public nodes, the runner supports an explicit reset hook:

```bash
export VPNNODE_BASELINE_REMOTE_RESET_CMD='powershell.exe -ExecutionPolicy Bypass -File scripts/reset_remote_topology.ps1'
```

That command is run before every scenario. It is not test fakery; it is an operational mitigation for the current remote runtime limitation described in `docs/wan_200_followup.md`.

## Remote target requirement

The remote matrix now uses option `A`: a plain-HTTP target that is expected to return `200 OK`.

For the current public topology, the required target is:

- on the exit host `31.192.232.26`
- local HTTP service `127.0.0.1:8080`
- managed by `vpnnode-target-http.service`

This requirement is deliberate. The earlier WAN `504` came from probing `github.com:443` with raw HTTP bytes. That was a target compatibility mismatch, not a valid smoke contract.

## Remote scenarios not yet automated

`target-unavailable -> 502` is still local-only for now.

Reason:

- it would require mutating the live exit target config to an intentionally unavailable address,
- then restarting the shared public exit node,
- then restoring the working config after the negative run.

That is operationally possible, but it is still intrusive and stateful because the current remote exit/replay/session behavior is not robust enough to treat those mutations as a cheap, side-effect-free matrix step. For now, the negative path remains deterministic only in the local suite.

## Readiness semantics

`tests/helpers/mod.rs` now separates three concepts:

- `wait_tcp_listener()`: transport-only listener readiness
- `probe_http()`: one explicit HTTP probe attempt
- `wait_http_ready()` / `wait_http_status_with_request()`: usable HTTP readiness

The important behavioral change is that HTTP readiness no longer succeeds on "any parseable status". The default `wait_http_ready()` path now accepts only `200 OK`. Tests that intentionally validate an error path, such as `baseline_target_unavailable_returns_502`, must use the explicit status-based helper instead of the generic ready check.

## What is intentionally still local-only

These scenarios remain in the local suite because they depend on a controlled target or deterministic error injection:

- `baseline_large_transfer_ge_5mb`
- `baseline_parallel_streams_ge_5`
- `baseline_target_unavailable_returns_502`
- `baseline_small_response_returns_200_and_completes`
- `baseline_response_completion_respects_non_pathological_growth`

Running them "remotely" without controlling the exit target would be a false equivalence, not an honest baseline.

## Current remote topology used during investigation

The WAN smoke work in this branch used:

- `relay-1`: `45.197.133.115:30001`
- `relay-2`: `185.144.28.95:30002`
- `exit`: `31.192.232.26:30000`

The intended route chain is:

`client -> relay-1 -> relay-2 -> exit -> target`

See `docs/wan_504_rca.md` for the original `504` failure mode on that topology.
The captured remote smoke failure log is stored at `docs/artifacts/remote_wan_smoke_2026-03-20.log`.
The captured remote smoke pass log is stored at `docs/artifacts/remote_wan_smoke_2026-03-20_pass.log`.
The pass-run evidence bundle is stored at `docs/artifacts/wan_2026-03-20_pass_evidence.md`.
The expanded 1-hop/2-hop/3-hop matrix log is stored at `docs/artifacts/remote_wan_matrix_2026-03-20.log`.
The expanded matrix evidence bundle is stored at `docs/artifacts/wan_matrix_2026-03-20_evidence.md`.
Focused relay/exit excerpts for that pass are stored at:

- `docs/artifacts/relay1_remote_matrix_2026-03-20.log`
- `docs/artifacts/relay2_remote_matrix_2026-03-20.log`
- `docs/artifacts/exit_remote_matrix_2026-03-20.log`
