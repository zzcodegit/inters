# WAN evidence snapshot (2026-03-20)

## Exit runtime config

```toml
role = "exit"
bind_ip = "0.0.0.0"
bind_port = 30000
exit_target_addr = "github.com:443"
```

## Real relay forwarding path

```text
relay-1  2026-03-20T19:21:08.699901Z from=Udp://46.242.13.60:2061 to=Udp://185.144.28.95:30002
relay-2  2026-03-20T19:21:11.886584Z from=Udp://45.197.133.115:30001 to=Udp://31.192.232.26:30000
```

## Exit receives request but emits no response bytes

```text
2026-03-20T19:21:13.245985Z INFO vpnnode::roles::exit: exit: appended request bytes from client stream_id=1000000002 appended=50 total=50
2026-03-20T19:21:13.246148Z INFO vpnnode::roles::exit: exit: received end-of-request marker from client stream_id=1000000002
2026-03-20T19:21:13.423779Z INFO vpnnode::roles::exit: VALIDATION_ARTIFACT_FINAL,stream_id=1000000002,site=probe,exit=Udp://31.192.232.26:30000,route=1,resp_bytes=0,frames_sent=0,bursts_count=1,avg_inflight=1.000,max_inflight=1,time_at_inflight_1_ms=150,retransmit_rate=0.000000,stream_duration_ms=151,throughput_bps=0,ack_latency_ms_avg=0,http_code=0,is_final=1,mode=overlay
```

## Client handshake succeeds, then route attempts fail with no response

```text
2026-03-20T19:21:12.105034Z INFO vpnnode::roles::client: client handshake: session established with exit (Stage 3.1 challenge flow)
2026-03-20T19:21:36.912379Z INFO vpnnode::roles::client: route failed, switching stream_id=1 attempt=0 hops=3 ... e=no response (timeout or channel closed)
2026-03-20T19:21:37.154205Z INFO vpnnode::roles::client: route failed, switching stream_id=1 attempt=1 hops=1 ... e=no response (timeout or channel closed)
2026-03-20T19:21:37.396918Z INFO vpnnode::roles::client: route failed, switching stream_id=1 attempt=2 hops=2 ... e=no response (timeout or channel closed)
2026-03-20T19:21:37.397233Z ERROR vpnnode::roles::client: client: all routes failed stream_id=1
```

## Observed HTTP result

```text
HTTP/1.1 504 Gateway Timeout
connect=0.001434 starttransfer=21.077637 total=21.077705 code=504
```
