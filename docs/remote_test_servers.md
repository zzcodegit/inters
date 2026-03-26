# Remote Test Servers

The shared remote test-server bootstrap lives in:

- [examples/config/remote_test_servers.ps1](/C:/neinternet/vpnnode_stage5_baseline_v2_exact_route/examples/config/remote_test_servers.ps1)

Load it into the current PowerShell session before remote validation:

```powershell
. .\examples\config\remote_test_servers.ps1
```

For a fresh exact-route baseline v2 run, provision the relay fleet, deploy the binary, then hard-reset the topology:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/provision_remote_relays.ps1
powershell -ExecutionPolicy Bypass -File scripts/deploy_remote_binary.ps1
powershell -ExecutionPolicy Bypass -File scripts/reset_remote_topology_singleline.ps1
```

The bootstrap exports:

- remote host/user/password env vars for:
  - `31.192.232.26`
  - `45.197.133.115`
  - `185.144.28.95`
  - `155.212.135.200`
  - `155.212.135.95`
  - `155.212.143.112`
- `VPNNODE_REMOTE_RESET_SETTLE_SECS=15`
- `VPNNODE_BASELINE_REMOTE_RESET_CMD=powershell.exe -ExecutionPolicy Bypass -File scripts/reset_remote_topology_singleline.ps1`

Current provisioning contract for baseline v2:

- every relay config is provisioned with `peers = []`
- the relay forwards using the routed packet header
- the client-side route seed is the only thing that chooses `1-hop/2-hop/3-hop/5-hop`
- there is no static downstream relay chain in the remote configs

Default managed topology from the bootstrap:

- `relay1,relay2,relay3,relay4,relay5,relay6,exit`

Current active 5-hop chain used for exact-route validation:

- `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 155.212.135.200:30003 -> 155.212.135.95:30004 -> 31.192.232.26:30000 -> target`
