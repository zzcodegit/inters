# Stage 5 Baseline v2 Validation

## Summary

- Baseline v2 removes the relay `peers[]` startup dependency and runs on exact-route infrastructure.
- Exact-route validation for `1-hop`, `2-hop`, `3-hop`, and `5-hop` succeeded with full reset before each isolated run.
- Relay startup is clean for `relay1`, `relay2`, `relay3`, `relay4`, `relay6`, and `exit`.
- `relay5` (`155.212.143.112`) remained inaccessible with the provided credential, so full-fleet startup proof is incomplete.
- Minimal perf snapshot is now complete for the required exact routes:
  - `1-hop`, `2-hop`, `3-hop`, and `5-hop` all have measured `total_time_ms`
  - `1-hop`, `2-hop`, `3-hop`, and `5-hop` all have measured large-payload `ack p95`
  - `1-hop`, `2-hop`, `3-hop`, and `5-hop` all have measured `retransmit_rate`

## Peers Dependency Removed

- `src/node_config.rs`: relay validation now allows `peers = []`
- `src/main.rs`: relay startup no longer dereferences `cfg.peers[0]`
- `src/ops/health.rs`: health path no longer injects relay peers
- `scripts/provision_remote_relays.ps1`: relays are provisioned with `peers = []`

## Route Correctness

Declared routes:

- `1-hop`: `client -> 31.192.232.26:30000 -> target`
- `2-hop`: `client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target`
- `3-hop`: `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target`
- `5-hop`: `client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 155.212.135.200:30003 -> 155.212.135.95:30004 -> 31.192.232.26:30000 -> target`

Exit peer proof:

- `1-hop`: exit peer `Udp://46.242.13.60:2051`
- `2-hop`: exit peer `Udp://45.197.133.115:30001`
- `3-hop`: exit peer `Udp://185.144.28.95:30002`
- `5-hop`: exit peer `Udp://155.212.135.95:30004`

Non-route relay proof after the last restart in each run:

- `1-hop`: `relay1/relay2/relay3/relay4/relay6` all `forward_lines=0`
- `2-hop`: only `relay1` active; `relay2/relay3/relay4/relay6` all `forward_lines=0`
- `3-hop`: only `relay1/relay2` active; `relay3/relay4/relay6` all `forward_lines=0`
- `5-hop`: `relay1/relay2/relay3/relay4` active; `relay6` `forward_lines=0`

## Relay Startup

Clean startup proof exists for:

- `exit`
- `relay1`
- `relay2`
- `relay3`
- `relay4`
- `relay6`

Observed on those nodes:

- `Active: active (running)`
- `loaded config role=Relay bind=... peers=0 ants=false`
- no restart loop
- no `relay requires at least one peer` error

Blocked node:

- `relay5` startup proof is missing because SSH auth to `155.212.143.112` failed with `Access denied`

## Minimal Perf Snapshot

### Exact-route measured totals

Large-payload stage snapshot (`runs=1`, exact-route only):

| Route | total_time_ms | ack p95 ms | retransmit_rate | window_wait_total_ms |
| --- | ---: | ---: | ---: | ---: |
| `1-hop` | `1106.41` | `179` | `0` | `0` |
| `2-hop` | `1101.85` | `189` | `0` | `29` |
| `3-hop` | `1728.74` | `246` | `0` | `5` |
| `5-hop` | `1995.45` | `335` | `0` | `91` |

Supporting multi-run end-to-end matrix:

| Route | total avg ms | total p95 ms | total max ms |
| --- | ---: | ---: | ---: |
| `1-hop` | `1111.37` | `1140.60` | `1140.60` |
| `2-hop` | `1112.65` | `1124.78` | `1124.78` |
| `3-hop` | `1475.43` | `1655.66` | `1655.66` |

Corroborating isolated `5-hop` payload run after exact-route warmup:

- `5-hop` measured request: `TIME_TOTAL=1.768523`, `HTTP_CODE=200`, `SIZE_DOWNLOAD=262144`

Observed shape:

- `1-hop` and `2-hop` are essentially tied
- latency rises materially at `3-hop`
- latency rises again at `5-hop`
- no route shows non-zero retransmit in the clean stage snapshot

## Local Compatibility Checks

- `relay_without_peers_is_valid_for_exact_route_forwarding`: passed
- `health_bind_conflict_fails`: passed

The PowerShell wrappers around those `cargo test` invocations emitted package-lock waiting noise to stderr, but the targeted tests themselves returned `ok` in the captured logs.

## Cleanup

- temporary public perf forwarder on the exit host was disabled after measurements
- `vpnnode-target-http-public.service` is `inactive`
- `vpnnode-target-http.service` remains `active`
- no local `cargo` / `vpnnode` processes remained after cleanup

## Verdict

`baseline v2 = INVALID`

Reason:

- exact-route compatibility itself is proven
- route correctness for `1/2/3/5-hop` is proven
- minimal perf snapshot for the required routes is now complete
- but the requested baseline is still not trustworthy as a Stage 5 reference because full-fleet startup proof is incomplete: `relay5` (`155.212.143.112`) remains inaccessible with the provided credential, so the requested "deploy on all nodes / show status for each relay" requirement is not satisfied
