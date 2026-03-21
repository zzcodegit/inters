# Chaos / Fault Injection Stage

## Scope

This stage introduces a deterministic transport-chaos layer for integration use without changing the default production transport path.

The injected faults are controlled only by environment variables:

- packet loss
- packet duplication
- packet reordering via delayed send
- artificial base delay and jitter

The default path remains unchanged when no chaos env vars are set.

## Profiles

The committed runner [scripts/run_chaos_matrix.ps1](/C:/neinternet/vpnnode/scripts/run_chaos_matrix.ps1) executes these profiles:

- `none`
- `mild-loss`
- `mild-reorder`
- `mild-delay`
- `combined`

Each profile runs:

- exact local 3-hop path
- adaptive local path with 1/2/3-hop candidates
- 5 measured requests per mode

The request body is fixed to `32768` bytes and the measurement path now verifies `Content-Length`, so a `200 OK` with a truncated body is treated as a real failure.

## What was proven

Raw outputs and summaries are committed under [docs/artifacts](/C:/neinternet/vpnnode/docs/artifacts):

- master run log: [chaos_matrix_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_matrix_2026-03-22.log)
- master summary: [chaos_matrix_2026-03-22.md](/C:/neinternet/vpnnode/docs/artifacts/chaos_matrix_2026-03-22.md)
- machine summary: [chaos_matrix_2026-03-22.summary.json](/C:/neinternet/vpnnode/docs/artifacts/chaos_matrix_2026-03-22.summary.json)
- cleanup proof: [chaos_matrix_2026-03-22_cleanup.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_matrix_2026-03-22_cleanup.log)

Per-profile raw artifacts are also committed:

- `*.profile.json`
- `*.jsonl`
- `*.stage.jsonl`
- `*.decisions.jsonl`

## Selector behavior under mild chaos

The adaptive selector did not flap under the mild profiles that completed:

- `none`: 5 decision events, 0 route changes
- `mild-loss`: 5 decision events, 0 route changes
- `mild-reorder`: 5 decision events, 0 route changes
- `mild-delay`: 5 decision events, 0 route changes

All of those decisions were `current_still_best`, which is consistent with a stable route-quality policy rather than noise-driven switching.

## First chaos bottleneck

The first honest failure appears in the `combined` profile, not in the selector:

- the run fails fast
- the HTTP status can still be `200`
- but the received body is incomplete versus `Content-Length`

Captured failure:

- expected body: `32863`
- received body: `23323`

This is a response-path reliability failure, not a readiness false-positive.

The strongest early signals leading into that failure were:

- `late_close_after_completion`
- `late_payload_after_completion`
- `duplicate packet detected`
- `packet too old for replay window`

That makes the first chaos bottleneck a transport/lifecycle issue on the response path under mixed reordering, duplication, delay, and loss. The selector remains stable under mild chaos; the response pipeline degrades first.

## Ranking

From the committed summary:

- `mild-reorder` exposed replay-window rejects plus late completion events
- `mild-delay` amplified response-tail latency and late completion events
- `combined` triggered honest body truncation with duplicate/replay errors

The current evidence points to this ordering:

1. response-path duplicate/reorder tolerance
2. completion/lifecycle behavior after late packets
3. only then route-quality selection behavior

## Regression status

After adding the chaos layer, the existing accepted flows were revalidated:

- readiness: [chaos_validation_http_probe_ready_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_validation_http_probe_ready_2026-03-22.log)
- local baseline: [chaos_validation_local_baseline_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_validation_local_baseline_2026-03-22.log)
- remote WAN matrix: [chaos_validation_remote_wan_matrix_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_validation_remote_wan_matrix_2026-03-22.log)
- remote perf: [chaos_validation_remote_perf_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_validation_remote_perf_2026-03-22.log)
- remote stage: [chaos_validation_remote_perf_stage_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_validation_remote_perf_stage_2026-03-22.log)

The remote matrix needed the existing topology reset hook on shared public nodes. That is an operational requirement of the current remote environment, not a localhost substitution.

## Cleanup

Cleanup is captured in [chaos_matrix_2026-03-22_cleanup.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_matrix_2026-03-22_cleanup.log).

There were no matching lingering `vpnnode`, `cargo`, or `rustc` processes after the chaos run. The remaining socket lines are `TIME_WAIT`, not active listeners.
