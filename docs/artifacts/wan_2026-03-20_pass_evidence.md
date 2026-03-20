# WAN pass evidence snapshot (2026-03-20)

## Exit config used for the pass run

```toml
role = "exit"
bind_ip = "0.0.0.0"
bind_port = 30000

# Remote target HTTP service (runs on the same server for the public-node smoke).
exit_target_addr = "127.0.0.1:8080"

ants_enabled = false
drain_timeout_sec = 20
health_enabled = true
```

## Remote target itself answers 200 on the exit host

```text
HTTP/1.0 200 OK
Server: SimpleHTTP/0.6 Python/3.10.6
```

## Topology used by the pass run

```text
client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> 127.0.0.1:8080
```

## Relay and exit evidence

```text
relay-1  2026-03-20T20:00:14.336618Z from=Udp://46.242.13.60:10828 to=Udp://185.144.28.95:30002
relay-2  2026-03-20T20:00:17.564220Z from=Udp://45.197.133.115:30001 to=Udp://31.192.232.26:30000
exit     2026-03-20T19:59:36.106837Z target_addr=127.0.0.1:8080
exit     2026-03-20T20:00:18.755288Z appended request bytes from client stream_id=1000000002 appended=50 total=50
exit     2026-03-20T20:00:18.755493Z received end-of-request marker from client stream_id=1000000002
exit     2026-03-20T20:00:18.839373Z VALIDATION_ARTIFACT_FINAL,stream_id=2,site=example,exit=Udp://31.192.232.26:30000,...,http_code=200,is_final=1,mode=overlay
```

## Remote runner outcome

See `docs/artifacts/remote_wan_smoke_2026-03-20_pass.log` for the full run. The key lines are:

```text
mode=remote
relay1=45.197.133.115:30001
relay2=185.144.28.95:30002
exit=31.192.232.26:30000
target_scheme=http
...
test remote_http_smoke_requires_usable_path ... ok
...
remote_smoke observed_status=200 response_bytes=187 route_chain=client -> 45.197.133.115:30001 -> 185.144.28.95:30002 -> 31.192.232.26:30000 -> target
```
