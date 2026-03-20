## vpnnode — Stage 1–5 dataplane MVP

### Build

- **Build**: `cargo build`
- **Run tests**: `cargo test`

### Baseline modes

- **Local baseline**: `bash scripts/run_baseline.sh`
- **Remote WAN smoke**: `bash scripts/run_baseline_remote.sh`
- Details: `docs/remote_baseline.md`
- Current WAN `504` investigation: `docs/wan_504_rca.md`

### Ops: golden startup examples (TOML config)

From the project root (`vpnnode/`):

```powershell
# Validate config and bind feasibility (no long-running server)
cargo run -- check --config .\examples\config\relay.toml

# Run nodes
cargo run -- run --config .\examples\config\relay.toml
cargo run -- run --config .\examples\config\exit.toml
cargo run -- run --config .\examples\config\client.toml
```

Expected readiness logs (examples):

```text
INFO vpnnode: startup build=vpnnode v0.1.0 commit=<...> build_ts_ms=<...>
INFO vpnnode: node ready (self-check ok) role=Relay bind=0.0.0.0:30000
INFO vpnnode::roles::relay: node ready role=relay addr=0.0.0.0:30000
INFO vpnnode::roles::exit: node ready role=exit addr=0.0.0.0:30001
INFO vpnnode::roles::client: node ready role=client udp=Udp://0.0.0.0:<ephemeral> tcp=0.0.0.0:10080
```

### Ops: config contract (NodeConfig TOML)

Config files are loaded via `vpnnode run --config <file>` and validated fail-fast.

- **Required fields (all roles)**:
  - `role`: `"client" | "relay" | "exit"`
  - `bind_ip`: IP to bind UDP socket (`"0.0.0.0"` or concrete IP)
  - `bind_port`: UDP port to bind (must be non-zero)
- **Optional fields (all roles)**:
  - `peers`: `Vec<NodeAddr>` used as manual route/peer seed (UDP-only in current stage)
  - `ants_enabled`: boolean (enables Stage 7 measurement ants; default `false`)
  - `discovery_*`: optional Stage 9.1 minimal discovery knobs; see `docs/stage9_minimal_discovery.md`
- **Client-only fields**:
  - `client_local_listen`: local TCP listener for operators/curl (default `127.0.0.1:10080`)
  - `route_length`: `1..=16` (default `2`)
- **Exit-only fields**:
  - `exit_target_addr`: target TCP address exit connects to (default `127.0.0.1:8080`)
  - `exit_target_allowlist`: optional list of target hosts (foundation only; not enforced yet)

How `peers[]` is interpreted:

- **Client**: ordered hop seed for route building.
  - `route_length=1`: `peers[]` may be empty (client can use defaults), or contain `[exit]`
  - `route_length=2`: expect `[relay, exit]`
  - `route_length=3`: expect `[relay1, relay2, exit]`
- **Relay**: `peers[0]` is the exit UDP address
- **Exit**: `peers[]` currently unused

### Local manual run (TCP listener mode, without Docker)

In separate terminals:

1. **Target service**
   - `cargo run -- target --listen 127.0.0.1:8080`
2. **Exit node**
   - `cargo run -- exit --listen 127.0.0.1:30001 --target-addr 127.0.0.1:8080`
3. **Relay node**
   - `cargo run -- relay --listen 127.0.0.1:30000 --exit-addr 127.0.0.1:30001`
4. **Client node (tcp-listener mode)**
   - `cargo run -- client --local-listen 127.0.0.1:10080 --relay-addr 127.0.0.1:30000 --exit-addr 127.0.0.1:30001 --route-length 2`

Then perform manual check:

- `curl -v http://127.0.0.1:10080/`

You should see an HTTP `200 OK` response from the target service, routed as:

`curl -> client(local TCP) -> client(UDP tunnel) -> relay -> exit -> target -> exit -> relay -> client -> curl`.

### Docker / docker-compose (TCP listener path)

From the project root (`vpnnode/`):

> **Stage 4 note:** On **Docker Desktop (Windows)**, large UDP packets may be dropped; validation can fail with "Empty reply from server". Use **Linux Docker** or **native run** (`run-local.ps1` on Windows) for reliable validation.

1. **Сборка и запуск:**
   ```bash
   docker-compose build
   docker-compose up
   ```
   Или в фоне: `docker-compose up -d`

