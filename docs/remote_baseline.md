# Remote-capable baseline

## What changed

The repository now has two explicit baseline modes:

- `local` mode keeps the existing localhost integration suite and still exercises the full 9-test baseline matrix.
- `remote` mode is a separate WAN smoke test that takes topology from environment variables instead of hardcoded `127.0.0.1` addresses.

The split is intentional. The full local suite can deterministically control target behavior, port allocation, and failure modes. The remote suite is restricted to an honest smoke path that can be exercised against real relays/exits without pretending that localhost-only scenarios are now WAN-safe.

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

Run the WAN smoke test with:

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
- `VPNNODE_BASELINE_REMOTE_EXPECT_READY_STATUS`: status the remote path must produce to count as usable; defaults to `200`
- `VPNNODE_BASELINE_REMOTE_HTTP_HOST`: `Host:` header for the smoke probe
- `VPNNODE_BASELINE_REMOTE_ROUTE_CACHE_PATH`: local cache file for the remote client run

Remote safety checks:

- Remote topology values are loaded from environment, not test hardcode.
- `relay1`, `relay2`, and `exit` are rejected if they point to `127.0.0.1`, `localhost`, or `::1`.
- The remote runner prints the effective topology before it starts the test.

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

See `docs/wan_504_rca.md` for the current failure mode on that topology.
The captured remote smoke failure log is stored at `docs/artifacts/remote_wan_smoke_2026-03-20.log`.
