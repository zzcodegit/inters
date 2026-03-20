## Ops — Full public 3-hop WAN path

Goal:

`client (local) → relay #1 (public) → relay #2 (public) → exit (public) → target`

Nodes:

- **relay #1**: `155.212.135.95:30001/udp`
- **relay #2**: `155.212.135.200:30002/udp`
- **exit**: `155.212.143.112:30000/udp`
- **target** (for deterministic smoke): runs on the exit host as `127.0.0.1:8080`

Why target is local-to-exit:

- Avoids variability from public HTTP targets (Host header, CDNs, throttling).
- Still validates the WAN dataplane: UDP traversal across **two public relays** to a public exit.

---

## NAT limitation / why relays must be public

This stage is UDP-only with no NAT traversal. The reverse path relies on addresses in the Route and observed UDP endpoints.

If any relay hop is a private address (e.g. `192.168.x.x`), remote nodes cannot route packets back to it, and the handshake/streams will fail.

Therefore, for multi-hop across the internet: **every relay hop must be publicly reachable**.

---

## Remote exit (already used)

On `155.212.143.112`:

- config: `/root/vpnnode-exit.toml` (points to `exit_target_addr = "127.0.0.1:8080"`)
- start stack (target + exit):

```bash
killall vpnnode 2>/dev/null || true
nohup env RUST_LOG=info /root/vpnnode/target/release/vpnnode target --listen 127.0.0.1:8080 > /root/vpnnode-target.log 2>&1 &
nohup env RUST_LOG=info /root/vpnnode/target/release/vpnnode run --config /root/vpnnode-exit.toml > /root/vpnnode-exit.log 2>&1 &
ss -ltnp | grep 8080
ss -lunp | grep 30000
tail -n 30 /root/vpnnode-exit.log
```

Expected:

- `node ready role="exit" addr=0.0.0.0:30000`

---

## Remote relay #2 (new)

On `155.212.135.200`:

- UDP: `30002/udp` open
- config: `/root/vpnnode-relay2.toml` (peer = exit)

```bash
/root/vpnnode/target/release/vpnnode check --config /root/vpnnode-relay2.toml
/root/vpnnode/target/release/vpnnode health --config /root/vpnnode-relay2.toml
nohup env RUST_LOG=info /root/vpnnode/target/release/vpnnode run --config /root/vpnnode-relay2.toml > /root/vpnnode-relay2.log 2>&1 &
ss -lunp | grep 30002
tail -n 30 /root/vpnnode-relay2.log
```

Expected:

- `node ready role="relay" addr=155.212.135.200:30002`

---

## Remote relay #1 (updated)

On `155.212.135.95`:

- config: `/root/vpnnode-relay.toml` (peer = relay #2)

```bash
/root/vpnnode/target/release/vpnnode check --config /root/vpnnode-relay.toml
/root/vpnnode/target/release/vpnnode health --config /root/vpnnode-relay.toml
killall vpnnode 2>/dev/null || true
nohup env RUST_LOG=info /root/vpnnode/target/release/vpnnode run --config /root/vpnnode-relay.toml > /root/vpnnode-relay.log 2>&1 &
ss -lunp | grep 30001
tail -n 30 /root/vpnnode-relay.log
```

Expected:

- `node ready role="relay" addr=155.212.135.95:30001`

---

## Local client

Use config: `examples/config/client-full-public-3hop.toml`

```powershell
cd C:\neinternet\vpnnode
.\target\release\vpnnode.exe run --config .\examples\config\client-full-public-3hop.toml
```

Smoke:

```powershell
curl.exe -v --max-time 10 http://127.0.0.1:10080/
```

Expected:

- `HTTP/1.1 200 OK`
- body is an echo of the request (from remote target through the whole 3-hop chain)

---

## Quick troubleshooting

- **relay #1 not reachable**: `ss -lunp | grep 30001`, firewall, bind_ip must be `155.212.135.95`
- **relay #2 not reachable**: `ss -lunp | grep 30002`, firewall, bind_ip must be `155.212.135.200`
- **exit in bad state**: restart exit stack (exit currently is single-session oriented)
- **handshake timeouts**: verify the route has only public IPs and correct peer ordering