2. **Проверка** (с хоста):
   - `curl -v http://127.0.0.1:10080/`
   - Большой POST: `curl.exe -v --data-binary '@largefile.bin' http://127.0.0.1:10080/ -o response.bin` (PowerShell: имя файла в кавычках)

Порт **10080** проброшен с контейнера client на хост. Остальные сервисы общаются внутри Docker-сети (target:8080, exit:30001, relay:30000).

> **PowerShell note (Windows)**: the built-in `curl` alias (`Invoke-WebRequest`) may print a
> security warning (`риск выполнения сценария`) and ask for confirmation. Answer `Y` (or use
> `curl.exe` from Git for Windows) to actually see the HTTP response body.
>
> **Large POST from PowerShell**: `@` is the splatting operator in PowerShell, so for real curl use
> a quoted argument: `curl.exe -v --data-binary '@largefile.bin' http://127.0.0.1:10080/`
> (or `Invoke-WebRequest -Uri http://127.0.0.1:10080/ -Method POST -InFile .\largefile.bin -UseBasicParsing`).

### Current limitations

- **Crypto / handshake**
  - From **Stage 3** onwards, the dataplane uses a **runtime x25519-based handshake** between client and exit. The relay remains a blind forwarder and never sees decrypted payloads.
  - Each client↔exit session derives a **per-session AEAD key** via x25519 + HKDF; all tunnel DATA/OpenStream/CloseStream/Error frames are protected with this key.
  - Handshake messages themselves are currently **unauthenticated and cleartext** (lab/dev trust model): the protocol provides **confidentiality and integrity for tunnel data**, but not yet a full authenticated PKI or strong node identity guarantees.
  - The old static dev PSK (`dev_default_aead_key`) is still present in the codebase for tests and helpers but is **no longer used in the main runtime datapath**.
- **Transport abstraction**
  - A generic `Transport` trait exists and is used by roles (client / relay / exit).
  - Current implementation is UDP-only (`UdpTransport`). Future adapters (QUIC / WS-TLS / WireGuard / VLESS-like) are planned but not implemented.
- **Client modes**
  - Primary, stable mode is **tcp-listener mode** (local TCP listener that forwards HTTP through the tunnel).
  - **TUN mode is experimental** in Stage 2, Unix-only, and intended for **controlled lab environments**, not a full system VPN.
- **Routing / route memory**
  - Route memory is **local-only** and stores **only technical route hints** (relay/exit/transport/latency/TTL). It does **not** record concrete user destinations.
  - There is no distributed control plane or shared route recommendation yet.
- **Stage 5: Variable-length circuits**
  - Routes can be 1 hop (client→exit), 2 hops (client→relay→exit), or 3+ hops (client→relay₁→relay₂→…→exit).
  - Use `--route-length 1|2|3` and `--exit-addr`, `--relay2-addr` (for 3-hop). Relays remain blind forwarders; handshake is once per circuit at the exit.

### TUN packet contract (Stage 2)

- **Scope**
  - Stage 2 TUN support is limited to **IPv4/TCP** traffic in a controlled lab scenario.
  - Only the **client** owns a TUN device; relay and exit remain UDP-only and do not see or parse IP headers.
- **Forward path (app → TUN → tunnel)**
  - The client TUN loop (`run_client_tun_mode`) reads a full **raw IPv4 packet** from the TUN device.
  - It parses the IPv4 header (`Ipv4Header::parse`) and the TCP ports (`parse_tcp_ports`) to derive a `FlowKey` \([src ip, dst ip, src port, dst port, protocol]\).
  - For Stage 2, the **entire raw IPv4 packet** (header + TCP header + payload) is placed as the `payload` of a `TunnelMessage` with `MsgType::Data` and sent through the UDP tunnel.
- **Reverse path (tunnel → client → TUN)**
  - The client receives `TunnelMessage::Data` from the exit, treats the `payload` as an opaque byte-buffer, and writes it **verbatim back to the TUN device**.
  - For a realistic IP round-trip in a lab, the upstream target/exit combination must preserve or reconstruct full IPv4 packets in the `payload`; in the current MVP the exit **does not parse IP**, and simply moves TCP payloads to/from the target TCP socket. This makes TUN mode suitable for controlled experiments and byte-level tracing, but **not a full general-purpose IP VPN yet**.
