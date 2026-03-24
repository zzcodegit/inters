# Congestion Response Compare: Before vs After

## Compared Runs

- before: [docs/artifacts/retransmit_backoff_remote_perf_stage_2026-03-23.md](/C:/neinternet/vpnnode/docs/artifacts/retransmit_backoff_remote_perf_stage_2026-03-23.md)
- after: [docs/artifacts/congestion_response_remote_perf_stage_2026-03-24.md](/C:/neinternet/vpnnode/docs/artifacts/congestion_response_remote_perf_stage_2026-03-24.md)

## Stage Metrics

| route | total ms before | total ms after | ack avg before | ack avg after | ack p95 before | ack p95 after | retransmit rate before | retransmit rate after | blocked by cap before | blocked by cap after | congestion events after | congestion duration ms after |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| 1-hop | 1107.18 | 1110.70 | 181.00 | 182.40 | 184.40 | 186.80 | 0.0000 | 0.0000 | 0 | 0 | 0 | 0.00 |
| 2-hop | 1167.11 | 1113.97 | 179.40 | 181.60 | 189.60 | 186.60 | 0.0441 | 0.0000 | 0 | 0 | 0 | 0.00 |
| 3-hop | 1470.85 | 1395.58 | 230.80 | 234.60 | 248.40 | 243.80 | 0.0401 | 0.0000 | 0 | 0 | 0 | 0.00 |
| 5-hop | 1897.28 | 2048.52 | 271.20 | 324.80 | 293.80 | 349.60 | 0.2810 | 0.0000 | 0 | 5 | 2 | 180.80 |

## End-to-End Perf Metrics

| route | total ms before | total ms after |
| --- | ---: | ---: |
| 1-hop | 1143.17 | 1161.23 |
| 2-hop | 1097.70 | 1306.79 |
| 3-hop | 1386.50 | 1595.01 |
| 5-hop | 1780.34 | 2109.14 |

## Interpretation

This step proves detection and behavioral coupling, not universal WAN speedup:

- `1-hop/2-hop/3-hop` stay out of congestion state entirely
- `5-hop` is the only route that enters congestion state
- `5-hop` shows `blocked_by_cap = 5` and `congestion_events = 2`
- the control layer is therefore selective, not globally throttling every path

The live WAN sample also shows that congestion detection alone is not yet a latency win by itself on the longest route. The remaining bottleneck after this step is the quality of the actual congestion response, not the existence of the signal.
