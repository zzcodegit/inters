# Retransmit Backoff Compare

## Scope

Before:

- [stage5_shortpath_remote_perf_matrix_2026-03-23.md](/C:/neinternet/vpnnode/docs/artifacts/stage5_shortpath_remote_perf_matrix_2026-03-23.md)
- [stage5_shortpath_remote_perf_stage_2026-03-23.md](/C:/neinternet/vpnnode/docs/artifacts/stage5_shortpath_remote_perf_stage_2026-03-23.md)

After:

- [retransmit_backoff_remote_perf_matrix_2026-03-23.md](/C:/neinternet/vpnnode/docs/artifacts/retransmit_backoff_remote_perf_matrix_2026-03-23.md)
- [retransmit_backoff_remote_perf_stage_2026-03-23.md](/C:/neinternet/vpnnode/docs/artifacts/retransmit_backoff_remote_perf_stage_2026-03-23.md)

`5-hop` is a new topology in this step, so it has no earlier direct baseline in the accepted artifact lineage.

## 1-hop / 2-hop / 3-hop

| route | total ms before | total ms after | delta | ACK p95 before | ACK p95 after | retransmit rate before | retransmit rate after | coarse timeout before | RTT-aware timeout after |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `1-hop` | `1122.00` | `1143.17` | `+21.17 ms` (`+1.89%`) | `191.0` | `184.4` | `0.0366` | `0.0000` | `350ms` | `-` |
| `2-hop` | `1196.78` | `1097.70` | `-99.08 ms` (`-8.28%`) | `193.6` | `189.6` | `0.0000` | `0.0441` | `350ms` | `273ms` |
| `3-hop` | `1445.42` | `1386.50` | `-58.92 ms` (`-4.08%`) | `246.6` | `248.4` | `0.0000` | `0.0401` | `350ms` | `369ms` |

## New 5-hop Sample

| route | total ms after | ACK avg | ACK p95 | retransmit rate | timeout avg | timeout p95 | trigger count | timeout / RTT |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `5-hop` | `1780.34` | `271.2` | `293.8` | `0.2810` | `231` | `242` | `81` avg, `397` worst run | `1.63` |

## What Changed

- The previous policy used one coarse stream-level timeout with a floor of `350ms`.
- The new policy uses per-frame RTT-aware timeout with factors `1.5 / 2.0 / 2.5+`.
- `1-hop` no longer shows inflated retransmit pressure in the fresh sample.
- `2-hop` and `3-hop` now expose retransmit timeout values that track current ACK cadence instead of sitting on the same coarse `350ms` floor.

## Honest Notes

- `1-hop` total time is slightly above the previous short-path sample even though ACK p95 and retransmit pressure are cleaner. This is live WAN variance, not effective-cap throttling.
- `2-hop` and `3-hop` keep the important direction for this step: retransmit timing is now explicitly coupled to RTT, and end-to-end total is lower than in the previous accepted short-path run.
- `5-hop` is the new stress path for this step. Its traces show the policy adapting under repeated loss rather than reusing the same timeout as `1-hop` and `2-hop`.
