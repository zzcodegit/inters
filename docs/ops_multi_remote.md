## Ops — Multi-remote topology (client → local relay → remote relay → remote exit)

Goal topology:

`client (local) → relay (local) → relay (remote) → exit (remote) → target`

Remote nodes used:

- **remote relay**: `155.212.135.95:30001/udp`
- **remote exit**: `155.212.143.112:30000/udp` (with remote target on `127.0.0.1:8080`)

---

## What works / what blocks (important)

### Forward direction works

Local relay successfully forwards packets to the remote relay, and the remote relay forwards them to the remote exit.

### Reverse direction is blocked by NAT (no dataplane changes allowed)

In this project’s current Stage (UDP-only, no NAT traversal), the reverse path uses the **addresses embedded in the Route hops**.

If your “local relay” hop address is a **private LAN IP** (e.g. `192.168.x.x`), remote servers will try to send the return packets to that private IP, which is **not routable** from the internet.

Symptoms:

- client handshake timeouts (no `HandshakeChallenge` returned)
- remote relay logs show reverse forwarding to `192.168.x.x:30000`

This is expected without one of:

- a public IP directly on the local relay host, or
- explicit UDP port-forwarding + using the public IP in the route hop, **and** the relay must be able to bind/advertise that public IP (not possible behind typical NAT), or
- a dataplane change (public address advertisement / NAT traversal), which is out of scope for this ops step.

---

## Deployment steps (remote relay)

### 1) SSH and inspect

```bash
uname -a
ip a
ss -lun
```

### 2) Open UDP port

```bash
ufw allow 30001/udp || true
iptables -I INPUT -p udp --dport 30001 -j ACCEPT || true
```

### 3) Build and run

Build `vpnnode` on the remote relay server and create:

- `/root/vpnnode-relay.toml` (example in `examples/config/relay-remote.toml`)

Validate:

```bash
/root/vpnnode/target/release/vpnnode check --config /root/vpnnode-relay.toml
/root/vpnnode/target/release/vpnnode health --config /root/vpnnode-relay.toml
```

Run:

```bash
nohup env RUST_LOG=info /root/vpnnode/target/release/vpnnode run --config /root/vpnnode-relay.toml > /root/vpnnode-relay.log 2>&1 &
ss -lunp | grep 30001
tail -n 50 /root/vpnnode-relay.log
```

Expected:

- `node ready role="relay" addr=155.212.135.95:30001`

---

## Configs in repo

- `examples/config/relay-remote.toml`: remote relay → remote exit
- `examples/config/relay-public-to-remote-relay.toml`: local relay → remote relay (requires local relay to be internet-reachable for reverse path)
- `examples/config/client-public-2relays.toml`: 3-hop route seed

---

## How to actually make the full WAN chain pass

Because of the reverse-path NAT issue, to pass the full chain **without changing dataplane** you must ensure hop#1 (“local relay”) is a node with an internet-routable address.

Practical options:

- **Option A (recommended for now)**: run the “first relay” on a small VPS (public IP) instead of a home LAN host.
  - Then the topology becomes: `client(local) → relay(vps1) → relay(vps2) → exit(vps3)`
- **Option B**: place the “local relay” host directly on a public IP (no NAT), and use that public IP in the hop list.

Once hop#1 is publicly reachable, the exact same configs and routing logic should work end-to-end.

