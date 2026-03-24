# RTT-Aware Retransmit Backoff RCA

## Problem

After the short-path ACK-signal fix, the next transport mismatch was no longer in `effective_inflight_cap`.

The remaining issue was the retransmit timer itself on the exit response path:

- it was stream-wide, not frame-aware
- it was anchored to coarse ACK summaries
- it used a minimum of `350ms`, plus a `+75ms` safety margin on top of `p95/avg/p50`

From the previous committed code in `HEAD`:

```rust
const EXIT_RESPONSE_RETRANSMIT_BASE_MS: u64 = 350;
const EXIT_RESPONSE_RETRANSMIT_SAFETY_MARGIN_MS: u64 = 75;
const EXIT_RESPONSE_RETRANSMIT_MAX_MS: u64 = 1500;
interval_ms = max(350, ack_summary + 75)
```

That policy was too coarse for short and medium WAN paths, and not expressive enough for repeated-loss behavior on longer paths.

## Root Cause

One root cause:

- exit retransmit timing was driven by a coarse stream-level timeout `max(350ms, summary+75ms)` instead of a per-frame timeout tied to the current RTT state of the path

This created a real RTT mismatch:

| route | current ACK avg ms | current ACK p95 ms | previous coarse timeout estimate |
| --- | --- | --- | --- |
| `1-hop` | `181.0` | `184.4` | `350ms` |
| `2-hop` | `179.4` | `189.6` | `350ms` |
| `3-hop` | `230.8` | `248.4` | `350ms` |
| `5-hop` | `271.2` | `293.8` | `369ms` |

Why this mattered:

- `1-hop` and `2-hop`: `350ms` is about `1.9x` real ACK cadence, so retransmit reacts late and stretches tail recovery
- `3-hop`: `350ms` is closer to `1.5x`, so the mismatch is smaller
- `5-hop`: one fixed stream-wide timeout is still too coarse once repeated loss starts; it does not distinguish first suspicion from repeated loss pressure

## Fix

The new policy keeps the same hard window and does not introduce congestion control.

It only makes retransmit timeout RTT-aware and frame-aware:

```text
base_rtt = ack_summary.p50
       or ack_summary.avg
       or ack_summary.p95
       or bootstrap_rtt

factor:
  0 retransmits => 1.5
  1 retransmit  => 2.0
  2+            => 2.5 + 0.25 * extra, capped at 3.5

timeout = clamp(base_rtt * factor, 50ms, 1000ms)
```

Important details:

- uses `last_sent`, not `first_sent`
- computes timeout per unacked frame
- records timeout/RTT ratio, early count, and late count
- does not change routing, ACK semantics, pacing, or window size

## Current Evidence

Fresh stage-run evidence:

- [retransmit_backoff_remote_perf_stage_2026-03-23.md](/C:/neinternet/vpnnode/docs/artifacts/retransmit_backoff_remote_perf_stage_2026-03-23.md)
- [retransmit_backoff_remote_perf_stage_2026-03-23.joined.jsonl](/C:/neinternet/vpnnode/docs/artifacts/retransmit_backoff_remote_perf_stage_2026-03-23.joined.jsonl)
- [retransmit_backoff_remote_perf_stage_2026-03-23.exit.jsonl](/C:/neinternet/vpnnode/docs/artifacts/retransmit_backoff_remote_perf_stage_2026-03-23.exit.jsonl)

Observed per route:

| route | retransmit rate | timeout avg ms | timeout p95 ms | trigger count | early | late | avg timeout / RTT |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `1-hop` | `0.0000` | `-` | `-` | `0` | `0` | `0` | `-` |
| `2-hop` | `0.0441` | `273` | `273` | `13` avg, `64` in the noisy run | `0` | `0` | `1.50` |
| `3-hop` | `0.0401` | `369` | `369` | `12` avg, `58` in the noisy run | `0` | `0` | `1.50` |
| `5-hop` | `0.2810` | `231` | `242` | `81` avg, `397` in the worst run | `0` | `6` | `1.63` |

Interpretation:

- `1-hop` no longer inflates retransmit timing on a healthy short path; it simply does not retransmit in the fresh sample
- `2-hop` and `3-hop` now sit exactly at the intended `~1.5x RTT` baseline
- `5-hop` can rise above the baseline under repeated loss, which is where the repeated-loss factor is supposed to help
- there are no early retransmits in the current local or remote evidence

## Why This Is The Right Bottleneck

This task specifically asked to remove the mismatch between real RTT path state and retransmit timing.

The current evidence shows:

- `effective_inflight_cap` stayed at `64` with `cap_reduced=0`
- pacing stayed enabled and stable
- Stage 4 chaos stayed green
- the new signal introduced by this step is exactly the retransmit timeout/RTT coupling

So the bottleneck addressed here is not route scoring, not replay, not lifecycle, and not a hidden window change.
It is the retransmit timeout policy itself.
