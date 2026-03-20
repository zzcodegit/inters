## Ops Step 2.5 — First Public Node (Remote Exit)

This runbook documents a minimal **manual** deployment of a public `exit` node on a remote Linux server, connected to a local `client → relay` setup.

Topology for the smoke test:

- **client (local Windows)** → **relay (local Windows)** → **exit (remote Linux)** → **target (remote Linux)**

The target is intentionally started on the same remote server to avoid issues with public HTTP targets that may drop connections on unexpected `Host` headers. This still validates the **real internet UDP path** between local relay and remote exit, and validates the full dataplane route end-to-end.

---

## Prereqs

- Remote server SSH access (root).
- Local machine has `plink.exe` / `pscp.exe` (PuTTY) available.
- `vpnnode` built locally (for local `client` + `relay`).

---

## Remote server setup

### 1) Connect and inspect

On local:

```powershell
& "C:\Program Files\PuTTY\plink.exe" -ssh root@155.212.143.112
```

On remote:

```bash
uname -a
ip a
ss -lun
```

### 2) Open UDP port for exit (example: 30000)

On remote (best-effort; depends on distro/firewall):

```bash
ufw allow 30000/udp || true
iptables -I INPUT -p udp --dport 30000 -j ACCEPT || true
```

### 3) Build `vpnnode` on the server

This repo is commonly built on the server to avoid cross-compiling from Windows.

Install Rust toolchain:

```bash
apt-get update -y || true
apt-get install -y curl build-essential pkg-config unzip || true
curl https://sh.rustup.rs -sSf | sh -s -- -y
. /root/.cargo/env
```

Copy sources to the server (one option):

- Create `tar.gz` on local from `Cargo.toml`, `src/`, `tests/`, `examples/`, etc.
- Upload to `/root/vpnnode-src.tar.gz`
- Extract into `/root/vpnnode`

Then build:

```bash
cd /root/vpnnode
cargo build --release
ls -la /root/vpnnode/target/release/vpnnode
```

### 4) Remote exit config

Create `/root/vpnnode-exit.toml`. Example used in this smoke:

```toml
role = "exit"
bind_ip = "0.0.0.0"
bind_port = 30000

exit_target_addr = "127.0.0.1:8080"

ants_enabled = false
drain_timeout_sec = 20
health_enabled = true
```

Validate:

```bash
/root/vpnnode/target/release/vpnnode check --config /root/vpnnode-exit.toml
/root/vpnnode/target/release/vpnnode health --config /root/vpnnode-exit.toml
# expected:
# HEALTH OK role=exit addr=0.0.0.0:30000
```

### 5) Start remote target + exit (nohup)

Start target HTTP echo service:

```bash
nohup env RUST_LOG=info /root/vpnnode/target/release/vpnnode target --listen 127.0.0.1:8080 > /root/vpnnode-target.log 2>&1 &
```

Start exit:

```bash
nohup env RUST_LOG=info /root/vpnnode/target/release/vpnnode run --config /root/vpnnode-exit.toml > /root/vpnnode-exit.log 2>&1 &
```

Verify:

```bash
ss -ltnp | grep 8080
ss -lunp | grep 30000
tail -n 50 /root/vpnnode-exit.log
```

Expected log line:

- `node ready role="exit" addr=0.0.0.0:30000`

---

## Local setup (Windows)

### 1) Pick the correct relay bind_ip

Important: relay compares its own address against the `Route` hops. If relay is bound to `0.0.0.0`, but the client route uses `127.0.0.1`, relay may drop traffic as “self address not present in route”.

For public-exit smoke we use a **real local interface IP** (example: `192.168.88.52`) so the relay can both:

- send UDP to the internet, and
- match itself inside the route.

### 2) Local configs

Use:

- `examples/config/relay-public-exit.toml`
- `examples/config/client-public-exit.toml`

These are pre-filled with:

- relay bind: `192.168.88.52:30000`
- exit peer: `155.212.143.112:30000`
- client local listener: `127.0.0.1:10080`

### 3) Start relay + client

In two terminals:

```powershell
cd C:\neinternet\vpnnode
.\target\release\vpnnode.exe run --config .\examples\config\relay-public-exit.toml
```

```powershell
cd C:\neinternet\vpnnode
.\target\release\vpnnode.exe run --config .\examples\config\client-public-exit.toml
```

Expected logs:

- relay: `node ready role="relay" addr=...:30000`
- client: `client handshake: session established with exit ...`
- client: `client listening for local tcp addr=127.0.0.1:10080`

---

## Smoke test

From local:

```powershell
curl.exe -v --max-time 10 http://127.0.0.1:10080/
```

Expected:

- HTTP `200 OK`
- body is an echo of the request (from remote `target`).

---

## Troubleshooting

### Remote exit not reachable

- On remote: `ss -lunp | grep 30000`
- Ensure firewall allows UDP `30000/udp`
- If a stale process holds the port: `ss -lunp` shows PID, then `kill -KILL <pid>`

### Client handshake timeouts

- Ensure local relay is bound to an IP that can route to the internet (not `127.0.0.1`)
- Ensure client route hop IP matches relay `bind_ip` exactly

### 504 Gateway Timeout

- Check remote `exit` logs for `target connect` or `read` errors:
  - `tail -n 100 /root/vpnnode-exit.log`
- Ensure remote target is running:
  - `ss -ltnp | grep 8080`
  - `tail -n 50 /root/vpnnode-target.log`