- **Known constraints**
  - **IPv4-only**, **TCP-only**, and **single-hop (client → relay → exit)**.
  - No DNS interception, no MTU/path-MTU discovery, and no TCP state machine on the dataplane.
  - Packet semantics are aligned for lab usage; production-grade IP semantics (including correct TCP/IP checksums and OS TCP stack expectations across both directions) are intentionally **out of scope for Stage 2**.

### Flow lifecycle and cleanup

- **Flow table**
  - `FlowTable` is a bidirectional map between `FlowKey` and `stream_id`, used only in TUN mode on the client to tie IPv4/TCP 5-tuples to tunnel streams.
  - Each flow entry tracks a `last_seen` timestamp to support **idle-timeout based cleanup**.
- **Cleanup policy (TUN mode)**
  - **On `MsgType::Error`** from the tunnel: the client drops the corresponding flow mapping (`remove_by_stream`), logging the error as the reason.
  - **On `MsgType::CloseStream`**: the client also removes the mapping, treating it as an explicit close.
  - **Idle timeout**: before creating or reusing a flow for an outbound TUN packet, the client runs `FlowTable::prune_idle` with a conservative timeout (currently **60 seconds**) and logs how many stale flows were removed.
  - **Activity refresh**: when a reverse-path data frame arrives for a known stream, the flow entry is `touch`‑ed, extending its lifetime.

### TUN client mode (experimental, manual verification)

- **Supported OS**
  - Unix-like systems with userspace TUN support (e.g. Linux). On non-Unix platforms TUN device creation fails early and TUN-specific tests are skipped.
- **Run topology (manual, no Docker)**
  1. **Target service**
     - `cargo run -- target --listen 127.0.0.1:8080`
  2. **Exit node**
     - `cargo run -- exit --listen 127.0.0.1:30001 --target-addr 127.0.0.1:8080`
  3. **Relay node**
     - `cargo run -- relay --listen 127.0.0.1:30000 --exit-addr 127.0.0.1:30001`
  4. **Client node (TUN mode, root required)**
     - `sudo RUST_LOG=info vpnnode client --mode tun --relay-addr 127.0.0.1:30000 --tun-name tun-vpnnode0 --tun-address 10.10.0.1 --tun-netmask 255.255.255.0 --tun-mtu 1500`
- **Minimal lab verification steps (Linux)**
  1. Verify that the TUN interface exists and is up:
     - `ip addr show dev tun-vpnnode0`
  2. From another terminal, run a packet sniffer on the TUN device:
     - `sudo tcpdump -ni tun-vpnnode0`
  3. In a third terminal, generate IPv4/TCP traffic destined for the TUN address (e.g. from a network namespace configured to use `10.10.0.1` as its default gateway) and hit the HTTP target:
     - `ip netns add vpnlab`
     - `ip link add veth-vpn type veth peer name veth-host`
     - `ip link set veth-vpn netns vpnlab`
     - `ip addr add 10.10.0.254/24 dev veth-host && ip link set veth-host up`
     - `ip netns exec vpnlab ip addr add 10.10.0.2/24 dev veth-vpn`
     - `ip netns exec vpnlab ip link set veth-vpn up`
     - `ip netns exec vpnlab ip route add default via 10.10.0.1`
     - `ip netns exec vpnlab curl -v http://127.0.0.1:8080/ || true`
  4. Observe logs and packets:
     - On the client: look for `client tun: read packet from tun`, `flow created or reused`, and `sending DATA over tunnel` for the forward path, and `client tun: wrote packet to tun` for the reverse path.
     - On the exit: observe connections to the target and `exit sent DATA back to client` for responses.
     - In `tcpdump` on `tun-vpnnode0`: you should see the injected IPv4/TCP packets on the forward path and corresponding packets written back on the reverse path.

This verifies the Stage 2 TUN datapath:

`app/process (in netns) -> TUN -> client (TUN mode) -> relay -> exit -> target -> exit -> relay -> client (TUN mode) -> TUN -> app/process`.

### Stage 2 support matrix

- **Linux (Unix)**:
  - **Supported**:
    - `client --mode tcp` (local TCP listener) — primary, stable Stage 2 path.
    - `client --mode tun` — **experimental, lab-only**, requires root and a userspace TUN device.
    - `relay` and `exit` over native UDP.
    - `docker-compose` stack for TCP listener mode.
  - **Not in scope yet**:
    - Full system-wide VPN integration.
    - Production-grade IP semantics for all applications.
- **macOS**:
  - **Supported**:
    - `client --mode tcp` (local TCP listener) and the end-to-end dataplane (client → relay → exit → target).
  - **Not supported / not tested**:
    - TUN mode (no TUN wiring for Stage 2 on macOS).
