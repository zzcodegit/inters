## Ops inventory — public vpnnode hosts

This file summarizes the current known public nodes and how they map to the deploy scripts.

---

## Standard layout

All nodes:

- binary: `/opt/vpnnode/bin/vpnnode`
- configs: `/etc/vpnnode/*.toml`
- state: `/var/lib/vpnnode/`
- units: `/etc/systemd/system/vpnnode-*.service`

---

## EXIT

- role: `exit`
- host: `root@155.212.143.112`
- UDP port: `30000`
- config: `/etc/vpnnode/exit.toml`
- unit: `vpnnode-exit.service`
- local config template: `examples/config/exit-remote-with-target.toml`
- local unit template: `systemd/vpnnode-exit.service`

Scripts:

```bash
scripts/deploy_node.sh exit
scripts/restart_node.sh exit
scripts/check_node.sh exit
scripts/tail_node_logs.sh exit 100
```

---

## RELAY #1

- role: `relay1`
- host: `root@155.212.135.95`
- UDP port: `30001`
- config: `/etc/vpnnode/relay1.toml`
- unit: `vpnnode-relay1.service`
- local config template: `examples/config/relay-remote-1-to-relay2.toml`
- local unit template: `systemd/vpnnode-relay1.service`

Scripts:

```bash
scripts/deploy_node.sh relay1
scripts/restart_node.sh relay1
scripts/check_node.sh relay1
scripts/tail_node_logs.sh relay1 100
```

---

## RELAY #2

- role: `relay2`
- host: `root@155.212.135.200`
- UDP port: `30002`
- config: `/etc/vpnnode/relay2.toml`
- unit: `vpnnode-relay2.service`
- local config template: `examples/config/relay-remote-2.toml`
- local unit template: `systemd/vpnnode-relay2.service`

Scripts:

```bash
scripts/deploy_node.sh relay2
scripts/restart_node.sh relay2
scripts/check_node.sh relay2
scripts/tail_node_logs.sh relay2 100
```

