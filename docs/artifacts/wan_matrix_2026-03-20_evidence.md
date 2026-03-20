# Remote WAN matrix evidence (2026-03-20)

## Topology

- client public IP seen by relays/exit: `46.242.13.60`
- relay-1 London: `45.197.133.115:30001`
- relay-2 Warsaw: `185.144.28.95:30002`
- exit Los Angeles: `31.192.232.26:30000`
- exit target: `127.0.0.1:8080` on the exit host

The remote runner printed the WAN topology up front and all three scenarios ran with `VPNNODE_BASELINE_REMOTE_EXACT_ROUTE_ONLY=true`, so the requested hop-count was not allowed to silently downgrade.

The only loopback address in these runs was the client's local TCP listener (`127.0.0.1:19081-19083`). That listener is the local ingress into the client process, not part of the WAN route. Remote hops were public IPs only.

## Matrix result

- matrix log: `docs/artifacts/remote_wan_matrix_2026-03-20.log`
- total wall-clock window: `2026-03-21T00:03:55+03:00` -> `2026-03-21T00:17:24+03:00`
- scenario results:
  - `1-hop`: `HTTP 200`, `duration_ms=1793`
  - `2-hop`: `HTTP 200`, `duration_ms=752`
  - `3-hop`: `HTTP 200`, `duration_ms=1259`

## 1-hop proof

Runner:

```text
remote_scenario=1hop route_length=1 ... route_chain=client -> 31.192.232.26:30000 -> target
remote_scenario=1hop observed_status=200 response_bytes=188 duration_ms=1793
```

Exit:

```text
2026-03-20T21:08:47.808871Z ... session_id=1 peer=Udp://46.242.13.60:10787
2026-03-20T21:08:48.064174Z ... route_len=1
2026-03-20T21:08:48.221491Z ... VALIDATION_ARTIFACT_FINAL,stream_id=2,site=remote-1hop,...,route=1,...,http_code=200
```

Interpretation:

- exit peer is the client public IP directly
- no relay IP appears as the upstream peer
- final artifact confirms `route=1` and `http_code=200`

## 2-hop proof

Runner:

```text
remote_scenario=2hop route_length=2 ... route_chain=client -> 45.197.133.115:30001 -> 31.192.232.26:30000 -> target
remote_scenario=2hop observed_status=200 response_bytes=188 duration_ms=752
```

Relay-1:

```text
2026-03-20T21:13:43.902297Z ... from=Udp://46.242.13.60:2101 to=Udp://31.192.232.26:30000
2026-03-20T21:13:44.034161Z ... from=Udp://31.192.232.26:30000 to=Udp://46.242.13.60:2101
```

Exit:

```text
2026-03-20T21:13:47.427725Z ... session_id=1 peer=Udp://45.197.133.115:30001
2026-03-20T21:13:47.714694Z ... route_len=2
2026-03-20T21:13:47.871709Z ... VALIDATION_ARTIFACT_FINAL,stream_id=2,site=remote-2hop,...,route=2,...,http_code=200
```

Interpretation:

- relay-1 saw traffic from the client public IP to the exit public IP
- exit peer is relay-1, not the client
- final artifact confirms `route=2` and `http_code=200`

## 3-hop proof

Runner:

```text
remote_scenario=3hop route_length=3 ... route_chain=client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target
remote_scenario=3hop observed_status=200 response_bytes=188 duration_ms=1259
```

Relay-1:

```text
2026-03-20T21:17:21.232525Z ... from=Udp://46.242.13.60:2055 to=Udp://185.144.28.95:30002
2026-03-20T21:17:21.411839Z ... from=Udp://185.144.28.95:30002 to=Udp://46.242.13.60:2055
```

Relay-2:

```text
2026-03-20T21:17:24.498845Z ... from=Udp://45.197.133.115:30001 to=Udp://31.192.232.26:30000
2026-03-20T21:17:24.649106Z ... from=Udp://31.192.232.26:30000 to=Udp://45.197.133.115:30001
```

Exit:

```text
2026-03-20T21:17:24.850976Z ... session_id=1 peer=Udp://185.144.28.95:30002
2026-03-20T21:17:25.271253Z ... route_len=3
2026-03-20T21:17:25.738996Z ... VALIDATION_ARTIFACT_FINAL,stream_id=2,site=remote-3hop,...,route=3,...,http_code=200
```

Interpretation:

- relay-1 forwarded client traffic to relay-2
- relay-2 forwarded relay-1 traffic to exit
- exit peer is relay-2
- final artifact confirms `route=3` and `http_code=200`

## Supporting raw artifacts

- runner log: `docs/artifacts/remote_wan_matrix_2026-03-20.log`
- relay-1 excerpt log: `docs/artifacts/relay1_remote_matrix_2026-03-20.log`
- relay-2 excerpt log: `docs/artifacts/relay2_remote_matrix_2026-03-20.log`
- exit excerpt log: `docs/artifacts/exit_remote_matrix_2026-03-20.log`

## Notes

- The repeated `SUSPICIOUSLY_SMALL_RESPONSE` warnings are expected here because the smoke target returns a small `188`-byte HTTP response. The warning is heuristic noise, not a failing condition.
- The remote matrix still uses a topology reset between scenarios because relay/exit currently retain effectively single-session state. That is documented in `docs/wan_200_followup.md`.
