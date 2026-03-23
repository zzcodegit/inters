# Response Pacing Before/After Compare

## Sources

- before remote stage baseline: [stage5_entry_perf_stage_2026-03-22.joined.jsonl](/C:/neinternet/vpnnode/docs/artifacts/stage5_entry_perf_stage_2026-03-22.joined.jsonl)
- after remote stage run: [stage5_pacing_remote_perf_stage_2026-03-22.joined.jsonl](/C:/neinternet/vpnnode/docs/artifacts/stage5_pacing_remote_perf_stage_2026-03-22.joined.jsonl)
- targeted local before: [stage5_pacing_targeted_before_2026-03-22.jsonl](/C:/neinternet/vpnnode/docs/artifacts/stage5_pacing_targeted_before_2026-03-22.jsonl)
- targeted local after: [stage5_pacing_targeted_after_2026-03-22.jsonl](/C:/neinternet/vpnnode/docs/artifacts/stage5_pacing_targeted_after_2026-03-22.jsonl)

## Local Targeted Regression

This is the direct A/B proof against the old fixed-window send path under `combined` chaos.

| metric | before | after |
| --- | ---: | ---: |
| avg total ms | 1223.07 | 615.84 |
| avg TTFB ms | 1221.57 | 613.60 |
| max send burst frames | 37.0 | 5.0 |
| pacing interval ms avg | n/a | 4.0 |
| pacing delay applied | 0.0 | 9.0 |
| window stall ms | 0.0 | 0.0 |

## Remote Stage Compare

| scenario | total ms before | total ms after | ack avg before | ack avg after | ack p95 before | ack p95 after | window stall before | window stall after | avg inflight before | avg inflight after | max inflight before | max inflight after | max burst after |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| remote-1hop | 1099.25 | 1113.37 | 217.0 | 182.8 | 306.4 | 190.0 | 817.4 | 32.4 | 58.96 | 55.53 | 64.0 | 61.8 | 4 |
| remote-2hop | 1125.36 | 1133.96 | 215.4 | 183.4 | 293.0 | 200.4 | 800.6 | 44.0 | 60.78 | 55.42 | 64.0 | 61.0 | 4 |
| remote-3hop | 1524.05 | 1517.50 | 294.8 | 251.2 | 325.8 | 288.2 | 1135.2 | 155.0 | 61.55 | 54.13 | 64.0 | 62.6 | 4 |

## Remote End-to-End Perf Matrix

| scenario | avg total before | avg total after |
| --- | ---: | ---: |
| remote-1hop | 1211.49 | 1151.68 |
| remote-2hop | 1080.18 | 1183.38 |
| remote-3hop | 1501.64 | 1609.04 |

## Interpretation

- The strongest confirmed effect is on send discipline:
  - old path: no pacing, window-fill burst, `max inflight ~= 64`
  - new path: paced sender, `max burst = 4`, lower ACK tail, much lower window stall
- The strongest stable remote gain is on transport cleanliness:
  - `window stall` drops by `96.0%`, `94.5%`, and `86.3%`
  - `ack p95` drops by `38.0%`, `31.6%`, and `11.5%`
- `total_time_ms` is not uniformly improved on every route:
  - `1-hop`: `+14.12 ms`
  - `2-hop`: `+8.61 ms`
  - `3-hop`: `-6.55 ms`

That is the current boundary of this step: pacing removed the burst/stall signature, but fixed-window transport without congestion response still limits the final tail.
