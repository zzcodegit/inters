# WAN remote smoke follow-up: 200 OK

## What changed

The original WAN `504` was caused by a target mismatch:

- the remote smoke probe sent plain HTTP
- the exit was configured for `github.com:443`
- the exit therefore produced `resp_bytes=0`

For the current remote smoke contract, the fix is explicit option `A`:

- remote smoke requires `VPNNODE_BASELINE_REMOTE_TARGET_SCHEME=http`
- the exit must point to a plain-HTTP target that is expected to return `200 OK`

Option `B` was intentionally not used here. A direct HTTPS smoke against `github.com:443` would require the client-side entrypoint to behave like a generic raw TCP forwarder for TLS bytes, but the current TCP-mode client still buffers an HTTP-shaped request before opening the tunnel stream.

The public exit host now satisfies that contract with:

- exit host: `31.192.232.26`
- target: `127.0.0.1:8080`
- service: `vpnnode-target-http.service`

## Why this is a real fix

Nothing is hidden behind localhost emulation on the client side. The route is still:

`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> 127.0.0.1:8080`

What changed is only the target compatibility at the exit.

That means:

- the public relays and exit are still used
- the exit still performs the remote TCP connect
- the target now speaks the same protocol as the smoke probe

## Evidence

- original fail: `docs/artifacts/remote_wan_smoke_2026-03-20.log`
- pass run: `docs/artifacts/remote_wan_smoke_2026-03-20_pass.log`
- config/evidence snapshot: `docs/artifacts/wan_2026-03-20_pass_evidence.md`
- expanded matrix: `docs/artifacts/remote_wan_matrix_2026-03-20.log`
- expanded matrix evidence: `docs/artifacts/wan_matrix_2026-03-20_evidence.md`

## What the expanded 1-hop/2-hop/3-hop matrix exposed

Once the exit target mismatch was fixed, the next honest remote check was a sequential WAN matrix over the same shared public nodes.

That matrix exposed a second issue: relay/exit still behave as effectively single-session processes.

In particular, `src/roles/exit.rs` stores one shared reverse-path peer and route:

- one `session_crypto`
- one `session_route`
- one `session_peer`

That is enough for a single smoke path, but not enough for back-to-back WAN scenarios that change hop-count. After a successful 1-hop run, a later 2-hop run can still reach the exit and produce `HTTP 200`, yet the reverse traffic can be addressed using stale peer/route state from the previous session.

Operationally, that produced this pattern:

- client handshake succeeds
- exit receives request bytes and logs `http_code=200`
- client eventually times out with `504`

This was not another fake acceptance problem. It was a real dataplane limitation on the shared public topology.

## Current honest mitigation

The remote runner now supports an explicit topology reset hook before each scenario:

- `VPNNODE_BASELINE_REMOTE_RESET_CMD`
- example implementation: `scripts/reset_remote_topology.ps1`

The reset restarts `vpnnode-relay1.service`, `vpnnode-relay2.service`, and `vpnnode-exit.service` before each scenario. That clears stale single-session state and allows the same real WAN topology to pass 1-hop, 2-hop, and 3-hop verification as three independent remote scenarios.

This is an operational workaround, not a claim that the runtime is already multi-session safe.

The committed pass artifacts for that matrix are:

- `docs/artifacts/remote_wan_matrix_2026-03-20.log`
- `docs/artifacts/wan_matrix_2026-03-20_evidence.md`
- `docs/artifacts/relay1_remote_matrix_2026-03-20.log`
- `docs/artifacts/relay2_remote_matrix_2026-03-20.log`
- `docs/artifacts/exit_remote_matrix_2026-03-20.log`