- **Windows**:
  - **Supported**:
    - `cargo build` for the library and binary.
    - `client --mode tcp` as an architectural mode (same packet contract; validated via Linux/Docker).
    - Docker-based TCP validation path (containers run Linux).
  - **Intentional limitations**:
    - `client --mode tun` is **not supported**; the client fails fast with a clear error:
      `client tun mode is only supported on Unix-like systems (e.g. Linux) in Stage 2. See README: Stage 2 support matrix.`
    - `TunDevice` APIs on non-Unix always error with an explicit Unix-only message.
    - Some integration tests may fail to link on local Windows hosts due to MSVC toolchain
      configuration; this is treated as an **environment issue**, not a Stage 2 dataplane bug.
- **Transports**:
  - Implemented: **Native UDP** only.
  - Architecturally planned (not yet implemented): WireGuard, QUIC, TLS/WebSocket, VLESS-like.

### Stage 3 session security model

- **Handshake flow**
  - On startup, the client establishes a **single logical session** with the exit:
    1. The client sends a cleartext `HandshakeInit` message containing its ephemeral x25519 public key and a random nonce, wrapped in a `TunnelMessage` with `MsgType::HandshakeInit`.
    2. The exit responds with a cleartext `HandshakeAck` containing its own ephemeral x25519 public key and a random nonce.
    3. Both sides derive a shared secret via x25519 and feed it into HKDF (`vpnnode-session-key` label) to obtain a **session-scoped AEAD key**.
    4. Once this completes, all subsequent tunnel traffic (DATA/OpenStream/CloseStream/Error/etc.) is encrypted and authenticated with ChaCha20-Poly1305 using per-packet nonces derived from a **monotonic per-session sequence number**.
- **Replay protection**
  - For each session, the receiver tracks:
    - The highest sequence number seen so far.
    - A small **sliding replay window** (currently 64 packets) with a bitmap of recently seen sequence numbers.
  - Packets that are:
    - **Too old** (sequence number falls entirely outside the receive window), or
    - **Duplicates** within the window
    are rejected before decryption, and a detailed error is logged.
- **Session lifecycle**
  - States are minimal but explicit in behavior:
    - **Handshake in progress**: only cleartext HandshakeInit/HandshakeAck are accepted; data frames are ignored.
    - **Established**: all application frames must be AEAD-protected with the negotiated key; cleartext non-handshake messages are rejected.
    - **Failed / timeout**: client handshake uses bounded timeouts; handshake failures surface as clear errors and the client/node exits instead of hanging.
  - Stream-level CloseStream and Error semantics from Stages 1/2 remain intact; streams are still cleaned up deterministically on both client and exit.
- **What Stage 3 provides**
  - End-to-end **confidentiality and integrity** for tunnel payloads between client and exit.
  - A **per-session key** derived via x25519 + HKDF (no long-term reuse of a single dev PSK).
  - Basic but concrete **anti-replay guarantees** at the per-session level.
  - A clear separation between:
    - **Handshake crypto state** (key agreement, cleartext control exchange).
    - **Established session crypto state** (AEAD with sequence numbers and replay window).
- **What Stage 3 does NOT yet provide**
  - No production-grade **identity / PKI**:
    - Handshake peers are not authenticated against certificates or a Web-of-Trust.
    - Static key files (`client.key`, `relay.key`, `exit.key`, etc.) are currently unused in the runtime handshake and reserved for future identity work.
  - No protection against **active network adversaries** that can impersonate nodes or inject handshake messages.
  - No multi-session, multi-transport negotiation yet; the handshake carries simple transport hints only (`udp-native`).

### Stage 3.1 Handshake Hardening (stateless anti-amplification)

- **Goal**
  - Protect the exit from **amplification** and **state exhaustion**: an attacker must not be able to force the exit to create session state or perform expensive x25519 until the client proves it can receive at the claimed address (cookie challenge).
- **Handshake flow**
  1. **Client** → `HandshakeInit` (no cookie).
  2. **Exit** → does **not** create session; sends `HandshakeChallenge(cookie)` only. No x25519 yet.
  3. **Client** → `HandshakeInit` (same client_pubkey + client_nonce, **with** cookie).
  4. **Exit** → verifies cookie; only then runs x25519, sends `HandshakeAck`, creates `SessionCrypto`.
