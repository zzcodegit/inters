# Stage 5 Entry Baseline

This document defines the entry baseline for Stage 5. It does not implement congestion control. It fixes the starting point after Stage 4 freeze so that the next step can focus on performance/control instead of chaos semantics.

## Input artifacts

- End-to-end remote perf freeze: [docs/artifacts/stage4_freeze_remote_perf_2026-03-22.log](artifacts/stage4_freeze_remote_perf_2026-03-22.log), [docs/artifacts/stage5_entry_perf_matrix_2026-03-22.jsonl](artifacts/stage5_entry_perf_matrix_2026-03-22.jsonl), [docs/artifacts/stage5_entry_perf_matrix_2026-03-22.md](artifacts/stage5_entry_perf_matrix_2026-03-22.md)
- Overlay stage freeze: [docs/artifacts/stage4_freeze_remote_stage_2026-03-22.log](artifacts/stage4_freeze_remote_stage_2026-03-22.log), [docs/artifacts/stage5_entry_perf_stage_2026-03-22.local.jsonl](artifacts/stage5_entry_perf_stage_2026-03-22.local.jsonl), [docs/artifacts/stage5_entry_perf_stage_2026-03-22.client.jsonl](artifacts/stage5_entry_perf_stage_2026-03-22.client.jsonl), [docs/artifacts/stage5_entry_perf_stage_2026-03-22.exit.jsonl](artifacts/stage5_entry_perf_stage_2026-03-22.exit.jsonl), [docs/artifacts/stage5_entry_perf_stage_2026-03-22.joined.jsonl](artifacts/stage5_entry_perf_stage_2026-03-22.joined.jsonl), [docs/artifacts/stage5_entry_perf_stage_2026-03-22.md](artifacts/stage5_entry_perf_stage_2026-03-22.md)
- Remote topology proof: [docs/artifacts/stage4_freeze_remote_evidence_2026-03-22.md](artifacts/stage4_freeze_remote_evidence_2026-03-22.md)

## End-to-end route-length snapshot

From [docs/artifacts/stage5_entry_perf_matrix_2026-03-22.md](artifacts/stage5_entry_perf_matrix_2026-03-22.md):

| scenario | route length | avg TTFB ms | avg total ms | avg throughput Bps | errors |
| --- | ---: | ---: | ---: | ---: | ---: |
| direct | 0 | 258.89 | 1821.51 | 145002 | 0 |
| remote-1hop | 1 | 1210.38 | 1211.49 | 218779 | 0 |
| remote-2hop | 2 | 1075.05 | 1080.18 | 242778 | 0 |
| remote-3hop | 3 | 1500.08 | 1501.64 | 175198 | 0 |

This sample shows:

- all remote routes are operational
- `3-hop` is the slowest overlay path
- `1-hop` and `2-hop` are close enough that hop-count alone is not a safe performance predictor

## Stage breakdown by route

From [docs/artifacts/stage5_entry_perf_stage_2026-03-22.md](artifacts/stage5_entry_perf_stage_2026-03-22.md) and [docs/artifacts/stage5_entry_perf_stage_2026-03-22.joined.jsonl](artifacts/stage5_entry_perf_stage_2026-03-22.joined.jsonl):

| route | avg route ready ms | avg handshake ms | avg ACK ms | avg ACK p95 ms | avg retransmit rate | avg inflight | max inflight | avg window stall ms | avg overlay first-send -> client ms | avg exit/body tail ms | avg client total ms |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1-hop | 2724.42 | 550.00 | 217.00 | 306.40 | 0.0048 | 58.96 | 64 | 817.40 | 1096.04 | 966.40 | 1099.25 |
| 2-hop | 1624.82 | 417.00 | 215.40 | 293.00 | 0.0000 | 60.78 | 64 | 800.60 | 1122.43 | 935.80 | 1125.36 |
| 3-hop | 4193.76 | 628.00 | 294.80 | 325.80 | 0.0000 | 61.55 | 64 | 1135.20 | 1521.19 | 1208.60 | 1524.05 |

## What the baseline says now

The current first bottleneck is not a chaos semantics bug anymore. The remaining dominant cost is response-path control behavior after the target is already reachable:

- `target_connect_ms` is effectively `0`
- first target byte wait is about `1.4-1.6 ms`
- the dominant cost sits in `overlay first-send -> client first-byte gap`
- the next dominant cost is `window stall / backpressure`
- `total_time_ms` vs `ack_latency_ms_avg` correlation is `0.976`

That points to one concrete class of problem: bursty response delivery over a fixed inflight window, not target-side latency and not unresolved chaos semantics.

## Route-quality vs latency/perf correlation

The accepted route-quality selector already scores on the same axes that dominate current performance:

- ACK latency
- retransmit rate
- TTFB / total time
- recent success / failure
- window stall / backpressure

The freeze baseline lines up with that model:

- the slowest path (`3-hop`) also shows the highest ACK tail and the largest window stall
- `1-hop` vs `2-hop` trade places depending on live WAN conditions, which confirms that hop-count alone is insufficient
- the quality-driven policy remains the right selection layer, but the next gain is not more selector logic; it is better send discipline on the response path

## Stage 5 target definition

The Stage 5 target is:

`RTT-aware response pacing over the current fixed inflight window`

Why this target, and not something else:

- not replay-window: accepted in Stage 4
- not lifecycle tail: accepted in Stage 4
- not duplicate/terminal semantics: accepted in Stage 4
- not target connect optimization: target-side cost is already near zero
- not route-score redesign: current quality model already tracks the right performance signals

Pacing is the next step because the current transport still behaves like a burst sender into a fixed `64`-frame window, then spends time stalled behind ACK arrival and backpressure.

## Stage 5 success criteria seed

Stage 5 should be evaluated against this baseline with the expectation that:

- window stall decreases
- overlay first-send -> client gap decreases
- ACK tail decreases
- route variance under shared WAN decreases
- throughput rises without reopening the Stage 4 chaos semantics failures
