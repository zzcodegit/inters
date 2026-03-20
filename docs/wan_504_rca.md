# WAN 504 root cause analysis

## Scope

This document explains why the current real WAN path returns `HTTP 504 Gateway Timeout` even though the remote route can complete the session handshake.

Topology under test:

- `relay-1`: `45.197.133.115:30001` (London)
- `relay-2`: `185.144.28.95:30002` (Warsaw)
- `exit`: `31.192.232.26:30000` (Los Angeles)
- client local TCP listener: `127.0.0.1:10081`

Expected route:

`client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`

## Established facts

### 1. Control-plane/session establishment is real

The client reached the remote chain and established a session with the exit:

```text
2026-03-20T19:21:12.105034Z INFO vpnnode::roles::client: client handshake: session established with exit (Stage 3.1 challenge flow)
```

Relay logs also show forwarding across the real public IP path:

```text
relay-1: from=Udp://46.242.13.60:2061 to=Udp://185.144.28.95:30002
relay-2: from=Udp://45.197.133.115:30001 to=Udp://31.192.232.26:30000
```

That rules out "the test accidentally stayed on localhost".

### 2. After a clean restart, the exit receives request data

The exit accepted the session and read the request body/end marker:

```text
2026-03-20T19:21:13.245985Z INFO vpnnode::roles::exit: exit: appended request bytes from client stream_id=1000000002 appended=50 total=50
2026-03-20T19:21:13.246148Z INFO vpnnode::roles::exit: exit: received end-of-request marker from client stream_id=1000000002
```

That means the dataplane is not failing before the exit. The request really arrives there.

### 3. The exit is currently configured with a target that does not match baseline HTTP semantics

Live remote config on the exit host:

```toml
role = "exit"
bind_ip = "0.0.0.0"
bind_port = 30000
exit_target_addr = "github.com:443"
```

The baseline client sends plain HTTP over the tunnel. The current exit target is `github.com:443`, which expects TLS, not raw HTTP.

### 4. The exit produces zero response bytes for the request

The exit-side validation log proves that no application response bytes came back from the configured target:

```text
2026-03-20T19:21:37.317214Z INFO vpnnode::roles::exit: VALIDATION_ARTIFACT_FINAL,stream_id=1,site=127.0.0.1,exit=Udp://31.192.232.26:30000,route=1,resp_bytes=0,frames_sent=0,...,http_code=0,is_final=1,mode=overlay
```

This is consistent with plain HTTP being sent to a TLS port.

### 5. The client falls back to synthesized 504 after route attempts see no usable response

Client-side result from the same run:

```text
2026-03-20T19:21:36.912379Z INFO vpnnode::roles::client: route failed, switching stream_id=1 attempt=0 hops=3 ... e=no response (timeout or channel closed)
2026-03-20T19:21:37.154205Z INFO vpnnode::roles::client: route failed, switching stream_id=1 attempt=1 hops=1 ... e=no response (timeout or channel closed)
2026-03-20T19:21:37.396918Z INFO vpnnode::roles::client: route failed, switching stream_id=1 attempt=2 hops=2 ... e=no response (timeout or channel closed)
2026-03-20T19:21:37.397233Z ERROR vpnnode::roles::client: client: all routes failed stream_id=1
```

The resulting probe observed:

```text
HTTP/1.1 504 Gateway Timeout
```

## Root cause

For the clean WAN run, the root cause is:

- control-plane OK
- relays forward to the exit
- exit receives the request
- target path is misconfigured for the baseline probe
- therefore no response bytes are produced
- client times out and synthesizes `504 Gateway Timeout`

Short version:

`504` is currently caused by a target-path mismatch, not by localhost emulation and not by simple relay reachability loss.

## Secondary risk observed during investigation

Before the remote stack was force-restarted, older runs also showed exit-side decrypt/replay errors:

```text
exit failed to open message e=decrypt error: aead::Error
exit failed to open message e=packet too old for replay window
```

That points to a separate operational/runtime weakness:

- relay/exit hold effectively single-session state
- stale client traffic or restart timing can poison the active session/replay window

This is a real risk, but it is not needed to explain the clean-run `504`. The clean-run failure is already reproducible from the `github.com:443` target mismatch alone.

## Why the new readiness rules matter

The old helper considered the path "ready" as soon as any parseable HTTP status appeared. On the current WAN topology that would incorrectly treat a `504` as success.

The new test helpers fix this by separating:

- listener readiness
- one-shot probe
- usable HTTP readiness with explicit accepted statuses

With that change, the current WAN path is reported honestly as unusable for the baseline HTTP smoke.

## What would make the remote smoke pass

One of the following must be true:

1. The exit must point at a real plain-HTTP target, such as a controlled `host:80` service.
2. The client/exit path must learn how to speak the target protocol being used at the exit, for example TLS if the target stays on `:443`.

Until one of those is done, a strict remote baseline that expects `200 OK` should fail.

## Operational access vs. code-only proof

Proven with code and local artifacts:

- the previous baseline suite was localhost-only
- the old readiness semantics were too weak
- the client synthesizes `504` after route exhaustion

Proven only because real infra access was available:

- the WAN relays/exits were real public IPs
- the exit was configured with `github.com:443`
- the exit received the request but emitted `resp_bytes=0`

Supporting artifacts committed with this branch:

- `docs/artifacts/wan_2026-03-20_evidence.md`
- `docs/artifacts/remote_wan_smoke_2026-03-20.log`
