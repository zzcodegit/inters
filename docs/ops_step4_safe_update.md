## Ops Step 4 — Safe Update / Rolling Update

Goal: provide a **drain-aware, one-node-at-a-time update flow** for public `vpnnode` nodes, with minimal rollback support.

This step does **not** change dataplane behavior — only how binaries are updated and services are restarted.

---

## Standard node layout (recap)

All remote nodes (exit + relay1 + relay2) use:

- binary: `/opt/vpnnode/bin/vpnnode`
- config: `/etc/vpnnode/<role>.toml`
- state (reserved): `/var/lib/vpnnode/`
- systemd unit: `/etc/systemd/system/vpnnode-<role>.service`

Public nodes:

- EXIT: `root@155.212.143.112`, UDP `30000`, unit `vpnnode-exit.service`, config `/etc/vpnnode/exit.toml`
- RELAY #1: `root@155.212.135.95`, UDP `30001`, unit `vpnnode-relay1.service`, config `/etc/vpnnode/relay1.toml`
- RELAY #2: `root@155.212.135.200`, UDP `30002`, unit `vpnnode-relay2.service`, config `/etc/vpnnode/relay2.toml`

---

## Safe update flow (per node)

Standard sequence for updating a single node:

1. **Upload new binary** to a remote temp path.
2. Run `vpnnode check --config ...` on the remote to catch obvious config/startup issues.
3. Trigger **graceful drain**:
   - send `SIGTERM` to the systemd unit (`systemctl kill -s SIGTERM ...`).
   - wait for up to *N* seconds (drain timeout) for graceful stop.
   - if still active after timeout, run `systemctl stop`.
4. **Backup current binary**:
   - if `/opt/vpnnode/bin/vpnnode` exists, copy it to `/opt/vpnnode/bin/vpnnode.prev`.
5. **Replace binary** with the new one.
6. `systemctl start vpnnode-<role>`.
7. Run `vpnnode health --config ...` to verify startup.
8. Tail recent logs:
   - `journalctl -u vpnnode-<role> -n 50 --no-pager`.
9. Print clear success/failure summary.

No `kill -9`, no skipping health, no all-nodes-at-once restarts.

---

## Binary backup / rollback behavior

Before replacing the active binary, the update flow:

- copies `/opt/vpnnode/bin/vpnnode` → `/opt/vpnnode/bin/vpnnode.prev` (if present).

Rollback uses:

- `/opt/vpnnode/bin/vpnnode.prev` → `/opt/vpnnode/bin/vpnnode`
- then restarts the systemd unit and runs health.

This gives a minimal version safety net without building a full version manager.

---

## Scripts

All scripts live in `scripts/` and use `hosts.env` as inventory.

### `safe_update_node.sh`

Usage:

```bash
scripts/safe_update_node.sh {exit|relay1|relay2} [drain_timeout_sec]
```

Behavior:

1. Verifies local binary `target/release/vpnnode` exists.
2. Uploads it to remote `/tmp/vpnnode-bin-update-$$`.
3. On remote:
   - `vpnnode check --config /etc/vpnnode/<role>.toml`
   - `systemctl kill -s SIGTERM vpnnode-<role>.service`
   - waits up to `drain_timeout_sec` (default 30s) for unit to stop;
     if still active, runs `systemctl stop`.
   - copies `/opt/vpnnode/bin/vpnnode` → `/opt/vpnnode/bin/vpnnode.prev` (if exists).
   - moves new binary into `/opt/vpnnode/bin/vpnnode` and `chmod 0755`.
   - `systemctl start vpnnode-<role>.service`
   - `vpnnode health --config /etc/vpnnode/<role>.toml`
   - `journalctl -u vpnnode-<role>.service -n 50 --no-pager`
4. If any step fails, the script stops and prints:
   - which role failed
   - recommendation to run `scripts/rollback_node.sh <role>`.

### `rollback_node.sh`

Usage:

```bash
scripts/rollback_node.sh {exit|relay1|relay2}
```

Behavior (remote):

1. Checks that `/opt/vpnnode/bin/vpnnode.prev` exists and is executable.
2. `systemctl stop vpnnode-<role>.service`.
3. Copies `.prev` over `/opt/vpnnode/bin/vpnnode` and `chmod 0755`.
4. `systemctl start vpnnode-<role>.service`.
5. `vpnnode health --config /etc/vpnnode/<role>.toml`.
6. `journalctl -u vpnnode-<role>.service -n 50 --no-pager`.

If `.prev` is missing, rollback fails early with a clear error.

### `rolling_update.sh`

Usage:

```bash
scripts/rolling_update.sh [drain_timeout_sec]
```

Default drain timeout: `30` seconds.

**Rolling order** (hard-coded, minimal blast radius):

1. `relay2`
2. `relay1`
3. `exit`

Reasoning:

- relays are stateless blind forwarders → safe to update first.
- exit holds decrypt + target-connect responsibilities → update last.

Behavior:

1. For each role in order:

   ```bash
   scripts/safe_update_node.sh <role> <drain_timeout_sec>
   ```

2. On the **first failure**:
   - stop immediately,
   - print which role failed,
   - print suggested rollback command:

     ```bash
     scripts/rollback_node.sh <role>
     ```

3. If all succeed, prints final success summary.

---

## Systemd stop/restart behavior

The scripts rely on existing systemd units (`vpnnode-exit.service`, `vpnnode-relay1.service`, `vpnnode-relay2.service`), and on `vpnnode`’s **signal handling**:

- `SIGTERM` → activates drain mode (via existing ops/signal + ops/drain).

Safe stop sequence:

1. `systemctl kill -s SIGTERM vpnnode-<role>.service`
2. wait up to `drain_timeout_sec` for graceful stop
3. `systemctl stop` if still active

Then the binary is replaced and `systemctl start` is used to bring it back.

---

## Verification after update

For each node:

1. `vpnnode health --config /etc/vpnnode/<role>.toml`
2. `journalctl -u vpnnode-<role>.service -n 50 --no-pager`

Optional extra smoke from local client:

```bash
curl.exe -v --max-time 10 http://127.0.0.1:10080/
```

Expected: `HTTP/1.1 200 OK` with echo body, confirming:

`client(local) → relay1(remote) → relay2(remote) → exit(remote) → target`

---

## Typical operator workflows

### Update a single node safely

```bash
# build new binary locally
cargo build --release

# update one node, e.g. relay2
scripts/safe_update_node.sh relay2 45
```

If it fails:

```bash
scripts/rollback_node.sh relay2
```

### Rolling update of all public nodes

```bash
cargo build --release
scripts/rolling_update.sh 45   # order: relay2 -> relay1 -> exit
```

If it stops on a failure:

- read the message (which role failed),
- rollback that node:

```bash
scripts/rollback_node.sh <role>
```

---

## Known-good topology guarantee

The safe update layer:

- does **not** change:
  - routing semantics,
  - crypto/session logic,
  - packet formats,
  - relay/exit roles.
- only controls:
  - when a new binary is swapped in,
  - how services are drained and restarted,
  - how health is verified per node.

The full public 3-hop topology:

`client(local) → relay1(remote) → relay2(remote) → exit(remote) → target`

remains the baseline and should be re-smoke-tested after rolling updates to validate end-to-end behavior.

