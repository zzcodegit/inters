## Ops Step 3 — systemd + auto-deploy foundation

This document describes how to run `vpnnode` nodes as **systemd services** with a simple, repeatable deploy flow.

Scope:

- single EXIТ node + two public RELAY nodes
- no dataplane changes, no new transports
- pure ops layer: filesystem layout, systemd units, SSH-based deploy scripts

---

## Standard filesystem layout (remote hosts)

All remote nodes (exit + relays) use the same layout:

- **binary**: `/opt/vpnnode/bin/vpnnode`
- **configs**: `/etc/vpnnode/*.toml`
- **state (reserved)**: `/var/lib/vpnnode/`
- **systemd units**: `/etc/systemd/system/vpnnode-*.service`

This is enforced by:

- `scripts/hosts.env` — inventory of hosts/paths
- `scripts/deploy_node.sh` — installs binary, config, and service unit

---

## Public nodes inventory (current)

Used in scripts and examples:

- **EXIT**
  - host: `root@155.212.143.112`
  - UDP: `30000`
  - config: `/etc/vpnnode/exit.toml`
  - unit: `vpnnode-exit.service`

- **RELAY #1**
  - host: `root@155.212.135.95`
  - UDP: `30001`
  - config: `/etc/vpnnode/relay1.toml`
  - unit: `vpnnode-relay1.service`

- **RELAY #2**
  - host: `root@155.212.135.200`
  - UDP: `30002`
  - config: `/etc/vpnnode/relay2.toml`
  - unit: `vpnnode-relay2.service`

The corresponding role-specific entries live in `scripts/hosts.env`.

---

## Systemd units (templates in repo)

Templates live under `systemd/`:

- `systemd/vpnnode-exit.service`
- `systemd/vpnnode-relay1.service`
- `systemd/vpnnode-relay2.service`

All use the standard layout:

- `ExecStart=/opt/vpnnode/bin/vpnnode run --config /etc/vpnnode/<role>.toml`
- `Restart=on-failure`
- `RestartSec=5`
- `Environment=RUST_LOG=info`

Examples of useful commands (on remote host):

```bash
systemctl start vpnnode-exit
systemctl restart vpnnode-relay1
systemctl status vpnnode-relay2
journalctl -u vpnnode-exit -f
```

---

## Scripts overview

Location: `scripts/`

- `hosts.env`
  - single source of truth for:
    - host IP
    - ssh user
    - role
    - remote config path
    - unit name
    - UDP port (for reference)
  - used by all other scripts via `source`.

- `deploy_node.sh`
  - Usage: `scripts/deploy_node.sh {exit|relay1|relay2}`
  - Behavior:
    1. Verifies local binary (`target/release/vpnnode`) and config template exist.
    2. Copies:
       - binary → remote `/tmp/vpnnode-bin.$PID`
       - config → remote `/tmp/vpnnode-config-...`
       - unit template → remote `/tmp/<unit>.service`
    3. On remote:
       - creates `/opt/vpnnode/bin`, `/etc/vpnnode`, `/var/lib/vpnnode`
       - moves binary to `/opt/vpnnode/bin/vpnnode` (0755)
       - moves config to `/etc/vpnnode/<role>.toml` (0644)
       - installs unit as `/etc/systemd/system/<unit>.service` (0644)
       - runs `systemctl daemon-reload`
       - enables unit (`systemctl enable`)
       - runs `vpnnode check --config ...`
       - restarts unit
       - runs `vpnnode health --config ...`

- `restart_node.sh`
  - Usage: `scripts/restart_node.sh {exit|relay1|relay2}`
  - On remote:
    - `vpnnode check --config ...`
    - `systemctl restart vpnnode-...`
    - `vpnnode health --config ...`

- `check_node.sh`
  - Usage: `scripts/check_node.sh {exit|relay1|relay2}`
  - On remote:
    - `vpnnode check --config ...`
    - `vpnnode health --config ...`

- `tail_node_logs.sh`
  - Usage: `scripts/tail_node_logs.sh {exit|relay1|relay2} [lines]`
  - On remote:
    - `journalctl -u vpnnode-... -n <lines> --no-pager`

All scripts are plain bash + ssh/scp; no external deploy tools.

---

## Standard operator flow (per node)

After updating binary/configs for a node:

1. **Deploy/update**

   ```bash
   # from repo root
   cargo build --release
   scripts/deploy_node.sh exit      # or relay1 / relay2
   ```

2. **Verify via systemd**

   On remote:

   ```bash
   systemctl status vpnnode-exit
   journalctl -u vpnnode-exit -n 50
   ```

3. **Standard verification sequence (per node)**

   ```bash
   vpnnode check --config /etc/vpnnode/<role>.toml
   systemctl restart vpnnode-<role>
   vpnnode health --config /etc/vpnnode/<role>.toml
   journalctl -u vpnnode-<role> -n 50
   ```

   This flow is mirrored by `restart_node.sh` and `check_node.sh`.

---

## Example: exit node

Deploy / update:

```bash
cargo build --release
scripts/deploy_node.sh exit
```

Remote checks:

```bash
ssh root@155.212.143.112
systemctl status vpnnode-exit
journalctl -u vpnnode-exit -n 50
/opt/vpnnode/bin/vpnnode check --config /etc/vpnnode/exit.toml
/opt/vpnnode/bin/vpnnode health --config /etc/vpnnode/exit.toml
```

---

## Example: relay #1 and relay #2

Relay #1:

```bash
scripts/deploy_node.sh relay1
scripts/restart_node.sh relay1
scripts/tail_node_logs.sh relay1 80
```

Relay #2:

```bash
scripts/deploy_node.sh relay2
scripts/restart_node.sh relay2
scripts/tail_node_logs.sh relay2 80
```

On each remote:

```bash
systemctl status vpnnode-relay1
systemctl status vpnnode-relay2
```

---

## Preserving the known-good 3-hop topology

The following configs (already in `examples/config/`) are used by deploy scripts:

- `exit-remote-with-target.toml` → `/etc/vpnnode/exit.toml`
- `relay-remote-1-to-relay2.toml` → `/etc/vpnnode/relay1.toml`
- `relay-remote-2.toml` → `/etc/vpnnode/relay2.toml`

These preserve the full public chain:

`client(local) → relay1(remote) → relay2(remote) → exit(remote) → target`

The dataplane, routing semantics, crypto, and packet formats are **not** changed by Ops Step 3 — we only standardize how nodes are run and updated.

