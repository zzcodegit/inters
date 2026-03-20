# Stage 9.1 — Minimal Discovery / Capability Advertisement

## Purpose
Stage 9.1 adds a *small, explicit, local-first* discovery bootstrap-assist layer so nodes can:

1. advertise their existence + basic capabilities (role + UDP address)
2. cache recent node advertisements locally with strict TTL expiry
3. answer bounded discovery queries from bootstrap peers
4. learn a small candidate pool from discovery responses

This step is intentionally **not** full autonomous discovery, no DHT, no reputation/trust phase, and it does **not** change dataplane routing behavior.

Manual bootstrap via `peers[]` remains the primary operator-controlled seed for route selection.

## What this stage does (in this repo)
Discovery is control-plane only and uses plaintext `TunnelMessage`s over the existing UDP transport/wire wrapper. Relays remain stateless blind forwarders; discovery handling is implemented in the client + exit control paths.

### Node advertisement format
Advertisement (cached locally) is:
- `node_id`: `[u8; 32]` identifier
- `role`: `client | relay | exit`
- `addr`: `NodeAddr` (IP, port, protocol) for UDP contact
- `version`: string (build/version marker)
- `advertised_at_ms`: time created (ms)
- `ttl_ms`: advertisement TTL (ms)

Freshness rule:
- `is_fresh(now_ms)` means `now_ms <= advertised_at_ms + ttl_ms`
- expired advertisements are rejected and purged from the local cache

### Local bounded store
Each node keeps a bounded `DiscoveryStore`:
- `max_entries` is configured
- insert/update is by `node_id`
- stale entries are purged via `purge_expired(now_ms)`
- when over capacity, eviction is simple: evict the advertisement with the oldest expiry

## Discovery control-plane flow

### 1. Self-advertise (startup)
On startup/readiness (best-effort), the node:
- creates its own `NodeAdvertisement`
- inserts it into the local `DiscoveryStore`
- logs `event=discovery_self_advertise`

Publishing to peers is gated by `discovery_enabled`.

### 2. Publish (optional, best-effort)
If `discovery_enabled=true`, the node sends to configured bootstrap peers:
- `DiscoveryAdvertise(self_advertisement)` (plaintext)
- optionally `DiscoveryQuery` if `discovery_query_on_start=true`

Publishing and querying are best-effort and **must not** affect dataplane startup.

Bootstrap peers are taken from the existing operator `peers[]` config (converted to UDP socket addresses). If `peers[]` is empty for a role (commonly the `exit` examples), the node will still cache its own self-advertisement, but it will not actively publish/query.

### 3. Handle inbound advertise
When a node receives `DiscoveryAdvertise`:
- it validates freshness/TTL (store rejects expired)
- stores/updates the advertisement in the local cache (bounded)
- logs:
  - `event=discovery_advertise_received`
  - `event=discovery_store_update` (from the store)

### 4. Handle inbound query
When receiving `DiscoveryQuery`, the responder:
- purges expired cache entries
- selects a bounded list of fresh advertisements (prefers newest)
- responds with `DiscoveryResponse` containing at most `MAX_DISCOVERY_RESPONSE_ADS` (32) advertisements

Logs:
- `event=discovery_response` (includes number sent)

### 5. Handle inbound response
When receiving `DiscoveryResponse`:
- expired entries are ignored/rejected
- fresh advertisements are inserted into the local bounded store
- logs:
  - `event=discovery_response_received`
  - store update/purge logs

## Config knobs (minimal)
These fields are added to `NodeConfig` (TOML):
- `discovery_enabled` (default: `false`)
- `discovery_query_on_start` (default: `false`)
- `discovery_max_entries` (default: `256`, validated `1..=4096`)
- `discovery_advertise_ttl_sec` (default: `300`, validated `1..=3600`)

Notes:
- `discovery_max_entries` bounds memory usage for the local cache.
- Freshness is always enforced via TTL per advertisement.

Operator expectation:
- keep this disabled unless you explicitly want discovery-assisted candidate learning.
- manual topology via `peers[]` should remain unchanged.

## Safety / non-goals
Non-goals for Stage 9.1:
- no full DHT or global routing table
- no reputation / trust / anti-sybil
- no autonomous mesh formation or auto-connect to discovered candidates
- no pheromone or measurement sharing across nodes
- no dataplane packet format changes

## Observability
Stage 9.1 uses concise debug logs from:
- the discovery publish/receive path (`event=discovery_*`)
- store insert/update/evict/purge (`event=discovery_store_update`, `event=discovery_store_purge`)

## Manual bootstrap still works
If discovery is disabled, nodes behave exactly as before:
- `peers[]` remains the primary operator-provided seed input
- discovery does not rewrite operator config and does not change route selection

## Example TOML snippet
Below is a minimal operator-facing configuration block (flat schema; the
values are all optional and default to conservative `off`):

```toml
# Stage 9.1 minimal discovery (optional, control-plane only)
discovery_enabled = true
discovery_query_on_start = true   # optional: ask bootstrap peers at startup
discovery_max_entries = 256       # bounded local cache size
discovery_advertise_ttl_sec = 300 # accepted advertisement freshness (seconds)
```

Operator notes:
- Discovery is best-effort and control-plane only (plaintext advertisement/query/response over UDP).
- It uses existing `peers[]` as discovery bootstrap targets; manual topology still comes from `peers[]`.
- Discovered nodes are supplemental candidate data only; this stage does not auto-connect to them or rewrite routes/config.