- **Stateless cookie**
  - Cookie = HMAC-SHA256(server_secret, client_ip ‖ client_port ‖ client_pubkey ‖ client_nonce ‖ timestamp_bucket).
  - `timestamp_bucket = current_time_secs / 30`; cookies expire after ~30s; verification allows current and previous bucket for clock skew.
  - Exit does **not** store cookies; verification recomputes and compares. Protects against **replay** of old challenges and **spoofed source IP** (cookie is bound to client address and nonce).
- **Rate limiting**
  - Exit enforces a simple per-IP handshake rate limit (default **50 attempts per second**). Excess init requests are dropped before any cookie is issued.
- **Security properties**
  - **Anti-amplification**: exit sends a small challenge instead of a full HandshakeAck until the client echoes the cookie.
  - **Anti state exhaustion**: no session or x25519 until cookie is verified.
  - **No session before verify**: x25519 and HKDF run only after a valid init+cookie.

### Stage 4 Reliable Streaming

- **Goal**  
  Reliable, in-order delivery of stream data over UDP: sequence tracking, acknowledgements, retransmission, buffering, and flow control on the **stream** level (client and exit only; relay stays a blind forwarder).

- **StreamFrame (DATA payload)**  
  Each DATA message carries a bincode-serialized `StreamFrame`: `stream_id`, `frame_seq`, `ack_seq`, `payload`.  
  - `frame_seq`: sender’s frame number.  
  - `ack_seq`: last contiguous frame the receiver has accepted (cumulative ACK).

- **Sequence and ordering**  
  Sender assigns monotonic `frame_seq`; receiver delivers payloads in order and buffers out-of-order frames until the gap is filled, then delivers in sequence.

- **Acknowledgement**  
  Receiver sends back ACK via `ack_seq` (piggy-backed on data or in an empty-payload StreamFrame). Sender clears `unacked` for all `frame_seq < ack_seq`.

- **Retransmission**  
  Sender keeps unacked frames in a map. A background task runs every **200 ms**, collects all streams’ unacked frames, and resends them over the tunnel. No per-frame timeout (minimal implementation).

- **Buffering (out-of-order)**  
  If a frame arrives with `frame_seq > recv_next`, it is stored in `recv_buffer`. When the missing frame arrives, the run of contiguous frames is delivered in order.

- **Flow control**  
  `max_inflight_frames = 64`. If a stream has more than 64 unacked frames, the client pauses sending new data (short sleep and recheck) until the count drops.

- **End-of-request**  
  Client sends an empty-payload StreamFrame after the last request chunk; exit accumulates chunks and flushes the full request to the target when it sees this empty frame.

- **Where it runs**  
  Logic lives in `stream_reliable::ReliableStream` and is used only on **client** (TCP mode) and **exit**; relay and `TunnelMessage` format are unchanged.

### Stage 4 Validation (Reliable Streaming)

Use native execution or Linux environments for authoritative validation.

- **Native run works**
  - **Windows**: `.\run-local.ps1` (see *Native local run (Windows recommended)* below)
  - **Linux / macOS**: `cargo run` in four terminals, or equivalent script
- **Docker on Linux** works and is recommended for CI and validation.
- **Docker Desktop on Windows** is **not reliable** for UDP dataplane:
  - Large UDP packets (response DATA, typically 200+ bytes) may be **dropped** between relay and client.
  - Client receives only small ACK packets; large response chunks are lost.
  - `curl` may return "Empty reply from server".
  - This is a **known Docker Desktop / WSL2 limitation**, not a code defect.
  - Do **not** use Docker Desktop on Windows as a source of truth for UDP dataplane validation.

### Stage 4 Support Matrix

| Environment                | Status | Notes                              |
| -------------------------- | ------ | ---------------------------------- |
| Linux (native)             | ✅     | Fully supported                     |
| Linux (Docker)             | ✅     | Recommended validation environment  |
| Windows (native run-local) | ✅     | Supported for development           |
| Windows (Docker Desktop)   | ⚠️     | UDP packet loss possible; unreliable |

### Stage 4 Status

- Reliable streaming implemented (sequence, ACK, retransmit).
- Retransmit loop working.
- Out-of-order handling implemented (buffering and in-order delivery).
- Large transfers validated (200 KB+ POST).
- Native dataplane verified end-to-end.

**Completed and accepted** (with environment caveat for Docker Desktop on Windows).

Docker Desktop on Windows is **not** used as a source of truth for UDP dataplane validation.

