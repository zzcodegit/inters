# Chaos Replay-Window Compare

## Replay Rejects

| profile | before client old | before exit old | after client old | after exit old |
| --- | ---: | ---: | ---: | ---: |
| mild-reorder | 5 | 6 | 0 | 0 |
| combined | 0 | 1 | 0 | 0 |

## Duplicate Rejects

| profile | before client dup | before exit dup | after client dup | after exit dup |
| --- | ---: | ---: | ---: | ---: |
| mild-reorder | 3 | 3 | 0 | 0 |
| combined | 4 | 5 | 3 | 4 |

The remaining duplicate rejects in `combined` are consistent with the profile still injecting real duplicated packets. The replay-window fix was aimed at bounded reorder bursts being misclassified as stale, not at suppressing legitimate duplicate detection.

## Exact 3-Hop Timing

| profile | before TTFB ms | after TTFB ms | before total ms | after total ms |
| --- | ---: | ---: | ---: | ---: |
| mild-reorder | 179.91 | 173.68 | 180.64 | 174.33 |
| combined | 402.03 | 606.19 | 405.86 | 611.91 |

The mixed profile stayed green, but its latency got worse in this particular sample. That is why the next remaining limiter is documented as delay-driven response tail, not replay-window rejection.

## Sources

Before:

- [docs/artifacts/chaos_matrix_2026-03-22_lifecycle_fix.summary.json](/C:/neinternet/vpnnode/docs/artifacts/chaos_matrix_2026-03-22_lifecycle_fix.summary.json)
- [docs/artifacts/chaos_matrix_2026-03-22_lifecycle_fix_mild-reorder.stage.jsonl](/C:/neinternet/vpnnode/docs/artifacts/chaos_matrix_2026-03-22_lifecycle_fix_mild-reorder.stage.jsonl)
- [docs/artifacts/chaos_matrix_2026-03-22_lifecycle_fix_combined.stage.jsonl](/C:/neinternet/vpnnode/docs/artifacts/chaos_matrix_2026-03-22_lifecycle_fix_combined.stage.jsonl)

After:

- [docs/artifacts/chaos_matrix_2026-03-22_replay_fix_clean.summary.json](/C:/neinternet/vpnnode/docs/artifacts/chaos_matrix_2026-03-22_replay_fix_clean.summary.json)
- [docs/artifacts/chaos_matrix_2026-03-22_replay_fix_clean_mild-reorder.stage.jsonl](/C:/neinternet/vpnnode/docs/artifacts/chaos_matrix_2026-03-22_replay_fix_clean_mild-reorder.stage.jsonl)
- [docs/artifacts/chaos_matrix_2026-03-22_replay_fix_clean_combined.stage.jsonl](/C:/neinternet/vpnnode/docs/artifacts/chaos_matrix_2026-03-22_replay_fix_clean_combined.stage.jsonl)
- [docs/artifacts/chaos_replay_window_targeted_regression_2026-03-22.log](/C:/neinternet/vpnnode/docs/artifacts/chaos_replay_window_targeted_regression_2026-03-22.log)
