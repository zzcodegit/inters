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