### Native local run (Windows recommended)

On Windows, use `run-local.ps1` to avoid Docker UDP issues. It:

- Runs target, exit, relay, and client as **separate processes** on localhost.
- Uses random high ports (40000–50000) to avoid conflicts.
- Stores logs in `run-local-logs/` for debugging.

**Usage:**

```powershell
.\run-local.ps1
```

Run with fixed checklist ports (so you can always `curl http://127.0.0.1:10080/`):

```powershell
.\run-local.ps1 -FixedPorts
```

The script builds the release binary if needed, starts all four components, performs a `curl` request, and prints the response. Use this for authoritative Stage 4 validation on Windows.

### Ops: smoke e2e (manual procedure)

Goal: verify `client -> relay -> exit -> target` works on one host.

1. Start target service:
   - `cargo run -- target --listen 127.0.0.1:8080`
2. Start exit:
   - `cargo run -- run --config .\examples\config\exit.toml`
3. Start relay:
   - `cargo run -- run --config .\examples\config\relay.toml`
4. Start client:
   - `cargo run -- run --config .\examples\config\client.toml`
5. Verify traffic:
   - `curl -v http://127.0.0.1:10080/`

Expected:
- client logs show handshake established and stream open/close
- exit logs show target connect and response send
- curl receives `HTTP/1.1 200 OK`

### Stage 3 validation

- **Cargo**
  - `cargo build` must succeed.
  - `cargo test --lib` runs all unit tests, including:
    - Protocol encode/decode.
    - x25519 + HKDF key derivation and handshake roundtrip.
    - **Stage 3.1** cookie: generate/verify, wrong IP/nonce/expired rejected.
    - SessionCrypto replay window behavior.
    - Route memory behavior.
    - Transport abstraction basic roundtrip.
  - On some Windows/MSVC setups, the `tests/integration.rs` binary may fail to link with `LNK1104` due to local toolchain configuration; this is an **environmental limitation**. The same integration tests are expected to pass in a Linux/Docker environment.
- **Docker (TCP listener path, recommended)**
  1. `docker-compose build`
  2. `docker-compose up -d`
  3. From the host: `curl -v http://127.0.0.1:10080/`
  4. Expect `HTTP/1.1 200 OK` with the echoed HTTP request body, routed as:
     `curl -> client(local TCP) -> client(UDP tunnel, Stage 3.1 challenge then x25519/HKDF session) -> relay -> exit -> target -> exit -> relay -> client -> curl`.
- **Manual TCP validation (no Docker)**
  - Use the commands from *Local manual run (TCP listener mode, without Docker)* above; logs on client and exit should now include:
    - `HandshakeInit` → `HandshakeChallenge` → `HandshakeInit(cookie)` → `HandshakeAck`.
    - `session established via HandshakeChallenge then HandshakeAck` (exit) and `session established with exit (Stage 3.1 challenge flow)` (client).
    - Per-stream open/close and bytes in/out as before.
- **Manual TUN validation (Linux lab)**
  - Same as Stage 2, but with the understanding that the TUN traffic is now protected by the Stage 3 session crypto model (cleartext TUN packets are wrapped into AEAD-protected tunnel DATA frames).

### Stage 2 validation

- **Cargo**:
  - `cargo build` must succeed on the target host.
  - `cargo test` runs all unit and integration tests. On Windows, the integration test binary
    may fail to link with `LNK1104` if the local MSVC toolchain is misconfigured; this is an
    environment limitation and does **not** change the Stage 2 support statement, as the same
    tests pass in a Linux/Docker environment.
- **Docker (TCP listener path, recommended)**:
  1. `docker-compose build`
  2. `docker-compose up -d`
  3. From the host: `curl -v http://127.0.0.1:10080/`
  4. Expect `HTTP/1.1 200 OK` with the echoed HTTP request body, routed as:
     `curl -> client(local TCP) -> client(UDP tunnel) -> relay -> exit -> target -> exit -> relay -> client -> curl`.
- **Manual TCP validation (no Docker)**:
  - Use the commands from *Local manual run (TCP listener mode, without Docker)* above.
- **Manual TUN validation (Linux lab)**:
  - Use the steps from *TUN client mode (experimental, manual verification)* above.
- **Windows behavior**:
  - `client --mode tcp` is the only supported client mode.
  - `client --mode tun` fails fast with a clear, user-facing error and never attempts to open
    a TUN device.
